# Codex Rust Wrapper

Async helper around the OpenAI Codex CLI for programmatic prompting, streaming, apply/diff helpers, and stdio servers. The crate shells out to `codex`, applies safe defaults (temp working dirs, timeouts, quiet stderr mirroring), and supports bundled binaries plus capability-aware streaming and typed MCP/app-server helpers.

## Getting Started
- Add the dependency:
  ```toml
  [dependencies]
  codex = { path = "crates/codex" }
  ```
- Minimal prompt:
  ```rust
  use codex::CodexClient;

  # async fn demo() -> Result<(), Box<dyn std::error::Error>> {
  let client = CodexClient::builder().build();
  let reply = client.send_prompt("List rustfmt defaults").await?;
  println!("{reply}");
  # Ok(()) }
  ```

## Binary + `CODEX_HOME` isolation
- Point at a bundled binary with `CodexClientBuilder::binary`; when unset, the wrapper honors `CODEX_BINARY` or falls back to `codex` on `PATH`. `CODEX_BUNDLED_PATH` and a local `bin/codex` can be used as hints in the examples.
- Scope Codex data via `CodexClientBuilder::codex_home` and optionally create the layout with `CodexClientBuilder::create_home_dirs`; overrides are applied per spawn without mutating the parent environment. Use `CodexHomeLayout` to inspect `config.toml`, `auth.json`, `.credentials.json`, `history.jsonl`, `conversations/`, and `logs/` under an isolated home.
- Binary/home examples: see `crates/codex/examples/bundled_binary.rs`, `env_binary.rs`, and `codex_home.rs`. The `bundled_binary_home` example exercises both knobs together.

## Exec API and safety defaults
- Wrapper calls run `codex exec --skip-git-repo-check` with a temp working dir unless `working_dir` is set, a 120s timeout by default (`Duration::ZERO` disables), ANSI colors off by default (`ColorMode::Never`), and `RUST_LOG=error` when unset.
- Stdout mirrors by default; set `.mirror_stdout(false)` when parsing JSON. `.quiet(true)` suppresses stderr mirroring while still capturing it.
- Model pickers (`.model("gpt-5-codex")`), image attachment (`.image(...)`), and JSON mode (`.json(true)`) map directly to the CLI. Reasoning defaults for `gpt-5`/`gpt-5-codex` are applied when unset.
- `ExecStreamRequest` supports `idle_timeout` (fails fast on silent streams), `output_last_message`/`output_schema` (artifact paths), and `json_event_log` (tee raw JSONL before parsing).

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

Events include `thread.started`, `turn.started`/`turn.completed`/`turn.failed`, and `item.created`/`item.updated` with `item.type` such as `agent_message`, `reasoning`, `command_execution`, `file_change`, `mcp_tool_call`, `web_search`, or `todo_list` plus optional `status`/`content`/`input`. Errors surface as `{"type":"error","message":...}`. Sample payloads ship with the streaming examples for offline inspection.

## Log tee, artifacts, and RUST_LOG defaults
- Set `json_event_log` on the builder or per request to tee every raw JSONL line to disk before parsing; logs append to existing files, flush per line, and create parent directories.
- `ExecStreamRequest` accepts optional `output_schema` (writes the JSON schema Codex reports) and `idle_timeout` (returns `ExecStreamError::IdleTimeout` if no events arrive in time). When `output_last_message` is `None`, a temporary path is generated and returned in `ExecCompletion::last_message_path`.
- When `RUST_LOG` is unset, the wrapper injects `RUST_LOG=error` for spawned processes to silence verbose upstream tracing. Set `RUST_LOG=info` (or higher) to debug codex internals alongside your own logs.

## Apply or inspect diffs

`CodexClient::apply` and `CodexClient::diff` wrap `codex apply/diff`, capture stdout/stderr, and return the exit status via [`ApplyDiffArtifacts`](crates/codex/src/lib.rs). They honor the builder flags you already use for streaming:

- `mirror_stdout` controls whether stdout is echoed while still being captured.
- `quiet` suppresses stderr mirroring (stderr is always returned in the artifacts).
- When you stream JSONL events, apply/diff output is also emitted inside `file_change` events (stdout/stderr/exit code) and tee'd to any `json_event_log` path you configure.

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

## MCP + App-Server flows
- `crates/codex/src/mcp.rs`: typed helpers to launch `codex mcp-server`/`codex app-server` over stdio, manage `[mcp_servers]`/`[app_runtimes]` entries, and stream approvals/notifications.
- Use `CodexClient::spawn_mcp_login_process` (capability-guarded) to collect bearer tokens for HTTP transports before storing them with `McpConfigManager::login`.
- `crates/codex/examples/mcp_codex_flow.rs`: use `CodexMcpServer` to call `codex/codex`, stream `codex/event`, optionally cancel, and follow up with `codex/codex-reply` when a conversation ID is returned.
- `crates/codex/examples/mcp_codex_tool.rs`: start `codex mcp-server --stdio`, call `tools/codex` with prompt/cwd/model/sandbox, and watch `approval_required`/`task_complete` notifications (supports `--sample`).
- `crates/codex/examples/mcp_codex_reply.rs`: resume a session via `tools/codex-reply`, taking `CODEX_CONVERSATION_ID` or a CLI arg; supports `--sample`.
- `crates/codex/examples/app_server_turns.rs`: use `CodexAppServer` to start or resume threads, stream items/task_complete, and optionally send `turn/interrupt` after a delay.
- `crates/codex/examples/app_server_thread_turn.rs`: launch `codex app-server --stdio`, send `thread/start` then `turn/start`, and stream task notifications; supports `--sample` and `CODEX_HOME` isolation.

## Feature detection and upgrades
- `crates/codex/examples/feature_detection.rs` probes `codex --version` and `codex features list` to gate streaming/log tee/artifact flags plus MCP/app-server endpoints and emit upgrade advisories.
- `crates/codex/EXAMPLES.md` maps every example to the matching CLI invocation for parity checks. Most examples ship `--sample` payloads so you can read shapes without a binary present.

## Release notes
- Streaming docs cover `ExecStreamRequest` fields, idle timeouts, artifact paths, and the `events`/`completion` contract alongside JSON event log teeing.
- Binary and `CODEX_HOME` isolation guidance is consolidated with bundled-binary fallbacks and layout helpers.
- MCP/app-server coverage now includes `codex::mcp` helpers, feature detection gates, and expanded example pointers.
