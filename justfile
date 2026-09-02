# List available recipes.
default:
    @just --list

# Run a development build with dynamic linking.
dev:
    cargo run --features dev

# Run without development-only features.
run:
    cargo run

# Run all Nix flake checks.
check:
    nix flake check

# Run the test suite.
test:
    cargo test

# Format the project with the Nix formatter.
fmt:
    nix fmt

# Run Clippy and reject warnings.
lint:
    cargo clippy --all-targets -- -D warnings

# Build an optimized release binary with LLVM.
release:
    cargo build --release

# Optimize WebAssembly binary for size.
wasm-opt input output:
    wasm-opt -Os --output "{{output}}" "{{input}}"
