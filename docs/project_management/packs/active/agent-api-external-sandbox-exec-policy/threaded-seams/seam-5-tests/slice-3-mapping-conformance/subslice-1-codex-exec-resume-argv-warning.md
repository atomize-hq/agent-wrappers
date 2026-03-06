### S3a — Codex exec/resume argv mapping + warning ordering

- **User/system value**: locks in deterministic Codex external-sandbox argv mapping so externally
  sandboxed hosts can rely on stable behavior, proves the backend fails closed if Codex rejects the
  dangerous override, and ensures the required dangerous-mode warning is emitted before other
  user-visible events.
- **Scope (in/out)**:
  - In:
    - Codex exec + resume mapping tests when `agent_api.exec.external_sandbox.v1=true` is accepted.
    - argv exactness per `docs/specs/codex-external-sandbox-mapping-contract.md` (ES-C04).
    - rejected-override tests for the pinned "no fallback" branch when the installed Codex binary
      refuses `--dangerously-bypass-approvals-and-sandbox`.
    - warning `Status` event message + ordering per
      `docs/specs/universal-agent-api/extensions-spec.md`.
  - Out:
    - Codex fork/app-server JSON-RPC params (covered by `S3b`).
    - Claude argv + allow-flag preflight (covered by `S3c`).
- **Acceptance criteria**:
  - Exec/resume argv contains `--dangerously-bypass-approvals-and-sandbox` exactly once.
  - Exec/resume argv contains none of: `--full-auto`, `--ask-for-approval`, `--sandbox`.
  - When the fake Codex binary rejects `--dangerously-bypass-approvals-and-sandbox`, the wrapper
    returns `AgentWrapperError::Backend { .. }` with a safe/redacted message.
  - The rejected-override path performs no fallback retry with alternate flags and no second spawn.
  - Exactly one warning `Status` event with the pinned message is emitted when
    `external_sandbox=true` is accepted.
  - Warning is emitted before the session handle facet `Status` event.
- **Dependencies**:
  - `docs/specs/codex-external-sandbox-mapping-contract.md` (ES-C04).
  - `docs/specs/universal-agent-api/extensions-spec.md` (warning contract).
  - SEAM-2 opt-in gate (`allow_external_sandbox_exec`) + SEAM-3 Codex mapping implementation.
- **Verification**:
  - `cargo test -p agent_api codex`
- **Rollout/safety**:
  - Tests + fake binary only.

#### S3.T1 — Codex exec/resume mapping tests (argv exactness + rejected override + warning ordering)

- **Outcome**: Codex exec/resume external sandbox mode is pinned to ES-C04 with deterministic argv
  shape, a fail-closed rejected-override branch, and pinned warning ordering.
- **Files**:
  - `crates/agent_api/tests/c1_codex_exec_policy.rs` (exec path assertions)
  - `crates/agent_api/tests/c2_codex_session_resume_v1.rs` (resume path assertions)
  - `crates/agent_api/src/bin/fake_codex_stream_exec_scenarios_agent_api.rs` (env-gated argv
    assertions)

Checklist:
- Implement:
  - Add exec + resume integration tests asserting required/forbidden argv flags.
  - Add a rejection-path integration test where the fake Codex binary rejects the dangerous flag
    and assert `AgentWrapperError::Backend { .. }`, redaction, and single-attempt behavior.
  - Assert the warning `Status` event is emitted exactly once, with pinned message + ordering.
  - Extend the fake Codex binary to assert dangerous bypass flag presence and absence of
    `--full-auto`, `--ask-for-approval`, `--sandbox`, and to expose a scenario/ENV-gated rejection
    mode that proves no fallback retry occurs.
- Test:
  - `cargo test -p agent_api codex`
- Validate:
  - Pin warning text + ordering to `extensions-spec.md` (do not over-pin unrelated messages).
  - Use a secret sentinel in the fake rejection output and assert the surfaced backend message is
    redacted.
  - Keep fake-binary assertions scenario/ENV-gated so existing scenarios remain stable.
