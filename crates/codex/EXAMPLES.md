# Codex Wrapper Examples vs. Native CLI

Every example under `crates/codex/examples/` maps to a `codex` CLI invocation. Wrapper calls (`cargo run -p codex --example ...`) run with safe defaults: `--skip-git-repo-check`, temp working dirs unless overridden, 120s timeout, ANSI color disabled, and `RUST_LOG=error` unless set. Select the binary with `CODEX_BINARY` or `.binary(...)`; set `CODEX_HOME` to keep config/auth/history/logs under an app-scoped directory. Examples labeled `--sample` print fixture data captured from the live CLI (compact `thread.started`/`turn.started`/`item.completed`/`turn.completed` events plus MCP/app-server notifications) when you do not have a binary handy; streaming/resume/apply fixtures live in `crates/codex/examples/fixtures/*` so docs and samples stay aligned.

## Basics

| Wrapper example | Native command | Notes |
| --- | --- | --- |
| `cargo run -p codex --example send_prompt -- "List Rust toolchain commands"` | `codex exec "List Rust toolchain commands" --skip-git-repo-check` | Baseline prompt with default timeout/temp dir. |
| `cargo run -p codex --example timeout -- "List long-running tasks"` | `codex exec "List long-running tasks" --skip-git-repo-check` | Wrapper-enforced 30‑second timeout (CLI flag no longer exposed). |
| `cargo run -p codex --example timeout_zero -- "Stream until completion"` | `codex exec "Stream until completion" --skip-git-repo-check` | Disables the wrapper timeout; CLI currently offers no `--timeout` flag. |
| `cargo run -p codex --example working_dir -- "C:\\path\\to\\repo" "List files here"` | `codex exec "List files here" --skip-git-repo-check --cd "C:\\path\\to\\repo"` | Run inside a specific directory. |
| `cargo run -p codex --example working_dir_json -- "C:\\path\\to\\repo" "Summarize repo status"` | `echo "Summarize repo status" \| codex exec --skip-git-repo-check --json --cd "C:\\path\\to\\repo"` | Combines working dir override with JSON streaming. |
| `cargo run -p codex --example select_model -- gpt-5-codex -- "Explain rustfmt defaults"` | `codex exec "Explain rustfmt defaults" --skip-git-repo-check --model gpt-5-codex` | Picks a specific model. |
| `cargo run -p codex --example color_always -- "Show colorful output"` | `codex exec "Show colorful output" --skip-git-repo-check --color always` | Forces ANSI color codes. |
| `cargo run -p codex --example image_json -- "C:\\path\\to\\mockup.png" "Describe the screenshot"` | `echo "Describe the screenshot" \| codex exec --skip-git-repo-check --json --image "C:\\path\\to\\mockup.png"` | Attach an image while streaming JSON quietly. |
| `cargo run -p codex --example quiet -- "Run without tool noise"` | `codex exec "Run without tool noise" --skip-git-repo-check --quiet` | Suppress stderr mirroring. |
| `cargo run -p codex --example no_stdout_mirror -- "Stream quietly"` | `codex exec "Stream quietly" --skip-git-repo-check > out.txt` | Disable stdout mirroring to capture output yourself. |

## Binary & CODEX_HOME

| Wrapper example | Native command | Notes |
| --- | --- | --- |
| `$env:CODEX_BINARY="C:\\bin\\codex-nightly.exe"; cargo run -p codex --example env_binary -- "Nightly sanity check"` | `C:\\bin\\codex-nightly.exe exec "Nightly sanity check" --skip-git-repo-check` | Honors `CODEX_BINARY` override. |
| `CODEX_BUNDLED_PATH=/opt/myapp/codex cargo run -p codex --example bundled_binary -- "Quick health check"` | `CODEX_BINARY=/opt/myapp/codex codex exec "Quick health check" --skip-git-repo-check` | Binary order: `CODEX_BINARY` > `CODEX_BUNDLED_PATH` > `<crate>/bin/codex`. |
| `CODEX_HOME=/tmp/codex-demo cargo run -p codex --example codex_home -- "Show CODEX_HOME contents"` | `CODEX_HOME=/tmp/codex-demo codex exec "Show CODEX_HOME contents" --skip-git-repo-check` | App-scoped `CODEX_HOME` showing config/auth/history/log paths. |

## Streaming & Logging

| Wrapper example | Native command | Notes |
| --- | --- | --- |
| `cargo run -p codex --example json_stream -- "Summarize repo status"` | `echo "Summarize repo status" \| codex exec --skip-git-repo-check --json` | Enable JSONL streaming; prompt is piped via stdin. |
| `cargo run -p codex --example stream_events -- "Summarize repo status"` | `echo "Summarize repo status" \| codex exec --skip-git-repo-check --json --sandbox read-only --color never` | Typed consumer for the compact `thread.started`/`turn.started`/`item.completed`/`turn.completed` events (token usage only on completion; per-event IDs omitted by the CLI); includes idle-timeout handling and a `--sample` replay path. |
| `cargo run -p codex --example stream_last_message -- "Summarize repo status"` | `codex exec --skip-git-repo-check --json --output-last-message <path> --output-schema <path> <<<"Summarize repo status"` | Reads `--output-last-message` + `--output-schema` files (thread/turn metadata included); ships sample payloads if no binary. |
| `CODEX_LOG_PATH=/tmp/codex.log cargo run -p codex --example stream_with_log -- "Stream with logging"` | `echo "Stream with logging" \| codex exec --skip-git-repo-check --json --sandbox read-only --color never` | Mirrors stdout and tees JSONL events to `CODEX_LOG_PATH` (or uses the live-captured sample events in `crates/codex/examples/fixtures/streaming.jsonl`). |

## Resume & Apply/Diff

| Wrapper example | Native command | Notes |
| --- | --- | --- |
| `CODEX_CONVERSATION_ID=abc cargo run -p codex --example resume_apply` | `codex exec --json --skip-git-repo-check --sandbox read-only resume --last` (optionally followed by `codex apply <task-id>`) | Streams resume events from the live CLI (`thread.started` + compact item.completed events); the CLI no longer exposes `diff`/JSON apply helpers, so the example falls back to fixture diff/apply payloads unless `--apply-task <id>` is provided. |

## MCP + App Server

| Wrapper example | Native command | Notes |
| --- | --- | --- |
| `cargo run -p codex --example mcp_codex_tool -- "Summarize repo status"` | `codex mcp-server --stdio` then send `tools/codex` JSON-RPC call | Streams codex tool notifications (approval/task_complete); `--sample` and optional `CODEX_HOME` for isolation. |
| `CODEX_CONVERSATION_ID=abc123 cargo run -p codex --example mcp_codex_reply -- "Continue the prior run"` | `codex mcp-server --stdio` then call `tools/codex-reply` with `conversationId=abc123` | Resume a session via `codex-reply`; needs `CODEX_CONVERSATION_ID` or first arg; `--sample` available. |
| `cargo run -p codex --example app_server_thread_turn -- "Draft a release note"` | `codex app-server --stdio` then send `thread/start` and `turn/start` | App-server thread/turn notifications; supports `--sample` and optional `CODEX_HOME` for state isolation. |

## Feature Detection

| Wrapper example | Native command | Notes |
| --- | --- | --- |
| `cargo run -p codex --example feature_detection` | `codex --version` and `codex features list` | Probes version + feature list (per-binary cache), gates streaming/log-tee/resume/apply/artifact flags, and emits upgrade advisories; falls back to sample data. |
