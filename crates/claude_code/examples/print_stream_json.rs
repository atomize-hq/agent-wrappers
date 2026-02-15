//! Demonstrates `claude --print --output-format stream-json` parsing.
//!
//! Usage:
//! - `CLAUDE_EXAMPLE_LIVE=1 cargo run -p claude_code --example print_stream_json -- "Hello"`
//! - Optional isolation: `CLAUDE_EXAMPLE_ISOLATED_HOME=1`

use std::{env, error::Error};

use claude_code::{parse_stream_json_lines, ClaudeOutputFormat, ClaudePrintRequest};

#[path = "support/real_cli.rs"]
mod real_cli;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    if !real_cli::live_enabled() {
        real_cli::require_live("print_stream_json")?;
        return Ok(());
    }

    let prompt = collect_prompt()?;
    let client = real_cli::maybe_isolated_client("print_stream_json")?;
    let req = ClaudePrintRequest::new(prompt)
        .output_format(ClaudeOutputFormat::StreamJson)
        .debug(true)
        .session_id("00000000-0000-0000-0000-000000000000");
    let res = client.print(req).await?;

    println!("exit: {}", res.output.status);
    let raw = String::from_utf8_lossy(&res.output.stdout);
    let lines = parse_stream_json_lines(&raw);
    println!("parsed stream-json lines: {}", lines.len());
    for (idx, line) in lines.iter().take(10).enumerate() {
        println!("{idx}: {line:?}");
    }

    Ok(())
}

fn collect_prompt() -> Result<String, Box<dyn Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        return Err("Provide a prompt string".into());
    }
    Ok(args.join(" "))
}
