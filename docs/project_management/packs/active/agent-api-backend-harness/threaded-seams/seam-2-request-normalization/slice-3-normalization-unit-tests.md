# S3 — Harness-layer unit tests for request normalization invariants

- **User/system value**: Prevents behavior drift by pinning fail-closed validation and deterministic env/timeout semantics in harness-owned tests (not per-backend copies).
- **Scope (in/out)**:
  - In:
    - Add focused unit tests for:
      - unknown extension key rejection (`BH-C02`)
      - env merge precedence (`BH-C03`)
      - timeout derivation semantics (request vs backend defaults)
      - universal invalid request checks (e.g., empty prompt)
    - Ensure errors are stable and do not leak raw backend output.
  - Out:
    - Integration tests that exercise real backends end-to-end (SEAM-5).
    - Stream pump/drain-on-drop tests (SEAM-3) and completion gating tests (SEAM-4).
- **Acceptance criteria**:
  - Tests live at the harness layer (co-located with `backend_harness.rs` or a sibling internal test module).
  - Tests cover the invariants listed in SEAM-2 “Verification” and use deterministic fixtures.
  - Failures are readable and stable (no dependence on hash iteration order).
- **Dependencies**:
  - Slice S1: `BH-C02` allowlist validator.
  - Slice S2: `BH-C03` env merge + timeout derivation helpers.
  - Upstream contract: `BH-C01 backend harness adapter interface` (SEAM-1) sufficient to create a toy adapter or harness entrypoint for tests.
- **Verification**:
  - `cargo test -p agent_api --features codex`
  - `cargo test -p agent_api --features claude_code`
  - `cargo test -p agent_api --features codex,claude_code`

## Atomic Tasks

#### S3.T1 — Test `BH-C02` fail-closed unknown extension keys

- **Outcome**: A unit test that proves the harness rejects an unknown extension key pre-spawn with `UnsupportedCapability(agent_kind, key)`.
- **Inputs/outputs**:
  - Output: tests in `crates/agent_api/src/backend_harness.rs` (or a dedicated internal test module)
- **Implementation notes**:
  - Use a toy adapter with a tiny allowlist (`{"known_key"}`) and a request that includes `{"unknown_key": ...}`.
  - Assert error type and relevant fields; avoid asserting full error strings if formatting might evolve.
- **Acceptance criteria**:
  - The harness returns `UnsupportedCapability` without invoking spawn.
  - The reported `agent_kind` matches the adapter’s `agent_kind`.
- **Test notes**: include a positive control where all keys are allowed and normalization proceeds.
- **Risk/rollback notes**: none (tests only).

Checklist:
- Implement: toy adapter fixture + unknown-key test.
- Test: run `cargo test -p agent_api --features codex,claude_code`.
- Validate: clippy-clean.
- Cleanup: keep fixture minimal (do not re-implement backend adapters).

#### S3.T2 — Test env merge precedence (`BH-C03`) and timeout derivation

- **Outcome**: Unit tests pinning env merge precedence and timeout selection semantics.
- **Inputs/outputs**:
  - Output: tests co-located with the harness normalization helpers
- **Implementation notes**:
  - Env tests:
    - backend defaults include `A=1`, `B=1`
    - request env includes `B=2`
    - merged env should include `A=1`, `B=2`
  - Timeout tests: cover the four presence/absence combinations (request vs default).
  - `Duration::ZERO` test: if the request sets `timeout=Some(Duration::ZERO)`, normalization MUST
    preserve `effective_timeout=Some(Duration::ZERO)` and downstream enforcement MUST treat it as
    “disable timeout” (not “timeout immediately”).
- **Acceptance criteria**:
  - Env precedence matches `BH-C03`.
  - Timeout derivation follows “request overrides default” and preserves absence semantics.
- **Test notes**: keep fixtures small; prefer explicit types rather than parsing strings.
- **Risk/rollback notes**: if current backends disagree, capture desired harness semantics and treat migration differences as SEAM-5 follow-ups.

Checklist:
- Implement: env merge tests + timeout derivation tests.
- Test: run harness unit tests under both backend feature flags.
- Validate: deterministic assertions (no hash-order dependencies).
- Cleanup: keep helper APIs internal.

#### S3.T3 — Test universal invalid request checks (e.g., empty prompt)

- **Outcome**: A unit test proving that obviously invalid universal requests are rejected consistently across backends.
- **Inputs/outputs**:
  - Output: tests next to normalization entrypoint
- **Implementation notes**:
  - Add at least one “universal invalid request” check that is backend-agnostic (empty prompt is a good first check per seam brief).
  - Preserve the current built-in behavior (both backends return `InvalidRequest { message: \"prompt must not be empty\" }` today) unless a deliberate behavior change is explicitly recorded elsewhere.
  - Ensure error does not include raw prompt content.
- **Acceptance criteria**:
  - The harness rejects invalid universal requests before any spawn attempt.
  - Error is stable and uses an appropriate universal variant (e.g., `InvalidRequest`).
- **Test notes**: include one valid prompt control case.
- **Risk/rollback notes**: ensure the rule matches ADR-0013 intent; avoid accidentally adding backend-specific policy here.
- **Risk/rollback notes**: preserve current built-in backend behavior; avoid adding new universal policy in a “no behavior change” refactor.
  - Evidence (current behavior): `crates/agent_api/src/backends/codex.rs` and `crates/agent_api/src/backends/claude_code.rs` already reject `request.prompt.trim().is_empty()`.

Checklist:
- Implement: invalid prompt test(s).
- Test: run `cargo test -p agent_api` with relevant features.
- Validate: no backend-specific policy leaks into the universal checks.
- Cleanup: keep the universal check list short and explicit.
