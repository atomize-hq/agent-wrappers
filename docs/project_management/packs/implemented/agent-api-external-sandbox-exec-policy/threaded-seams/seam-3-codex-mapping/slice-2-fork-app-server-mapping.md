### S2 — Fork mapping (Codex app-server JSON-RPC) (pinned)

- **User/system value**: enables external sandbox mode for Codex fork flows by sending pinned
  `approvalPolicy`/`sandbox` params to the app-server, while remaining deterministic and non-interactive.
- **Scope (in/out)**:
  - In:
    - When `external_sandbox=true` is accepted, set (pinned):
      - `thread/fork`: `approvalPolicy="never"`, `sandbox="danger-full-access"`
      - `turn/start`: `approvalPolicy="never"` (no `sandbox` param)
    - Ensure the mapping is deterministic (no spawn+retry loops).
  - Out:
    - Exec/resume mapping (S1).
    - Capability advertising / opt-in gating (SEAM-2).
    - Regression tests (SEAM-5).
- **Acceptance criteria**:
  - When `external_sandbox=true` is accepted, `thread/fork` always sends `approvalPolicy="never"`
    and `sandbox="danger-full-access"` (regardless of defaults).
  - When `external_sandbox=true` is accepted, `turn/start` always sends `approvalPolicy="never"` and
    does not attempt to send a `sandbox` field (per the pinned v2 subset contract).
  - No fallback mapping is attempted if the app-server rejects the pinned values; the run fails as
    `AgentWrapperError::Backend { message }` with a safe/redacted `message`.
- **Dependencies**:
  - `S1.T1` provides `policy.external_sandbox` extraction and contradiction validation.
  - `S1.T2` provides the pinned warning event emission for all flows (including fork).
  - Canonical mapping contract: `docs/specs/codex-external-sandbox-mapping-contract.md`.
- **Verification**:
  - Compile + existing tests: `cargo test -p agent_api codex`
  - SEAM-5 pins the exact JSON-RPC params in unit tests.
- **Rollout/safety**:
  - Reachable only behind host opt-in (SEAM-2) and explicit per-run request.
  - Deterministic and non-interactive: approval prompts should not occur under pinned `never` policy.

#### S2.T1 — Override fork JSON-RPC params when `external_sandbox=true`

- **Outcome**: Codex fork flows send the pinned `approvalPolicy` and `sandbox` values required by
  ES-C04, matching `docs/specs/codex-external-sandbox-mapping-contract.md`.
- **Inputs/outputs**:
  - Input: the fork mapping section in `docs/specs/codex-external-sandbox-mapping-contract.md`.
  - Output: code changes in:
    - `crates/agent_api/src/backends/codex.rs` (thread policy into fork flow request),
    - `crates/agent_api/src/backends/codex/fork.rs` (override `approval_policy`/`sandbox` when enabled).
- **Implementation notes**:
  - Add `external_sandbox: bool` to `fork::ForkFlowRequest`.
  - In `spawn_fork_v1_flow(...)`, when `external_sandbox == true`:
    - set `approval_policy = Some("never".to_string())`,
    - set `sandbox = Some("danger-full-access".to_string())`,
    - leave `turn_start_v2` without a `sandbox` field (already true in current struct).
  - Confirm that `thread/list` (used by the `"last"` selector) remains a selection-only surface
    and is not passed exec-policy override fields.
- **Acceptance criteria**:
  - Meets the slice acceptance criteria for fork mapping.
- **Test notes**:
  - SEAM-5 should assert the exact `thread/fork` and `turn/start` params for `external_sandbox=true`.
- **Risk/rollback notes**:
  - Medium risk: app-server contract surface; mitigate by pinned mapping + SEAM-5 tests.

Checklist:
- Implement: add `external_sandbox` to `ForkFlowRequest` and override params in `crates/agent_api/src/backends/codex/fork.rs`.
- Test: `cargo test -p agent_api codex`.
- Validate: `rg -n "danger-full-access|approval_policy" crates/agent_api/src/backends/codex/fork.rs`.
- Cleanup: avoid adding retry logic; ensure errors remain safe/redacted on failure paths.
