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

## Examples (real CLI, no stubs)

Examples live under `crates/claude_code/examples/` and always spawn a real `claude` binary.
See `crates/claude_code/EXAMPLES.md` for a 1:1 mapping of wrapper examples to native CLI commands.

Common environment variables:
- `CLAUDE_BINARY`: path to the `claude` binary (otherwise uses repo-local `./claude-<target>` when present, or `claude` from PATH).
- `CLAUDE_EXAMPLE_ISOLATED_HOME=1`: run examples with an isolated home under `target/`.
- `CLAUDE_EXAMPLE_LIVE=1`: enable examples that may require network/auth (e.g. `print_*`, `setup_token_flow`).
- `CLAUDE_EXAMPLE_ALLOW_MUTATION=1`: enable examples that may mutate local state (e.g. `update`, plugin/MCP management).
