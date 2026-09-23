#!/usr/bin/env bash
# Build and package the WebAssembly client into web/dist.
#
# Extra arguments are forwarded to cargo, so callers select the feature set:
#   scripts/build-wasm.sh                        production client
#   scripts/build-wasm.sh --features browser-dev loopback HTTP client
set -euo pipefail

cargo build -p space-game-client --target wasm32-unknown-unknown --profile wasm-release "$@"
rm -rf web/dist
wasm-bindgen --out-dir web/dist --out-name space_game_client --target web --no-typescript target/wasm32-unknown-unknown/wasm-release/space-game-client.wasm
wasm-opt -Os --output web/dist/space_game_client_bg.opt.wasm web/dist/space_game_client_bg.wasm
mv web/dist/space_game_client_bg.opt.wasm web/dist/space_game_client_bg.wasm
cp web/index.html web/dist/index.html
cp web/menu.css web/menu.js web/dist/
cp -r assets web/dist/assets
