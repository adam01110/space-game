# List available recipes.
default:
    @just --list

# Development features shared by workspace commands.
dev-features := "space-game-client/dev,space-game-server/dev"

# Run a local native client; it retries until the development server is available.
client:
    cargo run -p space-game-client --features dev

# Add latency and packet loss to server-to-client game traffic; run `just netem-reset` to restore networking.
netem latency="100ms" loss="0%":
    #!/usr/bin/env bash
    set -euo pipefail
    trap 'sudo tc qdisc del dev lo root 2>/dev/null || true' ERR
    sudo tc qdisc replace dev lo root handle 1: prio
    sudo tc qdisc add dev lo parent 1:3 handle 30: netem delay "{{latency}}" loss "{{loss}}"
    sudo tc filter add dev lo protocol ip parent 1: prio 3 u32 match ip protocol 17 0xff match ip sport 5000 0xffff flowid 1:3
    trap - ERR
    echo "Server-to-client network emulation enabled: latency={{latency}}, loss={{loss}}"

# Remove server-to-client latency and packet loss.
netem-reset:
    @if sudo tc qdisc del dev lo root 2>/dev/null; then echo "Server-to-client network emulation disabled"; else echo "Server-to-client network emulation was not enabled"; fi

# Run the local development server.
server:
    #!/usr/bin/env bash
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

# Type-check every crate and target with development features.
check:
    cargo check --workspace --all-targets --features "{{dev-features}}"

# Format the workspace.
fmt:
    cargo fmt --all

# Run Clippy with development features and reject warnings.
lint:
    cargo clippy --workspace --all-targets --features "{{dev-features}}" -- -D warnings

# Run the workspace test suite with Nextest and development features.
test:
    cargo nextest run --workspace --features "{{dev-features}}"

# Build the native binaries and optimized WebAssembly client for release.
release: wasm native-release

# Build the native client and server binaries for release.
native-release:
    cargo build --release -p space-game-client -p space-game-server

# Build and package the production WebAssembly client for size.
wasm:
    cargo build -p space-game-client --target wasm32-unknown-unknown --profile wasm-release
    rm -rf web/dist
    wasm-bindgen --out-dir web/dist --out-name space_game_client --target web --no-typescript target/wasm32-unknown-unknown/wasm-release/space-game-client.wasm
    wasm-opt -Os --output web/dist/space_game_client_bg.opt.wasm web/dist/space_game_client_bg.wasm
    mv web/dist/space_game_client_bg.opt.wasm web/dist/space_game_client_bg.wasm
    cp web/index.html web/dist/index.html
    cp -r assets web/dist/assets

# Build the browser client with loopback HTTP enabled.
web-build:
    cargo build -p space-game-client --target wasm32-unknown-unknown --profile wasm-release --features browser-dev
    rm -rf web/dist
    wasm-bindgen --out-dir web/dist --out-name space_game_client --target web --no-typescript target/wasm32-unknown-unknown/wasm-release/space-game-client.wasm
    wasm-opt -Os --output web/dist/space_game_client_bg.opt.wasm web/dist/space_game_client_bg.wasm
    mv web/dist/space_game_client_bg.opt.wasm web/dist/space_game_client_bg.wasm
    cp web/index.html web/dist/index.html
    cp -r assets web/dist/assets

# Build and serve the browser client, proxying /connect to `just server`.
web: web-build
    caddy run --config web/Caddyfile
