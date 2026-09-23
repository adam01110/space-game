# List available recipes.
default:
    @just --list

# Development features shared by workspace commands.
dev-features := "space-game-client/dev,space-game-server/dev"

# Run a local native client; it retries until the development server is available.
client:
    cargo run -p space-game-client --features dev

# Add round-trip latency and packet loss to game traffic in both directions; run `just netem-reset` to restore networking.
netem latency="100ms" loss="0%":
    bash scripts/netem.sh apply "{{latency}}" "{{loss}}"

# Remove game traffic latency and packet loss.
netem-reset:
    bash scripts/netem.sh reset

# Run the local development server.
server:
    bash scripts/server.sh

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
    bash scripts/build-wasm.sh

# Build the browser client with loopback HTTP enabled.
web-build:
    bash scripts/build-wasm.sh --features browser-dev

# Build and serve the browser client, asking before it rebuilds the bundle; proxies /connect to `just server`.
web:
    bash scripts/web.sh
