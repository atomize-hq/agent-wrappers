#!/usr/bin/env bash
set -euo pipefail

echo "## Claude Code live stream-json smoke (macos)"
rustc --version
cargo --version

cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features

echo "OK"

