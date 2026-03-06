### S2 — Backend validation + contradiction rules (fail before spawn)

- **User/system value**: pins the dangerous key’s validation and contradiction rules so invalid or
  ambiguous requests are rejected deterministically before any backend process is spawned.
- **Scope (in/out)**:
  - In:
    - Type validation: `agent_api.exec.external_sandbox.v1` MUST be boolean (ES-C01).
    - Contradiction validation: `external_sandbox=true` MUST NOT be combined with an explicit
      `agent_api.exec.non_interactive=false` request (ES-C02).
    - Exec-policy combination rule (when applicable): `external_sandbox=true` MUST reject any
      `backend.<agent_kind>.exec.*` keys that are supported per R0 (ES-C06).
    - Proof of “no spawn when invalid/contradictory”: use an intentionally-invalid binary path and
      assert validation errors are returned (not spawn failures).
  - Out:
    - The backend-owned mapping behavior when the key is accepted (pinned in `S3`).
- **Acceptance criteria**:
  - With opt-in enabled (key supported), a non-boolean value fails before spawn as
    `AgentWrapperError::InvalidRequest`.
  - With opt-in enabled (key supported), `external_sandbox=true` + `agent_api.exec.non_interactive=false`
    fails before spawn as `AgentWrapperError::InvalidRequest` (ES-C02).
  - With opt-in enabled (key supported), `external_sandbox=true` + any supported
    `backend.<agent_kind>.exec.*` key present fails before spawn as `AgentWrapperError::InvalidRequest`
    (ES-C06).
  - Validation failures occur before spawn (validated by using a nonexistent backend binary path and
    observing `InvalidRequest`, not `Backend` spawn failures).
- **Dependencies**:
  - Key schema + contradiction rules: `docs/specs/universal-agent-api/extensions-spec.md` (ES-C01/02/06).
  - Opt-in enablement: SEAM-2 (`allow_external_sandbox_exec`) so the key is supported and validation
    is reachable.
- **Verification**:
  - `cargo test -p agent_api codex`
  - `cargo test -p agent_api claude_code`
- **Rollout/safety**:
  - Pure tests; safe to land once SEAM-2 exposes the opt-in gate.

#### S2.T1 — Codex validation tests: type check + ES-C02 contradiction + ES-C06 combination rule

- **Outcome**: Codex backend validation is pinned to ES-C01/ES-C02/ES-C06 and proven to fail before
  spawn.
- **Inputs/outputs**:
  - Input: `docs/specs/universal-agent-api/extensions-spec.md` (validation rules) + `threading.md`.
  - Output: new integration tests (recommended) under `crates/agent_api/tests/` that exercise
    `CodexBackend::run(...)` and assert:
    - wrong type for `agent_api.exec.external_sandbox.v1` → `InvalidRequest` (pre-spawn),
    - `external_sandbox=true` + `agent_api.exec.non_interactive=false` → `InvalidRequest` (ES-C02),
    - `external_sandbox=true` + `backend.codex.exec.sandbox_mode=<any>` → `InvalidRequest` (ES-C06),
    - and all of the above still return `InvalidRequest` even when `CodexBackendConfig.binary` is
      set to a nonexistent path (proves no spawn).
- **Implementation notes**:
  - Ensure tests explicitly enable the capability gate (`allow_external_sandbox_exec=true`) so R0
    allows the key and validation is reachable.
  - Avoid over-pinning error messages unless a spec makes the message text Normative; pin variants
    + key identifiers.
- **Acceptance criteria**:
  - Tests prove Codex fails closed before spawn on invalid/contradictory requests.
- **Test notes**:
  - Run: `cargo test -p agent_api codex`.
- **Risk/rollback notes**:
  - None (tests only).

Checklist:
- Implement: add Codex validation tests covering ES-C01/02/06.
- Test: `cargo test -p agent_api codex`.
- Validate: ensure the test binary path is nonexistent to prove “no spawn”.
- Cleanup: none.

#### S2.T2 — Claude validation tests: type check + ES-C02 contradiction (fail before spawn)

- **Outcome**: Claude Code backend validation is pinned to ES-C01/ES-C02 and proven to fail before
  spawning `claude --print`.
- **Inputs/outputs**:
  - Input: `docs/specs/universal-agent-api/extensions-spec.md` (validation rules) + `threading.md`.
  - Output: new integration tests (recommended) under `crates/agent_api/tests/` that exercise
    `ClaudeCodeBackend::run(...)` and assert:
    - wrong type for `agent_api.exec.external_sandbox.v1` → `InvalidRequest` (pre-spawn),
    - `external_sandbox=true` + `agent_api.exec.non_interactive=false` → `InvalidRequest` (ES-C02),
    - and both still return `InvalidRequest` even when `ClaudeCodeBackendConfig.binary` is set to a
      nonexistent path (proves no spawn).
- **Implementation notes**:
  - Ensure tests explicitly enable the capability gate (`allow_external_sandbox_exec=true`) so R0
    allows the key and validation is reachable.
  - Note: the repo currently has no supported `backend.claude_code.exec.*` keys, so ES-C06 is
    effectively enforced by R0 (unsupported keys fail closed) until such keys are introduced.
- **Acceptance criteria**:
  - Tests prove Claude fails closed before spawn on invalid/contradictory requests.
- **Test notes**:
  - Run: `cargo test -p agent_api claude_code`.
- **Risk/rollback notes**:
  - None (tests only).

Checklist:
- Implement: add Claude validation tests covering ES-C01/02.
- Test: `cargo test -p agent_api claude_code`.
- Validate: ensure the test binary path is nonexistent to prove “no spawn”.
- Cleanup: none.

