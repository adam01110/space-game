#!/usr/bin/env bash
# Temporarily admit one LAN tester to the HTTPS browser site and game transport.
set -euo pipefail

usage() {
	echo 'usage: playtest-firewall.sh <tester IPv4 address>' >&2
	exit 2
}

(($# == 1)) || usage
peer=$1
if [[ ! $peer =~ ^[0-9]{1,3}(\.[0-9]{1,3}){3}$ ]]; then
	usage
fi
IFS=. read -r a b c d <<<"$peer"
for octet in "$a" "$b" "$c" "$d"; do
	if ((10#$octet > 255)); then
		usage
	fi
done

route=$(ip -4 route get "$peer")
if [[ $route == *' via '* || $route != *' dev '* ]]; then
	echo 'The tester must be on a directly connected IPv4 network (no router or VPN gateway).' >&2
	exit 1
fi
iface=${route#* dev }
iface=${iface%% *}
if [[ $iface == lo ]]; then
	echo 'Use a non-loopback LAN address for the tester.' >&2
	exit 1
fi

# Keep the privileged process alive so cleanup does not need a second sudo prompt.
if ((EUID != 0)); then
	exec sudo -- bash "$0" "$peer"
fi

marker="space-game-playtest-$$"
handles=()
cleanup() {
	for handle in "${handles[@]}"; do
		nft delete rule inet nixos-fw input-allow handle "$handle" || echo "Could not remove playtest firewall rule handle $handle; remove it manually." >&2
	done
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
trap 'exit 129' HUP

# NixOS routes new connections through input-allow. Do not use temp-ports:
# that set admits every source on every interface.
add_rule() {
	local protocol=$1 port=$2 output handle rule_marker="$marker-$1"
	output=$(nft --echo --handle insert rule inet nixos-fw input-allow iifname "$iface" ip saddr "$peer" "$protocol" dport "$port" accept comment "$rule_marker")
	if [[ $output =~ \#\ handle\ ([0-9]+) ]]; then
		handle=${BASH_REMATCH[1]}
	else
		# The rule was already inserted; locate it by its unique comment before exiting.
		handle=$(nft --handle list chain inet nixos-fw input-allow | awk -v marker="$rule_marker" 'index($0, marker) {print $NF}')
		if [[ ! $handle =~ ^[0-9]+$ ]]; then
			echo "Cannot locate inserted firewall rule $rule_marker; inspect nft list chain inet nixos-fw input-allow." >&2
			exit 1
		fi
	fi
	handles+=("$handle")
}
add_rule udp 5000
add_rule tcp 8000

echo "Allowing UDP 5000 and TCP 8000 from $peer on $iface while this command runs. Press Ctrl-C to close."
sleep infinity
