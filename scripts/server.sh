#!/usr/bin/env bash
# Run the local development server, refusing a second concurrent instance.
set -euo pipefail

umask 077
state="${XDG_STATE_HOME:-$HOME/.local/state}/space-game/dev"
mkdir -p "$state"
chmod 700 "$state"
exec 9>"$state/server.lock"
if ! flock -n 9; then
	echo 'A local development server is already running.' >&2
	exit 1
fi
cargo build -p space-game-server --features dev
export SPACE_GAME_NETCODE_KEY_FILE="$state/netcode.key"
if [[ ! -e "$SPACE_GAME_NETCODE_KEY_FILE" ]]; then
	cargo run --quiet -p space-game-server --features dev -- generate-key "$SPACE_GAME_NETCODE_KEY_FILE"
fi
exec cargo run -p space-game-server --features dev
