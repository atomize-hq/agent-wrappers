# S3 — Migrate Claude backend to the harness (behavior-equivalent)

- **User/system value**: Removes duplicated glue logic from the Claude backend and proves the harness abstraction works for a second, structurally different backend (stream-json parsing + tool facets).
- **Scope (in/out)**:
  - In:
    - Refactor `crates/agent_api/src/backends/claude_code.rs` to:
      - implement the `BH-C01` adapter shape, and
      - delegate request validation (`BH-C02`), env/timeout derivation (`BH-C03`), stream pump (`BH-C04`), bounds enforcement, and completion gating (`BH-C05`) to the harness.
    - Preserve Claude-specific mapping logic:
      - stream-json parsing and mapping (`map_stream_json_event`, `map_assistant_message`, etc.),
      - extraction of final assistant text from stream-json, if the harness requires a backend-provided completion payload.
  - Out:
    - Changes to Claude capability IDs / extension keys.
    - Changes to harness-owned semantics (treat as upstream seam work).
- **Acceptance criteria**:
  - `ClaudeCodeBackend::run` constructs an `AgentWrapperRunHandle` via the harness entrypoint.
  - Claude backend code no longer re-implements harness-owned invariants (unknown extension rejection, env merge, timeout policy, pump loop, gating).
  - Existing Claude mapping tests remain valid (or are adjusted only for refactor-induced symbol moves).
- **Dependencies**:
  - Upstream seams:
    - SEAM-1 (`BH-C01`)
    - SEAM-2 (`BH-C02`, `BH-C03`)
    - SEAM-3 (`BH-C04`)
    - SEAM-4 (`BH-C05`)
- **Verification**:
  - `cargo test -p agent_api --features claude_code`
  - `cargo test -p agent_api --features codex,claude_code`

## Atomic Tasks

#### S3.T1 — Implement the Claude adapter for `BH-C01` (spawn + mapping + completion extraction)

- **Outcome**: A Claude-owned adapter implementation that provides the harness with spawn, typed event mapping, and completion extraction.
- **Inputs/outputs**:
  - Output: `crates/agent_api/src/backends/claude_code.rs`
  - Input: existing Claude mapping helpers and stream-json parsing flow.
- **Implementation notes**:
  - As with Codex, move allowlist behavior to the harness: backend-local `validate_and_extract_non_interactive` should stop scanning unknown keys (unknown-key rejection is `BH-C02`).
  - Preserve current “non_interactive implies permission mode” behavior and surface it as backend-specific spawn config derived from known extensions.
- **Acceptance criteria**:
  - Adapter declares:
    - `agent_kind == "claude_code"`,
    - allowlisted extension keys (currently `agent_api.exec.non_interactive`),
    - spawn hook returning typed stream + completion,
    - mapping hook for `ClaudeStreamJsonEvent` → `AgentWrapperEvent`.

Checklist:
- Implement adapter with allowlist + spawn + mapping.
- Preserve current Claude-specific permission mode behavior.

#### S3.T2 — Rewire `ClaudeCodeBackend::run` to call the harness entrypoint (remove local run-loop)

- **Outcome**: Claude backend’s `run` is a thin “configure adapter + call harness” wrapper.
- **Inputs/outputs**:
  - Output: `crates/agent_api/src/backends/claude_code.rs`
- **Implementation notes**:
  - Remove backend-local `mpsc` channel plumbing and drain-on-drop loop.
  - Remove backend-local `tokio::time::timeout` wrapper if the harness owns timeout derivation/enforcement (per `BH-C03` and SEAM-2 normalization semantics).
  - Remove backend-local prompt-empty validation if the harness has a universal invalid request check (SEAM-2).
- **Acceptance criteria**:
  - No drain loop exists in `claude_code.rs`.
  - No direct calls to `crate::bounds` or `crate::run_handle_gate::build_gated_run_handle` remain in the backend module.

Checklist:
- Replace run implementation with harness entrypoint call.
- Delete legacy orchestration code paths.
- Ensure `--features claude_code` still compiles cleanly.

#### S3.T3 — Adjust Claude backend tests for the new architecture (keep mapping tests)

- **Outcome**: Claude backend tests remain valuable and focused on Claude-specific mapping semantics.
- **Inputs/outputs**:
  - Output: `crates/agent_api/src/backends/claude_code/tests.rs`
- **Implementation notes**:
  - Keep mapping/capability tests as-is.
  - Add the adapter compile-time guard from Slice S2 if not already present.
- **Acceptance criteria**:
  - Tests do not attempt to spawn real Claude CLI processes.
  - Tests do not re-test harness-owned invariants.

Checklist:
- Keep mapping fixture tests intact.
- Run: `cargo test -p agent_api --features claude_code`.
