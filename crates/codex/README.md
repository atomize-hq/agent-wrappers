# Codex Rust Wrapper

Async wrapper around the Codex CLI focused on the headless `codex exec` flow. The client shells out to the bundled or system Codex binary, mirrors stdout/stderr when asked, and keeps the parent process environment untouched.

## Binary and `CODEX_HOME` isolation

- Point the wrapper at a bundled Codex binary via [`CodexClientBuilder::binary`]; if unset, it honors `CODEX_BINARY` or falls back to `codex` on `PATH`.
- Apply an app-scoped home with [`CodexClientBuilder::codex_home`]. The resolved binary is mirrored into `CODEX_BINARY`, and the provided home is exported as `CODEX_HOME` for every spawn site (exec/login/status/logout). The parent environment is never mutated.
- Use [`CodexClientBuilder::create_home_dirs`] to control whether `CODEX_HOME`, `conversations/`, and `logs/` are created up front (defaults to `true` when a home is set). `RUST_LOG` defaults to `error` if you have not set it.

```rust
use codex::{CodexClient, CodexHomeLayout};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let binary = "/opt/myapp/bin/codex";
    let codex_home = "/var/lib/myapp/codex";

    // Discover (and optionally create) the CODEX_HOME layout.
    let layout = CodexHomeLayout::new(codex_home);
    layout.materialize(true)?;
    println!("Logs live at {}", layout.logs_dir().display());

    let client = CodexClient::builder()
        .binary(binary)
        .codex_home(codex_home)
        .create_home_dirs(true)
        .mirror_stdout(false)
        .quiet(true)
        .build();

    let reply = client.send_prompt("Health check").await?;
    println!("{reply}");
    Ok(())
}
```

## `CODEX_HOME` layout helper

`CodexHomeLayout` documents where Codex stores state under an app-scoped home:

- `config.toml`
- `auth.json`
- `.credentials.json`
- `history.jsonl`
- `conversations/` for transcript JSONL files
- `logs/` for `codex-*.log` files

Call [`CodexHomeLayout::materialize`] to create the root, `conversations/`, and `logs/` directories before spawning Codex.

## Stream JSONL events

Use the streaming surface to consume `codex exec --json` output as it arrives. Disable stdout mirroring so you control the console, and set an idle timeout to fail fast if the CLI stalls.

```rust
use codex::{CodexClient, ExecStreamRequest, ThreadEvent};
use futures_util::StreamExt;
use std::{path::PathBuf, time::Duration};

# async fn demo() -> Result<(), Box<dyn std::error::Error>> {
let client = CodexClient::builder()
    .json(true)
    .quiet(true)
    .mirror_stdout(false)
    .json_event_log("logs/codex_events.log")
    .build();

let mut stream = client
    .stream_exec(ExecStreamRequest {
        prompt: "List repo files".into(),
        idle_timeout: Some(Duration::from_secs(30)),
        output_last_message: Some(PathBuf::from("last_message.txt")),
        output_schema: None,
        json_event_log: None, // override per request if desired
    })
    .await?;

while let Some(event) = stream.events.next().await {
    match event {
        Ok(ThreadEvent::ItemDelta(delta)) => println!("delta: {:?}", delta.delta),
        Ok(other) => println!("event: {other:?}"),
        Err(err) => {
            eprintln!("stream error: {err}");
            break;
        }
    }
}

let completion = stream.completion.await?;
println!("codex exited with {}", completion.status);
if let Some(path) = completion.last_message_path {
    println!("last message saved to {}", path.display());
}
# Ok(()) }
```

## Log the raw JSON stream

Set `json_event_log` on the builder or per request to tee every raw JSONL line to disk before parsing:

- The log is appended to (existing files are preserved) and flushed per line.
- Parent directories are created automatically.
- An empty string is ignored; set a real path or leave `None` to disable.
- The per-request `json_event_log` overrides the builder default for that run.

Events still flow to your `events` stream even when teeing is enabled.

## Apply or inspect diffs

`CodexClient::apply` and `CodexClient::diff` wrap `codex apply/diff`, capture stdout/stderr, and return the exit status via [`ApplyDiffArtifacts`](crates/codex/src/lib.rs). They honor the builder flags you already use for streaming:

- `mirror_stdout` controls whether stdout is echoed while still being captured.
- `quiet` suppresses stderr mirroring (stderr is always returned in the artifacts).
- `RUST_LOG` defaults to `error` for these subcommands when the environment is unset; set `RUST_LOG=info` (or higher) to inspect codex internals.

```rust
use codex::CodexClient;

# async fn demo() -> Result<(), Box<dyn std::error::Error>> {
let client = CodexClient::builder()
    .mirror_stdout(false) // silence stdout while capturing
    .quiet(true)          // silence stderr while capturing
    .build();

let apply = client.apply().await?; // or client.diff()
println!("exit: {}", apply.status);
println!("stdout: {}", apply.stdout);
println!("stderr: {}", apply.stderr);
# Ok(()) }
```

When you stream JSONL events, apply/diff output is also emitted inside `file_change` events and tee'd to any `json_event_log` path you configure.

## RUST_LOG defaults

If `RUST_LOG` is unset, the wrapper injects `RUST_LOG=error` for spawned commands to silence verbose upstream tracing. Any existing `RUST_LOG` value is respected.

## MCP + app-server helpers

- `codex::mcp` offers typed clients for `codex mcp-server --stdio` and `codex app-server --stdio`, along with config managers for `[mcp_servers]` and `[app_runtimes]` plus launcher helpers when you want to spawn from saved config.
- Use `CodexClient::spawn_mcp_login_process` (capability-guarded) when you need an interactive bearer token for HTTP transports before persisting it via `McpConfigManager::login`.
- Examples: `mcp_codex_flow` (typed `codex/codex` + `codex/codex-reply` with optional cancellation), `mcp_codex_tool`/`mcp_codex_reply` (raw tool calls with `--sample` payloads), and `app_server_turns`/`app_server_thread_turn` (thread start/resume + optional interrupt). Pair these with `feature_detection` if the binary may be missing server endpoints.

## Examples

See `crates/codex/EXAMPLES.md` for one-to-one CLI parity examples, including `bundled_binary_home` to run Codex from an embedded binary with isolated state.
