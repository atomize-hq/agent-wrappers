$ErrorActionPreference = "Stop"

Write-Host "## Claude Code live stream-json smoke (windows)"

rustc --version
cargo --version

cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features

Write-Host "OK"

