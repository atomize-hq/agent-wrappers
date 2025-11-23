//! Demonstrates enabling Codex JSONL streaming mode (`--json`).
//! Usage:
//! ```powershell
//! cargo run -p codex --example json_stream -- "Summarize repo status"
//! ```

use codex::CodexClient;
use std::{env, error::Error};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let prompt = collect_prompt()?;
    let client = CodexClient::builder().json(true).build();
    let response = client.send_prompt(&prompt).await?;
    println!("{response}");
    Ok(())
}

fn collect_prompt() -> Result<String, Box<dyn Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        return Err("Provide a prompt".into());
    }
    Ok(args.join(" "))
}
