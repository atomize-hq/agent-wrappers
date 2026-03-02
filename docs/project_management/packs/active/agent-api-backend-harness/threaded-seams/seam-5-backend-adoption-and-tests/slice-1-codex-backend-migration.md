# S1 — Migrate Codex backend to the harness (behavior-equivalent)

- **User/system value**: Eliminates duplicated “glue” logic in the Codex backend by routing through the shared harness, reducing drift and proving the harness is viable for real backends.
- **Scope (in/out)**:
  - In:
    - Refactor `crates/agent_api/src/backends/codex.rs` to:
      - implement the `BH-C01` adapter shape, and
      - delegate request validation (`BH-C02`), env/timeout derivation (`BH-C03`), stream pump (`BH-C04`), bounds enforcement, and completion gating (`BH-C05`) to the harness.
    - Keep Codex-specific concerns in Codex-owned code:
      - extension parsing into a typed “exec policy” (post-allowlist),
      - spawn/build of `codex::CodexClient`,
      - mapping of Codex typed events (`ThreadEvent`) to `AgentWrapperEvent` via `backends/codex/mapping.rs`,
      - redaction of Codex backend errors (no raw stderr leakage).
  - Out:
    - Any changes to Codex capability IDs / extension keys.
    - Changes to harness-owned semantics (treat as upstream seam work).
- **Acceptance criteria**:
  - `CodexBackend::run` constructs an `AgentWrapperRunHandle` via the harness entrypoint (not via a backend-local channel + drain loop).
  - Codex backend code no longer contains backend-local re-implementations of:
    - unknown extension key rejection,
    - env merge precedence,
    - timeout derivation defaults,
    - bounds enforcement on forwarded events/completion,
    - drain-on-drop stream forwarding loop,
    - completion gating integration.
  - Existing Codex mapping tests remain valid (or are adjusted only for refactor-induced symbol moves).
- **Dependencies**:
  - Upstream seams:
    - SEAM-1 (`BH-C01`)
    - SEAM-2 (`BH-C02`, `BH-C03`)
    - SEAM-3 (`BH-C04`)
    - SEAM-4 (`BH-C05`)
- **Verification**:
  - `cargo test -p agent_api --features codex`
  - `cargo test -p agent_api --features codex,claude_code`

## Atomic Tasks

#### S1.T1 — Implement the Codex adapter for `BH-C01` (spawn + mapping + completion extraction)

- **Outcome**: A Codex-owned adapter implementation that provides the harness the minimal backend-specific hooks: how to spawn, how to map typed events, and how to extract an `AgentWrapperCompletion`.
- **Inputs/outputs**:
  - Output: `crates/agent_api/src/backends/codex.rs`
  - Input: existing Codex mapping module (`crates/agent_api/src/backends/codex/mapping.rs`)
  - Input: existing Codex wrapper types (`codex::{ExecStreamRequest, ThreadEvent, ExecCompletion, ...}`)
- **Implementation notes**:
  - Keep `validate_and_extract_exec_policy` (or rename) focused on parsing/validation of known keys only; do not duplicate allowlist behavior (`BH-C02` is harness-owned).
  - Preserve current redaction rules by exposing a Codex-specific error-to-event mapping hook (as supported by `BH-C01`).
  - Codex allowlisted extension keys (v1; must be accepted by BH-C02 when `agent_kind == "codex"`):
    - Core key:
      - `agent_api.exec.non_interactive` (boolean; default `true` when absent per `extensions-spec.md`)
    - Codex backend keys:
      - `backend.codex.exec.approval_policy` (string enum: `untrusted | on-failure | on-request | never`)
      - `backend.codex.exec.sandbox_mode` (string enum: `read-only | workspace-write | danger-full-access`)
    - No other `request.extensions` keys are permitted for the Codex backend in v1.
  - Single source of truth (v1):
    - These extension keys MUST appear verbatim in `CodexBackend::capabilities().ids`.
    - The harness allowlist MUST be provided via `BackendHarnessAdapter::supported_extension_keys()`
      and MUST match the backend’s advertised capabilities for extension keys (exact string match).
- **Acceptance criteria**:
  - Adapter declares:
    - `agent_kind == "codex"`,
    - the allowlisted extension keys for Codex (including Codex-specific exec policy keys),
    - a spawn implementation that returns a typed event stream + completion future,
    - mapping functions that reuse existing Codex mapping helpers.
- **Risk/rollback notes**:
  - Treat any observable behavior diffs as regressions; prefer pinning behavior via harness tests in upstream seams rather than encoding Codex-only exceptions.

Checklist:
- Identify the minimal adapter surface required by `BH-C01` and implement it in Codex.
- Ensure extension allowlist includes Codex-only keys (no behavior change).
- Keep error redaction for Codex stream/exec errors intact.
 - Keep/extend the existing Codex capability-reporting test in `crates/agent_api/src/backends/codex/tests.rs`
   so a change to the allowlist is caught immediately.

#### S1.T2 — Rewire `CodexBackend::run` to call the harness entrypoint (remove local run-loop)

- **Outcome**: Codex backend’s `run` is a thin “configure adapter + call harness” wrapper.
- **Inputs/outputs**:
  - Output: `crates/agent_api/src/backends/codex.rs`
  - Output: remove backend-local run-loop helpers that duplicate `BH-C03/BH-C04/BH-C05` behavior.
- **Implementation notes**:
  - Remove (or stop using) backend-local `mpsc` channel plumbing and backend-local completion gating.
  - Remove the backend-local drain loop helper and route through the harness pump (per `BH-C04`).
  - Remove backend-local prompt-empty validation if the harness has a universal invalid request check (SEAM-2).
- **Acceptance criteria**:
  - `codex.rs` no longer contains a drain loop equivalent to `drain_events_while_polling_completion`.
  - `codex.rs` no longer directly applies `crate::bounds` to forwarded events/completion (the harness does this).
  - `codex.rs` no longer directly calls `crate::run_handle_gate::build_gated_run_handle` (the harness does this via `BH-C05` integration).

Checklist:
- Replace the run implementation with the harness entrypoint call.
- Delete or inline-remove legacy backend-local orchestration code.
- Ensure feature flags (`--features codex`) still compile cleanly.

#### S1.T3 — Adjust Codex backend tests for the new architecture (keep mapping tests; drop pump tests)

- **Outcome**: Codex backend tests remain valuable and stop asserting harness-owned semantics at the backend layer.
- **Inputs/outputs**:
  - Output: `crates/agent_api/src/backends/codex/tests.rs`
- **Implementation notes**:
  - Keep mapping/capabilities tests as-is (they validate Codex-specific mapping behavior).
  - If there are tests exercising backend-local drain/pump helpers, migrate those expectations to the harness-layer tests owned by SEAM-3/SEAM-4 instead of keeping a duplicate “Codex pump” fixture.
- **Acceptance criteria**:
  - Codex backend tests do not re-test harness invariants (`BH-C02/BH-C03/BH-C04/BH-C05`) that are already covered in upstream seams.
  - Test suite continues to provide Codex-specific value (mapping correctness, capability reporting).

Checklist:
- Remove/update tests that reference deleted pump helpers.
- Run: `cargo test -p agent_api --features codex`.
