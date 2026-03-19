### S1 — Safe-by-default advertising + R0 fail-closed gating

- **User/system value**: ensures externally sandboxed execution remains an explicit, dangerous
  opt-in (no accidental default advertising), and preserves R0 fail-closed behavior (unsupported
  keys are rejected before any value/contradiction validation).
- **Scope (in/out)**:
  - In:
    - Capabilities/advertising tests for Codex + Claude Code:
      - default backend configs MUST NOT advertise `agent_api.exec.external_sandbox.v1` (ES-C03),
      - opt-in backend configs MUST advertise the key when `allow_external_sandbox_exec=true`
        (ES-C03).
    - Harness-level R0 gating regression tests:
      - unsupported extension keys (including `agent_api.exec.external_sandbox.v1` when not
        advertised) fail as `UnsupportedCapability` without invoking value parsing/contradiction
        validation, and without leaking extension values.
  - Out:
    - Mapping behavior and warning emission when the key is accepted (covered in `S3`).
- **Acceptance criteria**:
  - Default Codex + Claude capabilities exclude `agent_api.exec.external_sandbox.v1`.
  - Opt-in Codex + Claude capabilities include `agent_api.exec.external_sandbox.v1`.
  - When the key is present but unsupported, the run fails as
    `AgentWrapperError::UnsupportedCapability` with `capability="agent_api.exec.external_sandbox.v1"`
    and does not leak the provided value.
- **Dependencies**:
  - `docs/specs/universal-agent-api/extensions-spec.md` (key id; warning non-emission when
    unsupported).
  - SEAM-2 enablement (`allow_external_sandbox_exec`) for the opt-in advertising tests.
- **Verification**:
  - `cargo test -p agent_api backend_harness::normalize`
  - `cargo test -p agent_api codex`
  - `cargo test -p agent_api claude_code`
- **Rollout/safety**:
  - Pure tests; safe to land early to harden the fail-closed posture.

#### S1.T1 — Add capability advertising regression tests (default off; opt-in on)

- **Outcome**: Codex + Claude Code backend capability sets are pinned to ES-C03 (safe default
  advertising).
- **Inputs/outputs**:
  - Input: ES-C03 in `docs/specs/universal-agent-api/extensions-spec.md` + `threading.md`.
  - Output: unit test updates in:
    - `crates/agent_api/src/backends/codex/tests.rs`
    - `crates/agent_api/src/backends/claude_code/tests.rs`
    that assert:
    - default config: capabilities DO NOT contain `agent_api.exec.external_sandbox.v1`
    - opt-in config (`allow_external_sandbox_exec=true`): capabilities DO contain the key
- **Implementation notes**:
  - Prefer adding a dedicated opt-in test rather than expanding the existing “required
    capabilities” test too far; keep assertions minimal and readable.
- **Acceptance criteria**:
  - Tests fail loudly if the dangerous key is advertised by default.
- **Test notes**:
  - Run: `cargo test -p agent_api codex`; `cargo test -p agent_api claude_code`.
- **Risk/rollback notes**:
  - None (tests only).

Checklist:
- Implement: add default-off + opt-in-on capability assertions for both backends.
- Test: `cargo test -p agent_api codex`; `cargo test -p agent_api claude_code`.
- Validate: confirm key id matches `extensions-spec.md` exactly.
- Cleanup: none.

#### S1.T2 — Add harness-level R0 gating regression test for external sandbox key (no value parsing)

- **Outcome**: the harness normalize path is pinned to reject unsupported keys before calling
  `validate_and_extract_policy`, preventing accidental value parsing or secret leakage.
- **Inputs/outputs**:
  - Input: harness contract (R0) and `docs/specs/universal-agent-api/extensions-spec.md` key id.
  - Output: new test(s) in `crates/agent_api/src/backend_harness/normalize/tests.rs` that:
    - create an adapter that would panic if `validate_and_extract_policy` is called,
    - submit a request with `extensions["agent_api.exec.external_sandbox.v1"]=<secret string>`,
    - assert `AgentWrapperError::UnsupportedCapability { capability: "agent_api.exec.external_sandbox.v1" }`,
    - assert the error string does not contain the secret.
- **Implementation notes**:
  - Follow the existing “PanicOnPolicyAdapter” pattern to pin ordering.
  - Use a distinct secret sentinel string so the leak check is unambiguous.
- **Acceptance criteria**:
  - The test fails if value validation is attempted for unsupported keys.
- **Test notes**:
  - Run: `cargo test -p agent_api backend_harness::normalize`.
- **Risk/rollback notes**:
  - None (tests only).

Checklist:
- Implement: add R0 ordering + redaction test case.
- Test: `cargo test -p agent_api backend_harness::normalize`.
- Validate: confirm the key is rejected as unsupported (not InvalidRequest).
- Cleanup: none.

