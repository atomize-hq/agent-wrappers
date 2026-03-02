# S2 — Implement `BH-C03` env merge precedence + timeout derivation helpers

- **User/system value**: Ensures deterministic, consistent env and timeout semantics across backends by construction (no per-backend copies), reducing drift and onboarding cost.
- **Scope (in/out)**:
  - In:
    - Implement `BH-C03 env merge + timeout derivation`: deterministic env precedence + timeout selection.
    - Implement timeout derivation helpers (request timeout overrides backend default; preserve “absent” semantics consistently).
    - Integrate env + timeout derivation into the harness-owned normalized request shape.
  - Out:
    - Any backend-specific default configuration rules beyond “defaults overridden by request”.
    - Stream pump or completion gating behavior (SEAM-3/SEAM-4).
    - Backend adoption/migration (SEAM-5).
- **Acceptance criteria**:
  - The harness produces a single merged env map for spawn with deterministic precedence: backend defaults overridden by request env.
  - Timeout value used by the harness is derived deterministically:
    - request-provided timeout overrides backend default,
    - “no timeout” behavior is consistent and explicit.
  - `BH-C03` behavior is covered by focused unit tests (Slice S3).
- **Dependencies**:
  - Contract from SEAM-1: `BH-C01 backend harness adapter interface` (normalization lifecycle + spawn inputs).
  - Existing backend config model types (env defaults + default timeout fields).
- **Verification**:
  - `cargo test -p agent_api --features codex`
  - `cargo test -p agent_api --features claude_code`
  - `cargo test -p agent_api --features codex,claude_code`

## Atomic Tasks

#### S2.T1 — Add a deterministic env merge helper (`BH-C03`)

- **Outcome**: A harness-owned helper that merges backend-default env and request env with deterministic precedence.
- **Inputs/outputs**:
  - Input: backend config env (defaults) + `AgentWrapperRunRequest.env`
  - Output: merged env map used for spawn (internal)
  - Output: `crates/agent_api/src/backend_harness.rs`
- **Implementation notes**:
  - Precedence is strict: backend config env < request env.
  - Prefer deterministic map construction if errors must name keys (avoid nondeterministic key selection).
  - Do not mutate inputs; return a fresh merged map.
- **Acceptance criteria**:
  - The merged env contains all backend-default keys not overridden by the request.
  - Any key present in the request overwrites the backend default value.
  - Behavior matches the `BH-C03` definition in `threading.md`.
- **Test notes**: unit-tested in Slice S3 with a 2-key overlap case + empty cases.
- **Risk/rollback notes**: internal-only; safe to tweak representation for determinism.

Checklist:
- Implement: `merge_env_backend_defaults_then_request(...)`.
- Test: overlap + non-overlap precedence.
- Validate: clippy-clean and feature-flag build matrix.
- Cleanup: keep helper private; expose only via the normalized request struct.

#### S2.T2 — Add timeout derivation helper (request overrides backend default)

- **Outcome**: A harness-owned helper that deterministically decides which timeout applies for the run.
- **Inputs/outputs**:
  - Input: backend default timeout + request timeout override (if present)
  - Output: derived timeout representation consumed by the harness run loop
  - Output: `crates/agent_api/src/backend_harness.rs`
- **Implementation notes**:
  - Internal timeout representation (exact):
    - `NormalizedRequest.effective_timeout: Option<std::time::Duration>`
    - `None` means “absent” (no request override and no backend default).
    - `Some(Duration::ZERO)` is an explicit “no timeout” request override.
  - The helper MUST be pure (no tokio timeouts here); enforcement is backend-owned:
    - The harness passes `effective_timeout` into the adapter’s `spawn(...)` via `NormalizedRequest`.
    - The adapter maps that timeout into its wrapper runtime:
      - Codex wrapper (`codex::CodexClient`): map with `effective_timeout.unwrap_or(Duration::ZERO)`
        because `Duration::ZERO` means “no timeout” in `crates/codex/src/client_core.rs`.
      - Claude Code wrapper (`claude_code::ClaudeClient`): pass the `Option<Duration>` directly.
        - If any adapter layer uses `tokio::time::timeout(...)` (current behavior in
          `crates/agent_api/src/backends/claude_code.rs`), it MUST treat
          `Some(Duration::ZERO)` as “disable timeout” (i.e., do not call
          `tokio::time::timeout(Duration::ZERO, ...)` which would fail immediately).
  - Operations subject to timeout (v1):
    - The harness does not add a second “overall run” timeout.
    - Timeouts are enforced by the wrapper runtime (spawned CLI wait / stream/completion as implemented by that wrapper).
  - Timeout failure behavior (v1):
    - Any wrapper timeout MUST surface as `BackendError` at either the stream or completion boundary,
      and MUST be mapped via `BackendHarnessAdapter::redact_error(...)` to:
      - a safe `AgentWrapperEventKind::Error` event message, and
      - `AgentWrapperError::Backend { message }` as the completion outcome.
- **Acceptance criteria**:
  - Request timeout takes precedence when present.
  - Otherwise, the backend default timeout is used.
  - “Absent” is preserved consistently (no accidental defaulting).
- **Test notes**: unit-tested in Slice S3 (request present vs absent; backend default present vs absent).
- **Risk/rollback notes**: avoid changing observable behavior; if current backends disagree, pin desired behavior in tests before migration (SEAM-5).

Checklist:
- Implement: `derive_effective_timeout(request_timeout, backend_default_timeout)` helper.
- Test: all four presence/absence combinations.
- Test (timeout enforcement shape): port the existing Codex regression `codex_backend_enforces_timeout_while_draining_events`
  from `crates/agent_api/src/backends/codex/tests.rs` to a harness-level test once the pump is harness-owned (SEAM-3),
  asserting that at least one event can be forwarded before a timeout completion error is observed.
- Validate: no behavior changes intended; keep helper internal.
- Cleanup: document semantics next to the helper to prevent drift.

#### S2.T3 — Integrate env + timeout derivation into the normalized request struct

- **Outcome**: A single internal normalized request struct that downstream harness steps consume, rather than re-reading raw request/config.
- **Inputs/outputs**:
  - Output: normalized request struct in `crates/agent_api/src/backend_harness.rs`
  - Output: harness normalization function populating env + timeout
- **Implementation notes**:
  - Include only validated/derived fields needed for spawn and run-loop behavior.
  - Keep raw request/config accessible only for logging/debug at bounded/redacted boundaries.
- **Acceptance criteria**:
  - Spawn inputs (env + timeout) are derived exactly once.
  - Downstream seams (SEAM-3/SEAM-4) can consume derived values without introducing alternate semantics.
- **Test notes**: exercised by Slice S3 tests that call the normalization entrypoint and inspect the derived fields.
- **Risk/rollback notes**: internal-only; can iterate structure without public API impact.

Checklist:
- Implement: `NormalizedRequest` (or equivalent) including merged env + derived timeout.
- Test: normalization returns derived fields deterministically.
- Validate: compile under both backend feature flags.
- Cleanup: keep struct non-`pub`; scope it to the harness module.
