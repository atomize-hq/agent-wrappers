//! Demonstrates `claude doctor` via the wrapper (real CLI only).
//!
//! Usage:
//! - `cargo run -p claude_code --example doctor`
//! - Optional isolation: `CLAUDE_EXAMPLE_ISOLATED_HOME=1`

use std::error::Error;

#[path = "support/real_cli.rs"]
mod real_cli;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let client = real_cli::maybe_isolated_client("doctor")?;
    let out = client.doctor().await?;
    println!("exit: {}", out.status);
    print!("{}", String::from_utf8_lossy(&out.stdout));
    eprint!("{}", String::from_utf8_lossy(&out.stderr));
    Ok(())
}
