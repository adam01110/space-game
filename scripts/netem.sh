#!/usr/bin/env bash
# Impair loopback game traffic in both directions, or remove the impairment.
#
#   scripts/netem.sh apply <round-trip latency> <loss>
#   scripts/netem.sh reset
#
# The latency argument is a round-trip figure, matching an in-game ping
# display; it is halved and applied one-way in each direction.
set -euo pipefail

usage() {
	echo 'usage: netem.sh apply <latency> [loss] | netem.sh reset' >&2
	exit 2
}

mode=${1:-}
case "$mode" in
reset)
	(($# == 1)) || usage
	if sudo tc qdisc del dev lo root 2>/dev/null; then
		echo "Game network emulation disabled"
	else
		echo "Game network emulation was not enabled"
	fi
	;;
apply)
	(($# <= 3)) || usage
	latency=${2:-100ms}
	loss=${3:-0%}
	one_way=$(awk -v value="$latency" '
    BEGIN {
        if (value !~ /^[0-9]+(\.[0-9]+)?(us|ms|s)?$/) {
            print "latency must be a duration such as 100ms, 0.5s, or 25000us" > "/dev/stderr"
            exit 1
        }
        number = value
        scale = 1000
        if (value ~ /us$/) { sub(/us$/, "", number); scale = 1 }
        else if (value ~ /ms$/) { sub(/ms$/, "", number) }
        else if (value ~ /s$/) { sub(/s$/, "", number); scale = 1000000 }
        printf "%.0fus", number / 2 * scale
    }')
	trap 'sudo tc qdisc del dev lo root 2>/dev/null || true' ERR
	sudo tc qdisc replace dev lo root handle 1: prio bands 3
	sudo tc qdisc add dev lo parent 1:2 handle 20: netem delay "$one_way" loss "$loss"
	sudo tc qdisc add dev lo parent 1:3 handle 30: netem delay "$one_way" loss "$loss"
	sudo tc filter add dev lo protocol ip parent 1: prio 3 u32 match ip protocol 17 0xff match ip dport 5000 0xffff flowid 1:2
	sudo tc filter add dev lo protocol ip parent 1: prio 3 u32 match ip protocol 17 0xff match ip sport 5000 0xffff flowid 1:3
	trap - ERR
	echo "Game network emulation enabled: round-trip latency=$latency ($one_way each way), loss=$loss per direction"
	;;
*) usage ;;
esac
