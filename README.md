# Codex Rust Wrapper

Async helper around the OpenAI Codex CLI for programmatic prompting, streaming, and server flows. The crate shells out to `codex`, applies safe defaults (temp working dirs, timeouts, quiet stderr mirroring), and lets you pick either a packaged binary or an env-provided one.

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
- Default binary resolution: `CODEX_BINARY` if set, otherwise `codex` on `PATH`. Use `.binary(...)` to point at a bundled binary (see `crates/codex/examples/bundled_binary.rs`).

## Bundled Binary & `CODEX_HOME`
- Ship Codex with your app by setting `CODEX_BINARY` or calling `.binary("/opt/myapp/bin/codex")`. The `bundled_binary` example shows falling back to `CODEX_BUNDLED_PATH` and a local `bin/codex` hint.
- Isolate state with `CODEX_HOME` (config/auth/history/logs live under that directory: `config.toml`, `auth.json`, `.credentials.json`, `history.jsonl`, `conversations/*.jsonl`, `logs/codex-*.log`). The crate uses the current process env for every spawn.
- Quick isolated run (see `crates/codex/examples/codex_home.rs`):
  ```rust
  std::env::set_var("CODEX_HOME", "/tmp/my-app-codex");
  let client = CodexClient::builder().build();
  let _ = client.send_prompt("Health check").await?;
  ```

## Exec API & Safety Defaults
- `send_prompt` shells out to `codex exec --skip-git-repo-check` with:
  - temp working directory per call unless `working_dir` is set
  - 120s timeout (use `.timeout(Duration::ZERO)` to disable)
  - ANSI colors off by default (`ColorMode::Never`)
  - mirrors stdout by default; set `.mirror_stdout(false)` when parsing JSON
  - `RUST_LOG=error` if unset to keep the console quiet
  - model-specific reasoning config for `gpt-5`/`gpt-5-codex`
- Other builder flags: `.model("gpt-5-codex")`, `.image("/path/mock.png")`, `.json(true)` (pipes prompt via stdin), `.quiet(true)`.
- Example `crates/codex/examples/send_prompt.rs` covers the baseline; `working_dir(_json).rs`, `timeout*.rs`, `image_json.rs`, `color_always.rs`, `quiet.rs`, and `no_stdout_mirror.rs` expand on inputs and output handling.

## Streaming Output & Artifacts
- Enable JSONL streaming with `.json(true)` or by invoking the CLI directly. The crate returns captured output; use the examples to consume the stream yourself:
  - `crates/codex/examples/stream_events.rs`: typed consumer for `thread.started`, `turn.started/completed`, and `item.created` events; includes idle timeout handling and a `--sample` replay path.
  - `crates/codex/examples/stream_last_message.rs`: runs `--output-last-message` + `--output-schema`, reads the emitted files, and ships sample payloads if the binary is missing.
  - `crates/codex/examples/stream_with_log.rs`: mirrors JSON events to stdout and tees them to `CODEX_LOG_PATH` (default `codex-stream.log`); also supports `--sample`.
  - `crates/codex/examples/json_stream.rs`: simplest `--json` usage when you just want the raw stream buffered.
- Artifacts: Codex can persist the final assistant message and the output schema alongside streaming output; point them at writable locations per the `stream_last_message` example.

## MCP + App-Server Flows
- The CLI ships stdio servers for Model Context Protocol and the app-server APIs. Examples cover the JSON-RPC wiring, approvals, and shutdown:
  - `crates/codex/examples/mcp_codex_tool.rs`: start `codex mcp-server --stdio`, call `tools/codex` with prompt/cwd/model/sandbox, and watch `approval_required`/`task_complete` notifications (`--sample` available).
  - `crates/codex/examples/mcp_codex_reply.rs`: resume a session via `tools/codex-reply`, taking `CODEX_CONVERSATION_ID` or a CLI arg; supports `--sample`.
  - `crates/codex/examples/app_server_thread_turn.rs`: launch `codex app-server --stdio`, send `thread/start` then `turn/start`, and stream task notifications (`--sample` supported).
- Pass `CODEX_HOME` for isolated server state and `CODEX_BINARY` (or `.binary(...)`) to pin the binary version used by the servers.

## Feature Detection & Version Hooks
- `crates/codex/examples/feature_detection.rs` shows how to:
  - parse `codex --version`
  - list features via `codex features list` (if supported)
  - gate optional knobs like JSON streaming or log tee
  - emit an upgrade advisory hook when required capabilities are missing
- Use this when deciding whether to enable `--json`, log tee paths, or app-server endpoints in your app UI.

## Examples Index
- The full wrapper vs. native CLI matrix lives in `crates/codex/EXAMPLES.md`.
- Run any example via `cargo run -p codex --example <name> -- <args>`; most support `--sample` so you can read shapes without a binary present.
