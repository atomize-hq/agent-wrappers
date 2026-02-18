#!/usr/bin/env bash
set -euo pipefail

echo "## Universal Agent API smoke (macos)"
rustc --version
cargo --version

cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features

echo "OK"

