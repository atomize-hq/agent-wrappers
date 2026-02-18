$ErrorActionPreference = "Stop"

Write-Host "## Universal Agent API smoke (windows)"

rustc --version
cargo --version

cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features

Write-Host "OK"

