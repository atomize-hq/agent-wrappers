### S3 — Pinned mapping conformance (Codex + Claude) + warning ordering

- **User/system value**: locks in deterministic, backend-owned external sandbox mapping so
  externally sandboxed hosts can rely on stable behavior across Codex + Claude Code, and ensures
  the required dangerous-mode warning is always visible before any other user-visible events.
- **Scope (in/out)**:
  - In:
    - Warning event conformance (pinned by `extensions-spec.md`):
      - exactly one warning `Status` event with the pinned message when `external_sandbox=true` is
        accepted, and
      - warning ordering before any other user-visible events and before the session handle facet
        `Status` event.
    - Codex mapping conformance (ES-C04; pinned):
      - exec/resume argv includes `--dangerously-bypass-approvals-and-sandbox` exactly once and
        excludes `--full-auto`, `--ask-for-approval`, `--sandbox`.
      - fork/app-server JSON-RPC params:
        - `thread/fork`: `approvalPolicy="never"` + `sandbox="danger-full-access"`
        - `turn/start`: `approvalPolicy="never"` (no `sandbox` param)
    - Claude Code mapping conformance (ES-C05/ES-C07; pinned):
      - argv includes `--dangerously-skip-permissions`,
      - argv includes/excludes `--allow-dangerously-skip-permissions` based on deterministic cached
        `claude --help` preflight output, and
      - preflight failure returns `AgentWrapperError::Backend { .. }` before spawning `--print`.
  - Out:
    - Live-binary e2e coverage (explicitly out-of-scope for v1 acceptance; see `seam-5-tests.md`).
- **Acceptance criteria**:
  - Codex exec/resume mapping matches `docs/specs/codex-external-sandbox-mapping-contract.md`
    exactly.
  - Codex fork/app-server params match `docs/specs/codex-external-sandbox-mapping-contract.md`
    exactly.
  - Claude argv + allow-flag behavior matches `docs/specs/claude-code-session-mapping-contract.md`
    exactly.
  - Warning event message and ordering match `docs/specs/universal-agent-api/extensions-spec.md`
    exactly.
- **Dependencies**:
  - `docs/specs/universal-agent-api/extensions-spec.md` (warning contract).
  - `docs/specs/codex-external-sandbox-mapping-contract.md` (Codex mapping).
  - `docs/specs/claude-code-session-mapping-contract.md` (Claude mapping + preflight).
  - SEAM-2 opt-in gate (key must be supported to reach mapping).
  - SEAM-3 + SEAM-4 implementations (mapping behavior must exist before tests can land).
- **Verification**:
  - `cargo test -p agent_api codex`
  - `cargo test -p agent_api claude_code`
- **Rollout/safety**:
  - Pure tests + fake binaries; safe once mapping implementations exist.

#### S3.T1 — Codex exec/resume mapping tests (argv exactness + warning ordering)

- **Outcome**: Codex exec/resume external sandbox mode is pinned to ES-C04 with deterministic argv
  shape and warning ordering.
- **Inputs/outputs**:
  - Input: `docs/specs/codex-external-sandbox-mapping-contract.md` + `extensions-spec.md` warning
    contract.
  - Output:
    - integration tests under `crates/agent_api/tests/` that run Codex with a fake binary and assert:
      - argv contains `--dangerously-bypass-approvals-and-sandbox` exactly once,
      - argv contains none of: `--full-auto`, `--ask-for-approval`, `--sandbox`,
      - warning `Status` event is emitted exactly once with the pinned message, and
      - warning appears before the session handle facet `Status` event.
    - extensions to `crates/agent_api/src/bin/fake_codex_stream_exec_scenarios_agent_api.rs` so it
      can assert the pinned flag presence/absence deterministically (env-controlled).
- **Implementation notes**:
  - Cover both exec and resume flows (at least one test each) to ensure the mapping is applied
    across entrypoints.
  - Use the existing fake Codex binary env-var assertion pattern; avoid introducing new ad-hoc
    golden outputs.
- **Acceptance criteria**:
  - Tests fail loudly on any argv drift or warning ordering drift.
- **Test notes**:
  - Run: `cargo test -p agent_api codex`.
- **Risk/rollback notes**:
  - None (tests + fake binary only).

Checklist:
- Implement: add exec + resume external-sandbox mapping tests for Codex.
- Implement: extend fake Codex binary to assert dangerous bypass flag and absence of other flags.
- Test: `cargo test -p agent_api codex`.
- Validate: ensure warnings are pinned to `extensions-spec.md` exact text + ordering.
- Cleanup: none.

#### S3.T2 — Codex fork/app-server mapping tests (JSON-RPC params + warning ordering)

- **Outcome**: Codex fork/app-server external sandbox mode is pinned to ES-C04 with deterministic
  JSON-RPC params and warning ordering.
- **Inputs/outputs**:
  - Input: `docs/specs/codex-external-sandbox-mapping-contract.md` + `extensions-spec.md` warning
    contract.
  - Output:
    - integration tests (recommended alongside existing fork tests) that run fork flows against the
      fake app-server and assert:
      - `thread/fork` params include `approvalPolicy="never"` and `sandbox="danger-full-access"`,
      - `turn/start` params include `approvalPolicy="never"` and do not include a `sandbox` field,
      - warning `Status` event is emitted exactly once with the pinned message, and
      - warning appears before the session handle facet `Status` event.
    - extensions to `crates/agent_api/src/bin/fake_codex_app_server_jsonrpc_agent_api.rs` so it can
      assert `sandbox` when requested (env-controlled), and optionally assert `turn/start` lacks a
      sandbox param.
- **Implementation notes**:
  - Prefer adding a dedicated fork(id) test case for external sandbox mode; fork(last) can be
    covered if it’s low-effort.
  - Keep fake app-server assertions scenario/ENV-gated so existing fork tests remain stable.
- **Acceptance criteria**:
  - Tests fail loudly on any `approvalPolicy` / `sandbox` param drift.
- **Test notes**:
  - Run: `cargo test -p agent_api codex`.
- **Risk/rollback notes**:
  - None (tests + fake binary only).

Checklist:
- Implement: add fork-flow external-sandbox mapping tests for Codex app-server.
- Implement: extend fake app-server to assert `sandbox="danger-full-access"` on `thread/fork`.
- Test: `cargo test -p agent_api codex`.
- Validate: ensure `turn/start` does not send a sandbox param (pinned by spec).
- Cleanup: none.

#### S3.T3 — Claude mapping tests (argv + allow-flag preflight + warning ordering)

- **Outcome**: Claude external sandbox mode is pinned to ES-C05/ES-C07 with deterministic argv,
  deterministic allow-flag behavior, and pinned warning ordering.
- **Inputs/outputs**:
  - Input:
    - `docs/specs/claude-code-session-mapping-contract.md` (danger flags + allow-flag preflight),
    - `docs/specs/universal-agent-api/extensions-spec.md` (warning contract).
  - Output:
    - integration tests under `crates/agent_api/tests/` that assert:
      - `--dangerously-skip-permissions` is present when `external_sandbox=true`,
      - allow-flag supported → argv includes `--allow-dangerously-skip-permissions`,
      - allow-flag not supported → argv excludes `--allow-dangerously-skip-permissions`,
      - help-preflight failure returns `AgentWrapperError::Backend { .. }` before spawning `--print`,
      - warning `Status` event is emitted exactly once with the pinned message, and
      - warning appears before the session handle facet `Status` event.
    - extensions to `crates/agent_api/src/bin/fake_claude_stream_json_agent_api.rs` so it can:
      - respond to `claude --help` with controlled stdout (include/exclude allow flag token),
      - fail the `--help` preflight deterministically for the failure-path test, and
      - assert the printed argv contains/excludes the expected flags.
- **Implementation notes**:
  - Use the fake Claude binary to simulate both `--help` and `--print` invocations under a single
    `binary` path.
  - In the preflight-failure test, include a secret sentinel in fake output and assert the wrapper
    error message is redacted (does not contain the secret).
- **Acceptance criteria**:
  - Tests fail loudly on any argv drift, preflight drift, or warning ordering drift.
- **Test notes**:
  - Run: `cargo test -p agent_api claude_code`.
- **Risk/rollback notes**:
  - None (tests + fake binary only).

Checklist:
- Implement: add Claude external-sandbox mapping tests (allow-flag supported/unsupported + failure).
- Implement: extend fake Claude binary to support `--help` preflight and to assert dangerous flags.
- Test: `cargo test -p agent_api claude_code`.
- Validate: pin warning event message + ordering to `extensions-spec.md`.
- Cleanup: none.

