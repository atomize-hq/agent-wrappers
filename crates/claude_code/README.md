# Claude Code Rust Wrapper

Async wrapper around the Claude Code CLI (`claude`) focused on the headless `--print` flow.

Design goals:
- Non-interactive first: all supported prompting APIs run with `--print`.
- No automatic downloads: this crate never installs Claude Code and never auto-updates it; update only runs when explicitly invoked.
- Parent environment is never mutated; env overrides apply per-spawn only.

## Quickstart

```rust,no_run
use claude_code::{ClaudeClient, ClaudeOutputFormat, ClaudePrintRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ClaudeClient::builder().build();
    let req = ClaudePrintRequest::new("Hello from Rust")
        .output_format(ClaudeOutputFormat::Text);
    let res = client.print(req).await?;
    println!("{}", String::from_utf8_lossy(&res.output.stdout));
    Ok(())
}
```
