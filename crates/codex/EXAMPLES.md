# Codex Wrapper Examples vs. Native CLI

Every example under `crates/codex/examples/` corresponds to a specific `codex exec` flag combo. Use the table below to compare the wrapper invocation (`cargo run -p codex --example ...`) with the equivalent raw CLI call.

| Wrapper example | Native `codex exec` command | Notes |
| --- | --- | --- |
| `cargo run -p codex --example send_prompt -- "List Rust toolchain commands"` | `codex exec "List Rust toolchain commands" --skip-git-repo-check` | Baseline prompt with default timeout/temp dir. |
| `cargo run -p codex --example timeout -- "List long-running tasks"` | `codex exec "List long-running tasks" --skip-git-repo-check --timeout 30` | Forces a 30‑second timeout. |
| `cargo run -p codex --example timeout_zero -- "Stream until completion"` | `codex exec "Stream until completion" --skip-git-repo-check --timeout 0` | Disables the wrapper timeout. |
| `cargo run -p codex --example working_dir -- "C:\path\to\repo" "List files here"` | `codex exec "List files here" --skip-git-repo-check --cd "C:\path\to\repo"` | Runs Codex inside a specific directory. |
| `$env:CODEX_BINARY="C:\bin\codex-nightly.exe"; cargo run -p codex --example env_binary -- "Nightly sanity check"` | `C:\bin\codex-nightly.exe exec "Nightly sanity check" --skip-git-repo-check` | Demonstrates honoring `CODEX_BINARY`. |
| `cargo run -p codex --example select_model -- gpt-5-codex -- "Explain rustfmt defaults"` | `codex exec "Explain rustfmt defaults" --skip-git-repo-check --model gpt-5-codex` | Picks a specific model. |
| `cargo run -p codex --example color_always -- "Show colorful output"` | `codex exec "Show colorful output" --skip-git-repo-check --color always` | Forces ANSI color codes. |
| `cargo run -p codex --example image_json -- "C:\path\to\mockup.png" "Describe the screenshot"` | `echo "Describe the screenshot" \| codex exec --skip-git-repo-check --json --image "C:\path\to\mockup.png"` | Attaches an image while streaming JSON quietly. |
| `cargo run -p codex --example json_stream -- "Summarize repo status"` | `echo "Summarize repo status" \| codex exec --skip-git-repo-check --json` | Enables JSONL streaming; prompt is piped via stdin. |
| `cargo run -p codex --example working_dir_json -- "C:\path\to\repo" "Summarize repo status"` | `echo "Summarize repo status" \| codex exec --skip-git-repo-check --json --cd "C:\path\to\repo"` | Combines working dir override with JSON streaming. |
| `cargo run -p codex --example quiet -- "Run without tool noise"` | `codex exec "Run without tool noise" --skip-git-repo-check --quiet` | Suppresses stderr mirroring. |
| `cargo run -p codex --example no_stdout_mirror -- "Stream quietly"` | `codex exec "Stream quietly" --skip-git-repo-check > out.txt` | Disables stdout mirroring on the wrapper so you can capture output yourself. |
| `cargo run -p codex --example send_prompt --color never -- "Show monochrome"` | `codex exec "Show monochrome" --skip-git-repo-check --color never` | (Color example also works for `auto`/`never`.) |
| `cargo run -p ingestion --example ingest_to_codex -- --instructions "Summarize the documents" --model gpt-5-codex --json --include-prompt --image "C:\Docs\mockup.png" C:\Docs\spec.pdf` | `codex exec --skip-git-repo-check --json --model gpt-5-codex --image "C:\Docs\mockup.png" "<constructed prompt covering spec.pdf>"` | Full ingestion harness: it builds the multi-document prompt before calling `codex exec`. |

Use these pairs as a checklist when validating parity between the Rust wrapper and the raw Codex CLI.

## MCP + app-server examples

| Wrapper example | Behavior | Notes |
| --- | --- | --- |
| `cargo run -p codex --example mcp_codex_flow -- "<prompt>" ["<follow up>"]` | Launches `codex mcp-server`, sends `codex/codex`, streams `codex/event`, and optionally cancels or issues a follow-up `codex/codex-reply`. | Respects `CODEX_BINARY`/`CODEX_HOME`; uses `CANCEL_AFTER_MS` to trigger `$ /cancelRequest`; does not write to `[mcp_servers]`. |
| `cargo run -p codex --example app_server_turns -- "<prompt>" [thread-id]` | Starts or resumes a thread, sends `turn/start`, streams items/task_complete, and can issue `turn/interrupt` after the first item. | Respects `CODEX_BINARY`/`CODEX_HOME`; uses `INTERRUPT_AFTER_MS` to delay interrupts; leaves stored app runtime definitions and thread metadata untouched. |
