# Manual Testing Playbook — Claude Code live stream-json

Status: Draft  
Date (UTC): 2026-02-18  
Feature directory: `docs/project_management/next/claude-code-live-stream-json/`

## Purpose

Provide non-gating, operator-run steps to validate “live” behavior using a real `claude` binary on a
local macOS development environment.

This playbook validates two independent properties:
1) `crates/claude_code` streaming handle yields events before process exit.
2) `crates/agent_api` Universal Agent API DR-0012 completion gating holds: completion does not resolve until the event
   stream is final (or explicitly dropped by the consumer).

## Preconditions

- macOS host with Rust toolchain installed (`rustc`/`cargo`).
- A working `claude` CLI installed and authenticated in your environment:
  - `claude --version` must succeed.
- You are on a commit that includes:
  - `ClaudeClient::print_stream_json(...)` in `crates/claude_code`
  - `agent_api` Claude backend wiring + `agent_api.events.live`
  - (optional but helpful) CP1 smoke workflow/smoke scripts landed

Normative references (read before running):
- `contract.md` (pinned API + cancellation/timeout semantics)
- `stream-json-print-protocol-spec.md` (pinned backpressure/channel capacity/kill_on_drop)
- `platform-parity-spec.md` (parity envelope)

## Step 1 — Sanity preflight (macOS)

From repo root:

1. Confirm the Claude binary is available:
   - `which claude`
   - `claude --version`
2. Run local gates (so manual observations aren’t confounded by unrelated failures):
   - `cargo fmt --all -- --check`
   - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
   - `cargo test -p claude_code --all-targets --all-features`
   - `cargo test -p agent_api --all-targets --all-features`

## Step 2 — Validate `claude_code` live streaming (real `claude` binary)

This check asserts: “first streamed event arrives before `completion` resolves”.

1) Create a scratch binary project (outside the repo):

```bash
REPO_ROOT="$(git rev-parse --show-toplevel)"
TMP_ROOT="$(mktemp -d)"
SCRATCH="$TMP_ROOT/ccsj-manual"
cargo new --bin "$SCRATCH"
```

2) Replace scratch `Cargo.toml` with path deps to this repo:

```bash
cat > "$SCRATCH/Cargo.toml" <<TOML
[package]
name = "ccsj-manual"
version = "0.1.0"
edition = "2021"

[dependencies]
claude_code = { path = "$REPO_ROOT/crates/claude_code" }
tokio = { version = "1.38", features = ["macros", "rt-multi-thread", "time"] }
futures-util = "0.3"
TOML
```

3) Write `src/main.rs`:

```bash
cat > "$SCRATCH/src/main.rs" <<'RS'
use std::time::Duration;

use futures_util::StreamExt;

#[tokio::main]
async fn main() {
    let client = claude_code::ClaudeClient::builder().build();

    let req = claude_code::ClaudePrintRequest::new(
        "Write 30 short numbered lines (1..30), one per line. No preamble.",
    )
    .output_format(claude_code::ClaudeOutputFormat::StreamJson)
    .include_partial_messages(true);

    let handle = client
        .print_stream_json(req)
        .await
        .expect("print_stream_json failed");

    let mut events = handle.events;
    let mut completion = handle.completion;

    // Deterministic assertion: completion MUST NOT resolve before we can observe the first event.
    // We allow up to 60s for the first event (slow network / cold start / auth).
    let first = tokio::time::timeout(Duration::from_secs(60), async {
        tokio::select! {
            _ = &mut completion => {
                panic!("FAIL: completion resolved before first streamed event (not live)");
            }
            ev = events.next() => {
                ev.expect("stream ended before first event")
            }
        }
    })
    .await
    .expect("timed out waiting for first event");

    match first {
        Ok(_ev) => {
            println!("OK: observed first streamed event before completion");
        }
        Err(err) => {
            // Real-binary runs can produce parse errors; they MUST be redacted (no raw JSON line).
            println!("OK: first item was a redacted parse error (still satisfies 'live'): {err}");
        }
    }
}
RS
```

4) Run it:

```bash
(cd "$SCRATCH" && cargo run --quiet)
```

Expected result:
- Program prints `OK: observed first streamed event before completion`.

## Step 3 — Validate `agent_api` Universal Agent API DR-0012 completion gating (real `claude` binary)

This check asserts:
- `agent_api` completion does **not** resolve while the event stream is neither drained to finality nor dropped.
- Dropping the event stream allows completion to resolve (consumer opt-out), per Universal Agent API DR-0012.

1) Update the same scratch project’s `Cargo.toml` to include `agent_api` (feature `claude_code`):

```bash
REPO_ROOT="$(git rev-parse --show-toplevel)"
cat > "$SCRATCH/Cargo.toml" <<TOML
[package]
name = "ccsj-manual"
version = "0.1.0"
edition = "2021"

[dependencies]
agent_api = { path = "$REPO_ROOT/crates/agent_api", features = ["claude_code"] }
claude_code = { path = "$REPO_ROOT/crates/claude_code" }
tokio = { version = "1.38", features = ["macros", "rt-multi-thread", "time"] }
futures-util = "0.3"
TOML
```

2) Replace `src/main.rs` with a Universal Agent API DR-0012 gating check:

```bash
cat > "$SCRATCH/src/main.rs" <<'RS'
use std::{sync::Arc, time::Duration};

use futures_util::StreamExt;

#[tokio::main]
async fn main() {
    let mut gw = agent_api::AgentWrapperGateway::new();

    let backend = agent_api::backends::claude_code::ClaudeCodeBackend::new(
        agent_api::backends::claude_code::ClaudeCodeBackendConfig {
            binary: None, // relies on PATH; set Some(PathBuf::from("...")) to pin
            default_timeout: None,
            default_working_dir: None,
            env: Default::default(),
        },
    );
    let backend = Arc::new(backend);

    let caps = backend.capabilities();
    assert!(
        caps.contains("agent_api.events.live"),
        "FAIL: backend capabilities missing agent_api.events.live"
    );

    gw.register(backend).expect("register backend");

    let kind = agent_api::AgentWrapperKind::new("claude_code").unwrap();
    let req = agent_api::AgentWrapperRunRequest {
        prompt: "Write 30 short numbered lines (1..30), one per line. No preamble.".to_string(),
        ..Default::default()
    };

    let handle = gw.run(&kind, req).await.expect("run failed");

    // --- Universal Agent API DR-0012 gating check (deterministic) ---
    //
    // If we do not drain the event stream AND do not drop it, completion MUST NOT resolve.
    // (Finality is signaled only when the stream is exhausted via polling, or dropped.)
    let mut events = handle.events;
    let mut completion = handle.completion;

    // Ensure the run has actually started producing events before asserting gating (avoids false
    // failures if spawn/auth fails immediately).
    let _first = tokio::time::timeout(Duration::from_secs(60), async {
        tokio::select! {
            _ = completion.as_mut() => {
                panic!("FAIL: completion resolved before first event (not live or run failed early)");
            }
            ev = events.next() => {
                ev.expect("stream ended before first event")
            }
        }
    })
    .await
    .expect("timed out waiting for first event");

    let gated = tokio::time::timeout(Duration::from_millis(250), completion.as_mut()).await;
    assert!(
        gated.is_err(),
        "FAIL: completion resolved even though events were not drained or dropped"
    );

    // Now drop the stream (consumer opt-out / cancellation) and ensure completion can resolve.
    drop(events);

    // PASS criteria: completion resolves (Ok or Err) within a bounded time after dropping the stream.
    let resolved = tokio::time::timeout(Duration::from_secs(10), completion).await;
    assert!(
        resolved.is_ok(),
        "FAIL: completion did not resolve within 10s after dropping the events stream"
    );

    println!("OK: completion was gated until stream was dropped (Universal Agent API DR-0012)");
}
RS
```

3) Run it:

```bash
(cd "$SCRATCH" && cargo run --quiet)
```

Expected result:
- Program prints `OK: completion was gated until stream was dropped (Universal Agent API DR-0012)`.

## Record

Record results (time, SHA, command used, observations) in:
- `docs/project_management/next/claude-code-live-stream-json/session_log.md`
