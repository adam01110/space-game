# List available recipes.
default:
    @just --list

# Development features shared by workspace commands.
dev-features := "project-client/dev,project-server/dev"

# Run a native client.
client:
    cargo run -p project-client --features dev

# Run the headless server with development features.
server:
    cargo run -p project-server --features dev

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

# Build both binaries with release optimizations.
release:
    cargo build --workspace --release

# Build and optimize the WebAssembly client for size.
wasm:
    cargo build -p project-client --target wasm32-unknown-unknown --profile wasm-release
    wasm-opt -Os --output target/wasm32-unknown-unknown/wasm-release/project_client.opt.wasm target/wasm32-unknown-unknown/wasm-release/project_client.wasm
    mv target/wasm32-unknown-unknown/wasm-release/project_client.opt.wasm target/wasm32-unknown-unknown/wasm-release/project_client.wasm
