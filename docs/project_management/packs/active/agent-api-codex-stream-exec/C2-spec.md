# C2 Spec — Validation Hardening (fixtures + evidence for streaming parity)

Status: Draft  
Date (UTC): 2026-02-20  
Owner: agent-api-codex-stream-exec triad (C2)

## Scope (required)

Add validation artifacts that prove the refactor preserves required universal semantics without
requiring a real Codex installation on CI runners.

### In-scope deliverables

- A cross-platform fake-binary / fixture strategy that simulates:
  - streaming at least one JSONL event before process exit
  - emitting a deliberately malformed JSONL line that would trigger
    `ExecStreamError::{Parse,Normalize}` (raw line embedded in error)
  - reading env vars inside the child to prove env precedence
- A deterministic validation strategy for exec-policy defaults + overrides:
  - default non-interactive behavior (no prompts/hangs)
  - default sandbox mode (`workspace-write`)
  - per-run override to `danger-full-access` for hosts that need “no sandbox”
- Tests that prove (must be explicit assertions):
  1. **Live event before completion**: at least one `AgentWrapperEvent` is observed before
     `AgentWrapperRunHandle.completion` resolves (Codex backend advertises `agent_api.events.live`).
  2. **Env precedence**: for the same key, request env overrides backend env.
  3. **Redaction**: no emitted universal error/event message contains the raw JSONL line (including
     those embedded in `ExecStreamError` display/fields).
  4. **Exec policy**: default flags/behavior match `contract.md`, and overrides via extensions are
     honored deterministically.

### Out of scope (explicit)

- Tests that require a real Codex CLI on CI.
- Any changes to Rust crate behavior beyond what is required to support the tests.

## Acceptance Criteria (observable)

- On GitHub-hosted runners (Linux/macOS/Windows), `cargo test -p agent_api --features codex`
  passes without requiring a real Codex install.
- The test suite includes at least:
  - one test for live-event-before-completion
  - one test for env precedence
  - one test for redaction against `ExecStreamError::{Parse,Normalize}` raw-line embedding.

## Deterministic implementation contract (normative; removes ambiguity)

### Fake Codex binary

- The fake binary MUST be implemented as a Cargo bin target:
  - `crates/agent_api/src/bin/fake_codex_stream_json_agent_api.rs`
- Tests MUST locate the compiled binary via:
  - `env!("CARGO_BIN_EXE_fake_codex_stream_json_agent_api")`
- The fake binary MUST support scenario selection via an env var:
  - `FAKE_CODEX_SCENARIO`
  - Allowed values (normative):
    - `live_two_events_long_delay`
    - `emit_normalize_error_with_rawline_secret`
    - `dump_env_then_exit`

Exec policy expectations (normative; removes ambiguity):
- The fake binary MUST validate the universal backend’s spawn flags using:
  - `FAKE_CODEX_EXPECT_SANDBOX` (string; default `workspace-write`)
  - `FAKE_CODEX_EXPECT_APPROVAL` (string; default `never`)
- If `FAKE_CODEX_EXPECT_APPROVAL="<absent>"`, the fake binary MUST assert the argv does not
  include `--ask-for-approval`.

### Scenario requirements

1) `live_two_events_long_delay`
   - MUST emit at least one valid JSONL event line immediately (e.g., `thread.started`).
   - MUST then sleep for ≥ 250ms before exiting, so tests can prove an event is observable while
     completion is still pending.

2) `emit_normalize_error_with_rawline_secret`
   - MUST emit a line that parses as JSON but is missing required context so the Codex wrapper
     produces `ExecStreamError::Normalize { line, message }`.
   - The emitted JSON line MUST include a unique sentinel string:
     - `RAWLINE_SECRET_DO_NOT_LEAK`
   - Tests MUST assert that this sentinel does NOT appear in any universal emitted error message
     or event (redaction proof).

3) `dump_env_then_exit`
   - MUST read env and write it deterministically to a path provided by:
     - `CODEX_WRAPPER_TEST_DUMP_ENV` (absolute or relative file path; created with parent dirs)
   - Tests MUST compare the dump content to prove precedence:
     - same key in backend config env and request env → request env wins.

### Test file and assertions

- The C2 integration tests MUST live at:
  - `crates/agent_api/tests/c2_codex_stream_exec_parity.rs`
- Required tests (normative):
  - `events_are_observable_before_process_exit` (like the Claude backend test, but for Codex)
  - `request_env_overrides_backend_env` (assert by reading the dump file)
  - `redaction_does_not_leak_raw_jsonl_line` (assert sentinel absence)
