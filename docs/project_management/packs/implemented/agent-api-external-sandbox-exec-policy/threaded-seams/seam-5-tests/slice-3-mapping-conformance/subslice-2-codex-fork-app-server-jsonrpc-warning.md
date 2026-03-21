### S3b — Codex fork/app-server JSON-RPC mapping + warning ordering

- **User/system value**: pins Codex fork/app-server JSON-RPC params for external sandbox mode so
  externally sandboxed hosts get deterministic `approvalPolicy`/`sandbox` behavior, and ensures
  the backend fails closed if the app-server rejects those pinned values, while ensuring the warning
  is emitted before other user-visible events.
- **Scope (in/out)**:
  - In:
    - Codex `thread/fork` + `turn/start` param assertions when
      `agent_api.exec.external_sandbox.v1=true` is accepted.
    - JSON-RPC param exactness per `docs/specs/codex-external-sandbox-mapping-contract.md`
      (ES-C04).
    - rejected-mapping tests for the pinned "no fallback" branch when the app-server returns
      JSON-RPC errors for the external-sandbox `approvalPolicy` / `sandbox` values.
    - warning `Status` event message + ordering per
      `docs/specs/universal-agent-api/extensions-spec.md`.
  - Out:
    - Codex exec/resume argv mapping (covered by `S3a`).
    - Claude argv + allow-flag preflight (covered by `S3c`).
- **Acceptance criteria**:
  - `thread/fork` params include `approvalPolicy="never"` and `sandbox="danger-full-access"`.
  - `turn/start` params include `approvalPolicy="never"` and do not include a `sandbox` field.
  - When the fake app-server rejects the pinned `approvalPolicy` / `sandbox` values, the wrapper
    returns `AgentWrapperError::Backend { .. }` with a safe/redacted message.
  - The rejected-mapping path performs no fallback retry with alternate params.
  - If the events stream remains open, the rejected-mapping path emits exactly one terminal
    `AgentWrapperEventKind::Error` event with the same safe/redacted message before close.
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
  - Tests + fake app-server only.

#### S3.T2 — Codex fork/app-server mapping tests (JSON-RPC params + rejected mapping + warning ordering)

- **Outcome**: Codex fork/app-server external sandbox mode is pinned to ES-C04 with deterministic
  JSON-RPC params, a fail-closed rejected-mapping branch, and pinned warning ordering.
- **Files**:
  - `crates/agent_api/tests/session_fork_v1_codex.rs` (+ new module under
    `crates/agent_api/tests/session_fork_v1_codex/`)
  - `crates/agent_api/src/bin/fake_codex_app_server_jsonrpc_agent_api.rs` (env-gated JSON-RPC
    assertions)

Checklist:
- Implement:
  - Add a fork(id) (and optionally fork(last)) integration test case for external sandbox mode.
  - Assert `thread/fork` includes `approvalPolicy="never"` + `sandbox="danger-full-access"`.
  - Assert `turn/start` includes `approvalPolicy="never"` and does not include a `sandbox` field.
  - Add a rejection-path integration test where the fake app-server returns JSON-RPC errors for the
    pinned external-sandbox params and assert `AgentWrapperError::Backend { .. }`, redaction, and
    no fallback retry.
  - Assert the rejection path emits exactly one terminal `AgentWrapperEventKind::Error` event when
    the stream remains open.
  - Assert the warning `Status` event is emitted exactly once, with pinned message + ordering.
  - Extend the fake app-server binary to assert `sandbox` on `thread/fork` and to assert the
    absence of `sandbox` on `turn/start`, plus a scenario/ENV-gated rejection mode for the pinned
    params.
- Test:
  - `cargo test -p agent_api codex`
- Validate:
  - Keep fake app-server assertions scenario/ENV-gated so existing fork tests remain stable.
  - Pin `turn/start` "no sandbox field" behavior to the mapping contract.
  - Use a secret sentinel in the fake JSON-RPC error payload and assert the surfaced backend/error
    event message is redacted.
