# Codex Rust Wrapper

Async helper around the `codex` CLI (`codex exec` in particular). It shells out to the installed binary, applies safe defaults (color handling, timeouts, `RUST_LOG`), and surfaces stdout/stderr or typed JSONL events.

## Single prompt

```rust
use codex::CodexClient;

# async fn demo() -> Result<(), Box<dyn std::error::Error>> {
let client = CodexClient::builder().build();
let reply = client.send_prompt("Summarize src/lib.rs").await?;
println!("codex replied: {reply}");
# Ok(()) }
```

## Stream JSONL events

Use the streaming surface to consume `codex exec --json` output as it arrives. Disable stdout mirroring so you can own the console, and set an idle timeout to fail fast on hung sessions.

```rust
use codex::{CodexClient, ExecStreamRequest, ThreadEvent};
use futures_util::StreamExt;
use std::{path::PathBuf, time::Duration};

# async fn demo() -> Result<(), Box<dyn std::error::Error>> {
let client = CodexClient::builder()
    .json(true)
    .quiet(true)
    .mirror_stdout(false)
    .build();

let mut stream = client
    .stream_exec(ExecStreamRequest {
        prompt: "List repo files".into(),
        idle_timeout: Some(Duration::from_secs(30)),
        output_last_message: Some(PathBuf::from("last_message.txt")),
        output_schema: None,
        json_event_log: None, // inherit builder default if set
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

`ExecStreamRequest` accepts optional `output_schema` (writes the JSON schema codex reports for the run) and `idle_timeout` (returns `ExecStreamError::IdleTimeout` if no events arrive in time). When `output_last_message` is `None`, a temporary path is generated and returned in `ExecCompletion::last_message_path`.

## Log the raw JSON stream

Set `json_event_log` on the builder or per request to tee every raw JSONL line to disk before parsing:

- The log is appended to (existing files are preserved) and flushed per line.
- Parent directories are created automatically.
- An empty string is ignored; set a real path or leave `None` to disable.
- The per-request `json_event_log` overrides the builder default for that run.

The events continue to flow to your `events` stream even when log teeing is enabled.

## RUST_LOG defaults

If `RUST_LOG` is unset, the wrapper injects `RUST_LOG=error` for the spawned `codex` process to silence verbose upstream tracing. Any existing `RUST_LOG` value is respected. To debug codex internals alongside your own logs, set `RUST_LOG=info` (or higher) before invoking `CodexClient`.

## Release notes

- New streaming docs cover `ExecStreamRequest` fields, idle timeouts, and the `events`/`completion` contract.
- Documented the JSON event log tee: append-only, flushed per line, request-level override of builder default.
- Clarified `RUST_LOG` defaults (`error` when unset) and how to opt into more verbose codex logs.
