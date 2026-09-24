#!/usr/bin/env bash
# Serve the browser client, optionally rebuilding the WebAssembly bundle first.
set -euo pipefail

case "${1:-}" in
'') config=web/Caddyfile ;;
--lan)
	(($# == 1)) || exit 2
	: "${SPACE_GAME_LAN_IP:?Set SPACE_GAME_LAN_IP to the LAN IPv4 address of this machine}"
	config=web/Caddyfile.lan
	;;
*)
	echo 'usage: web.sh [--lan]' >&2
	exit 2
	;;
esac

if [[ -e web/dist/index.html ]]; then
	read -r -p 'Rebuild the WebAssembly client? [y/N] ' reply || reply=
else
	echo 'web/dist is missing, building the WebAssembly client'
	reply=y
fi

case "$reply" in
[yY] | [yY][eE][sS])
	if [[ $config == web/Caddyfile.lan ]]; then
		bash scripts/build-wasm.sh
	else
		bash scripts/build-wasm.sh --features browser-dev
	fi
	;;
*) echo 'Serving the existing web/dist' ;;
esac

if [[ $config == web/Caddyfile.lan ]]; then
	data_dir=$(caddy environ | sed -n 's/^caddy.AppDataDir=//p' | head -n 1)
	echo "Share the Caddy CA certificate with testers: $data_dir/pki/authorities/local/root.crt"
	echo "After they trust it, open https://$SPACE_GAME_LAN_IP:8000/"
fi
exec caddy run --config "$config"
