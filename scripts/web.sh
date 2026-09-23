#!/usr/bin/env bash
# Serve the browser client, optionally rebuilding the WebAssembly bundle first.
set -euo pipefail

if [[ -e web/dist/index.html ]]; then
	read -r -p 'Rebuild the WebAssembly client? [y/N] ' reply || reply=
else
	echo 'web/dist is missing, building the WebAssembly client'
	reply=y
fi

case "$reply" in
[yY] | [yY][eE][sS]) bash scripts/build-wasm.sh --features browser-dev ;;
*) echo 'Serving the existing web/dist' ;;
esac

exec caddy run --config web/Caddyfile
