# Codex Wrapper Examples vs. Native CLI

Every example under `crates/codex/examples/` maps to a `codex` CLI invocation. Use the sections below to compare wrapper calls (`cargo run -p codex --example ...`) with the equivalent raw commands and required env vars.

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
| `cargo run -p ingestion --example ingest_to_codex -- --instructions "Summarize the documents" --model gpt-5-codex --json --include-prompt --image "C:\\Docs\\mockup.png" C:\\Docs\\spec.pdf` | `codex exec --skip-git-repo-check --json --model gpt-5-codex --image "C:\\Docs\\mockup.png" "<constructed prompt covering spec.pdf>"` | Ingestion harness builds the prompt before calling `codex exec`. |

## Binary & CODEX_HOME

| Wrapper example | Native command | Notes |
| --- | --- | --- |
| `$env:CODEX_BINARY="C:\\bin\\codex-nightly.exe"; cargo run -p codex --example env_binary -- "Nightly sanity check"` | `C:\\bin\\codex-nightly.exe exec "Nightly sanity check" --skip-git-repo-check` | Honors `CODEX_BINARY` override. |
| `CODEX_BUNDLED_PATH=/opt/myapp/codex cargo run -p codex --example bundled_binary -- "Quick health check"` | `CODEX_BINARY=/opt/myapp/codex codex exec "Quick health check" --skip-git-repo-check` | Prefers bundled binary, falls back to `CODEX_BINARY`. |
| `CODEX_HOME=/tmp/codex-demo cargo run -p codex --example codex_home -- "Show CODEX_HOME contents"` | `CODEX_HOME=/tmp/codex-demo codex exec "Show CODEX_HOME contents" --skip-git-repo-check` | Uses an app-scoped CODEX_HOME (config/auth/history/logs). |

## Streaming & Logging

| Wrapper example | Native command | Notes |
| --- | --- | --- |
| `cargo run -p codex --example json_stream -- "Summarize repo status"` | `echo "Summarize repo status" \| codex exec --skip-git-repo-check --json` | Enable JSONL streaming; prompt is piped via stdin. |
| `cargo run -p codex --example stream_events -- "Summarize repo status"` | `echo "Summarize repo status" \| codex exec --skip-git-repo-check --json` | Typed event consumer; supports `--sample` when no binary is present. |
| `cargo run -p codex --example stream_last_message -- "Summarize repo status"` | `codex exec --skip-git-repo-check --json --output-last-message <path> --output-schema <path> <<<"Summarize repo status"` | Reads `--output-last-message` and `--output-schema` artifacts (falls back to samples). |
| `CODEX_LOG_PATH=/tmp/codex.log cargo run -p codex --example stream_with_log -- "Stream with logging"` | `echo "Stream with logging" \| codex exec --skip-git-repo-check --json` | Mirrors stdout and tees the JSONL stream to `CODEX_LOG_PATH` (or uses sample events). |

## MCP + App Server

| Wrapper example | Native command | Notes |
| --- | --- | --- |
| `cargo run -p codex --example mcp_codex_tool -- "Summarize repo status"` | `codex mcp-server --stdio` then send `tools/codex` JSON-RPC call | Streams codex tool notifications; `--sample` prints mocked approval/task_complete events. |
| `CODEX_CONVERSATION_ID=abc123 cargo run -p codex --example mcp_codex_reply -- "Continue the prior run"` | `codex mcp-server --stdio` then call `tools/codex-reply` with `conversationId=abc123` | Resumes a session; requires `CODEX_CONVERSATION_ID` or first arg, supports `--sample`. |
| `cargo run -p codex --example app_server_thread_turn -- "Draft a release note"` | `codex app-server --stdio` then send `thread/start` and `turn/start` | Demonstrates app-server thread/turn notifications; `--sample` prints mocked flow. |

## Feature Detection

| Wrapper example | Native command | Notes |
| --- | --- | --- |
| `cargo run -p codex --example feature_detection` | `codex --version` and `codex features list` | Probes version + feature list to gate streaming/logging; emits sample data when the binary is missing. |
