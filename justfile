# List available recipes.
default:
    @just --list

# Development features shared by workspace commands.
dev-features := "project-client/dev,project-server/dev"

# Run a local native client; it retries until the development server is available.
client:
    cargo run -p project-client --features dev

# Run the local development server.
server:
    #!/usr/bin/env bash
    set -euo pipefail
    umask 077
    state="${XDG_STATE_HOME:-$HOME/.local/state}/project-1/dev"
    mkdir -p "$state"
    chmod 700 "$state"
    exec 9>"$state/server.lock"
    if ! flock -n 9; then
        echo 'A local development server is already running.' >&2
        exit 1
    fi
    cargo build -p project-server --features dev
    export PROJECT_NETCODE_KEY_FILE="$state/netcode.key"
    if [[ ! -e "$PROJECT_NETCODE_KEY_FILE" ]]; then
        cargo run --quiet -p project-server --features dev -- generate-key "$PROJECT_NETCODE_KEY_FILE"
    fi
    exec cargo run -p project-server --features dev

# Type-check every crate and target with development features.
check:
    cargo check --workspace --all-targets --features "{{dev-features}}"

# Format the workspace.
fmt:
    cargo fmt --all

# Run Clippy with development features and reject warnings.
lint:
    cargo clippy --workspace --all-targets --features "{{dev-features}}" -- -D warnings

# Run the workspace test suite with development features.
test:
    cargo test --workspace --features "{{dev-features}}"

# Build the native binaries and optimized WebAssembly client for release.
release: wasm
    cargo build --workspace --release

# Build and optimize the WebAssembly client for size.
wasm:
    cargo build -p project-client --target wasm32-unknown-unknown --profile wasm-release
    wasm-opt -Os --output target/wasm32-unknown-unknown/wasm-release/project_client.opt.wasm target/wasm32-unknown-unknown/wasm-release/project_client.wasm
    mv target/wasm32-unknown-unknown/wasm-release/project_client.opt.wasm target/wasm32-unknown-unknown/wasm-release/project_client.wasm
