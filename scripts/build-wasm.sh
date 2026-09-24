#!/usr/bin/env bash
# Build and package the WebAssembly client into web/dist.
#
# Also transpiles web/menu.ts and refreshes the wasm-bindgen declarations that
# web/menu.ts type-checks against.
#
# Extra arguments are forwarded to cargo, so callers select the feature set:
#   scripts/build-wasm.sh                        production client
#   scripts/build-wasm.sh --features browser-dev loopback HTTP client
set -euo pipefail

cargo build -p space-game-client --target wasm32-unknown-unknown --profile wasm-release "$@"
rm -rf web/dist
wasm-bindgen --out-dir web/dist --out-name space_game_client --target web target/wasm32-unknown-unknown/wasm-release/space-game-client.wasm
wasm-opt -Os --output web/dist/space_game_client_bg.opt.wasm web/dist/space_game_client_bg.wasm
mv web/dist/space_game_client_bg.opt.wasm web/dist/space_game_client_bg.wasm
bun build web/menu.ts --outfile web/dist/menu.js --target browser --no-bundle
cp web/dist/space_game_client.d.ts web/space_game_client.d.ts
cp web/index.html web/dist/index.html
cp web/menu.css web/dist/
cp -r assets web/dist/assets
