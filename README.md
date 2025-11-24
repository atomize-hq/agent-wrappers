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
- Event schema (Codex CLI 0.61.0): JSONL lines carry `type` plus an item or usage payload. Expect `thread.started` (with `thread_id`), `turn.started`, `item.completed` (with `item.id/type/text`), and `turn.completed` (with `usage` token counts); the CLI no longer emits per-event thread/turn IDs or `item.created/updated` variants. Errors surface as `{"type":"error","message":...}`. Examples ship `--sample` payloads so you can inspect shapes without a binary.
- Sample streaming/resume/apply payloads live under `crates/codex/examples/fixtures/*` (captured from the live CLI) and power the `--sample` flags in examples; refresh them whenever the CLI JSON surface changes so docs stay aligned.
- Enable JSONL streaming with `.json(true)` or by invoking the CLI directly. The crate returns captured output; use the examples to consume the stream yourself:
  - `crates/codex/examples/stream_events.rs`: typed consumer for the compact `thread.started`/`turn.started`/`item.completed`/`turn.completed` events (includes idle-timeout handling) plus a `--sample` replay path.
  - `crates/codex/examples/stream_last_message.rs`: runs `--output-last-message` + `--output-schema`, reads the emitted files, and ships sample payloads if the binary is missing.
  - `crates/codex/examples/stream_with_log.rs`: mirrors JSON events to stdout and tees them to `CODEX_LOG_PATH` (default `codex-stream.log`); also supports `--sample` and can defer to the binary's built-in log tee feature when advertised via `codex features list`.
  - `crates/codex/examples/json_stream.rs`: simplest `--json` usage when you just want the raw stream buffered.
- Artifacts: Codex can persist the final assistant message and the output schema alongside streaming output; point them at writable locations per the `stream_last_message` example. Apply/diff flows also surface stdout/stderr/exit (see below) so you can log or mirror them alongside the JSON stream.

## Resume, Diff, and Apply
- Resume via `codex exec --json resume --last` (or pass a conversation ID); the CLI emits `thread.started` again plus `turn/item` events without per-event IDs. Reuse the streaming consumer above to read the feed.
- The CLI currently omits a `diff` subcommand and `apply` expects a task ID (no JSON payloads). Use fixtures for doc/demo purposes and gate live apply flows behind explicit task IDs.
- Approvals and cancellations surface as events in MCP/app-server flows; see the server examples for approval-required hooks around apply.
- Example `crates/codex/examples/resume_apply.rs` streams resume events from the live CLI and falls back to fixture diff/apply payloads unless `--apply-task <id>` is provided.

## MCP + App-Server Flows
- The CLI ships stdio servers for Model Context Protocol and the app-server APIs. Examples cover the JSON-RPC wiring, approvals, and shutdown:
  - `crates/codex/examples/mcp_codex_tool.rs`: start `codex mcp-server --stdio`, call `tools/codex` with prompt/cwd/model/sandbox, and watch `approval_required`/`task_complete` notifications (includes `turn_id`/`sandbox` and supports `--sample`).
  - `crates/codex/examples/mcp_codex_reply.rs`: resume a session via `tools/codex-reply`, taking `CODEX_CONVERSATION_ID` or a CLI arg; supports `--sample`.
  - `crates/codex/examples/app_server_thread_turn.rs`: launch `codex app-server --stdio`, send `thread/start` then `turn/start`, and stream task notifications (thread/turn IDs echoed; `--sample` supported).
- Pass `CODEX_HOME` for isolated server state and `CODEX_BINARY` (or `.binary(...)`) to pin the binary version used by the servers.

## Feature Detection & Version Hooks
- `crates/codex/examples/feature_detection.rs` shows how to:
  - parse `codex --version`
  - list features via `codex features list` (if supported) and cache them per binary path so repeated probes avoid extra processes
  - gate optional knobs like JSON streaming, log tee, MCP/app-server endpoints, resume/apply/diff flags, and artifact flags
  - emit an upgrade advisory hook when required capabilities are missing
- Use this when deciding whether to enable `--json`, log tee paths, resume/apply helpers, or app-server endpoints in your app UI. Always gate new feature names against `codex features list` so drift in the binary's output is handled gracefully.

## Upgrade Advisories & Gaps
- Streaming/resume fixtures are captured from Codex CLI 0.61.0 (compact events, no per-event IDs); refresh them whenever the CLI JSON surface shifts.
- The CLI currently lacks a `diff` subcommand and JSON apply output; resume emits `thread.started` again instead of `thread.resumed`, so examples fall back to fixtures unless you opt into a live `codex apply <task-id>`.
- The crate still buffers stdout/stderr from streaming/apply flows instead of exposing a typed stream API; use the examples to consume JSONL incrementally until a typed interface lands.
- Capability detection caches are keyed to a binary path/version pairing; refresh them whenever the Codex binary path, mtime, or `--version` output changes instead of reusing stale results across upgrades. Treat `codex features list` output as best-effort hints that may drift across releases and fall back to the fixtures above when probing fails.

## Examples Index
- The full wrapper vs. native CLI matrix lives in `crates/codex/EXAMPLES.md`.
- Run any example via `cargo run -p codex --example <name> -- <args>`; most support `--sample` so you can read shapes without a binary present.
