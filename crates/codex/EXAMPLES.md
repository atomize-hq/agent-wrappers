# Codex Wrapper Examples vs. Native CLI

Every example under `crates/codex/examples/` maps to a `codex` CLI invocation. Wrapper calls (`cargo run -p codex --example ...`) run with safe defaults: `--skip-git-repo-check`, temp working dirs unless overridden, 120s timeout, ANSI color disabled, and `RUST_LOG=error` unless set. Select the binary with `CODEX_BINARY` or `.binary(...)`; set `CODEX_HOME` to keep config/auth/history/logs under an app-scoped directory. Examples labeled `--sample` print mocked data (covering `thread/turn/item` events and MCP/app-server notifications) when you do not have a binary handy.

## Basics

| Wrapper example | Native command | Notes |
| --- | --- | --- |
| `cargo run -p codex --example send_prompt -- "List Rust toolchain commands"` | `codex exec "List Rust toolchain commands" --skip-git-repo-check` | Baseline prompt with default timeout/temp dir. |
| `cargo run -p codex --example timeout -- "List long-running tasks"` | `codex exec "List long-running tasks" --skip-git-repo-check --timeout 30` | Forces a 30‑second timeout. |
| `cargo run -p codex --example timeout_zero -- "Stream until completion"` | `codex exec "Stream until completion" --skip-git-repo-check --timeout 0` | Disables the wrapper timeout. |
| `cargo run -p codex --example working_dir -- "C:\\path\\to\\repo" "List files here"` | `codex exec "List files here" --skip-git-repo-check --cd "C:\\path\\to\\repo"` | Run inside a specific directory. |
| `cargo run -p codex --example working_dir_json -- "C:\\path\\to\\repo" "Summarize repo status"` | `echo "Summarize repo status" \| codex exec --skip-git-repo-check --json --cd "C:\\path\\to\\repo"` | Combines working dir override with JSON streaming. |
| `cargo run -p codex --example select_model -- gpt-5-codex -- "Explain rustfmt defaults"` | `codex exec "Explain rustfmt defaults" --skip-git-repo-check --model gpt-5-codex` | Picks a specific model. |
| `cargo run -p codex --example color_always -- "Show colorful output"` | `codex exec "Show colorful output" --skip-git-repo-check --color always` | Forces ANSI color codes. |
| `cargo run -p codex --example image_json -- "C:\\path\\to\\mockup.png" "Describe the screenshot"` | `echo "Describe the screenshot" \| codex exec --skip-git-repo-check --json --image "C:\\path\\to\\mockup.png"` | Attach an image while streaming JSON quietly. |
| `cargo run -p codex --example quiet -- "Run without tool noise"` | `codex exec "Run without tool noise" --skip-git-repo-check --quiet` | Suppress stderr mirroring. |
| `cargo run -p codex --example no_stdout_mirror -- "Stream quietly"` | `codex exec "Stream quietly" --skip-git-repo-check > out.txt` | Disable stdout mirroring to capture output yourself. |

## Binary + CODEX_HOME

| Wrapper example | Native command | Notes |
| --- | --- | --- |
| `$env:CODEX_BINARY="C:\\bin\\codex-nightly.exe"; cargo run -p codex --example env_binary -- "Nightly sanity check"` | `C:\\bin\\codex-nightly.exe exec "Nightly sanity check" --skip-git-repo-check` | Honors `CODEX_BINARY` override. |
| `CODEX_BUNDLED_PATH=/opt/myapp/codex cargo run -p codex --example bundled_binary -- "Quick health check"` | `CODEX_BINARY=/opt/myapp/codex codex exec "Quick health check" --skip-git-repo-check` | Binary order: `CODEX_BINARY` > `CODEX_BUNDLED_PATH` > `<crate>/bin/codex`. |
| `cargo run -p codex --example bundled_binary_home -- "Health check prompt"` | `CODEX_HOME="C:\\data\\codex" C:\\apps\\codex\\bin\\codex.exe exec "Health check prompt" --skip-git-repo-check` | Bundled binary with app-scoped `CODEX_HOME`; prints `CodexHomeLayout` paths and can create the isolated tree before spawning. |
| `CODEX_HOME=/tmp/codex-demo cargo run -p codex --example codex_home -- "Show CODEX_HOME contents"` | `CODEX_HOME=/tmp/codex-demo codex exec "Show CODEX_HOME contents" --skip-git-repo-check` | App-scoped `CODEX_HOME` showing config/auth/history/log paths. |

## Streaming & Logging

| Wrapper example | Native command | Notes |
| --- | --- | --- |
| `cargo run -p codex --example json_stream -- "Summarize repo status"` | `echo "Summarize repo status" \| codex exec --skip-git-repo-check --json` | Enable JSONL streaming; prompt is piped via stdin. |
| `cargo run -p codex --example stream_events -- "Summarize repo status"` | `echo "Summarize repo status" \| codex exec --skip-git-repo-check --json --timeout 0` | Typed consumer for `thread/turn/item` events (agent_message, reasoning, command_execution, file_change, mcp_tool_call, web_search, todo_list) plus `turn.failed`; `--sample` replays bundled events. |
| `cargo run -p codex --example stream_last_message -- "Summarize repo status"` | `codex exec --skip-git-repo-check --json --output-last-message <path> --output-schema <path> <<<"Summarize repo status"` | Reads `--output-last-message` + `--output-schema` files (thread/turn metadata included); ships sample payloads if no binary. |
| `CODEX_LOG_PATH=/tmp/codex.log cargo run -p codex --example stream_with_log -- "Stream with logging"` | `echo "Stream with logging" \| codex exec --skip-git-repo-check --json` | Mirrors stdout and tees JSONL events to `CODEX_LOG_PATH` (or uses sample events with IDs/status). |

## MCP + App Server

| Wrapper example | Native command | Notes |
| --- | --- | --- |
| `cargo run -p codex --example mcp_codex_flow -- "Draft a plan" ["Tighten scope"]` | `codex mcp-server --stdio` then call `codex/codex` + `codex/codex-reply` | Typed `codex::mcp` helper that streams `codex/event`, supports `$ /cancelRequest`, and chains a follow-up when the first call returns a conversation ID; gate with `feature_detection` if the binary lacks MCP endpoints. |
| `cargo run -p codex --example mcp_codex_tool -- "Summarize repo status"` | `codex mcp-server --stdio` then send `tools/codex` JSON-RPC call | Streams codex tool notifications (approval/task_complete); `--sample` and optional `CODEX_HOME` for isolation. |
| `CODEX_CONVERSATION_ID=abc123 cargo run -p codex --example mcp_codex_reply -- "Continue the prior run"` | `codex mcp-server --stdio` then call `tools/codex-reply` with `conversationId=abc123` | Resume a session via `codex-reply`; needs `CODEX_CONVERSATION_ID` or first arg; `--sample` available. |
| `cargo run -p codex --example app_server_turns -- "Draft a release note" [thread-id]` | `codex app-server --stdio` then `thread/start` or `thread/resume` plus `turn/start` (optional `turn/interrupt`) | Uses the `codex::mcp` app-server client to stream items and task_complete notices, optionally resuming a thread and sending `turn/interrupt` after a delay; pair with `feature_detection` if the binary omits app-server support. |
| `cargo run -p codex --example app_server_thread_turn -- "Draft a release note"` | `codex app-server --stdio` then send `thread/start` and `turn/start` | App-server thread/turn notifications; supports `--sample` and optional `CODEX_HOME` for state isolation. |

## Feature Detection

| Wrapper example | Native command | Notes |
| --- | --- | --- |
| `cargo run -p codex --example feature_detection` | `codex --version` and `codex features list` | Probes version + feature list, gates streaming/log-tee/artifact flags, and emits upgrade advisories; falls back to sample data. |

## Discovering `CODEX_HOME` layout

Use `CodexHomeLayout` to inspect where Codex stores config, credentials, history, conversations, and logs when you set an app-scoped `CODEX_HOME`:

```rust
use codex::CodexHomeLayout;

let layout = CodexHomeLayout::new("/apps/myhub/codex");
println!("Config: {}", layout.config_path().display());
println!("History: {}", layout.history_path().display());
println!("Conversations: {}", layout.conversations_dir().display());
println!("Logs: {}", layout.logs_dir().display());

// Optional: create the CODEX_HOME directories yourself before spawning Codex.
layout.materialize(true).expect("failed to prepare CODEX_HOME");
```

Use these pairs as a checklist when validating parity between the Rust wrapper and the raw Codex CLI.
