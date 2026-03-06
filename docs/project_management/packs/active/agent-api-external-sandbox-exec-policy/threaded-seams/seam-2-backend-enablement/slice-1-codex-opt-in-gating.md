### S1 — Codex backend opt-in gating (advertising + R0 allowlist)

- **User/system value**: unblocks SEAM-3 by making `agent_api.exec.external_sandbox.v1` available
  only when the host explicitly opts in, preserving safe-by-default behavior for Codex.
- **Scope (in/out)**:
  - In:
    - Add `CodexBackendConfig.allow_external_sandbox_exec: bool` (default `false`).
    - When `false`, exclude `agent_api.exec.external_sandbox.v1` from `capabilities().ids` and from
      `supported_extension_keys()` (R0 fail-closed as `UnsupportedCapability`).
    - When `true`, include the key in both `capabilities().ids` and `supported_extension_keys()`.
    - Add unit tests that pin both the advertising posture and the allowlist alignment.
  - Out:
    - Any Codex CLI argv / app-server RPC mapping for the key (SEAM-3).
    - Any changes to core key semantics or contradiction rules (SEAM-1).
- **Acceptance criteria**:
  - `CodexBackendConfig::default().allow_external_sandbox_exec == false`.
  - Default Codex backend `capabilities()` MUST NOT contain `agent_api.exec.external_sandbox.v1`.
  - With `allow_external_sandbox_exec == true`, Codex backend `capabilities()` MUST contain the key.
  - With `allow_external_sandbox_exec == false`, a request that includes
    `extensions["agent_api.exec.external_sandbox.v1"]` MUST fail closed as
    `AgentWrapperError::UnsupportedCapability` (R0).
  - The harness allowlist (`supported_extension_keys()`) and capability advertising remain aligned.
- **Dependencies**:
  - Contracts: `ES-C01` (key id) from SEAM-1; `ES-C03` (safe default advertising) owned by this seam.
  - Docs: `docs/specs/universal-agent-api/contract.md` “Dangerous capability opt-in ...” section.
- **Verification**:
  - Unit tests in `crates/agent_api/src/backends/codex/tests.rs`.
  - Optional integration check: run `cargo run -p xtask -- capability-matrix` and confirm the
    default matrix does not include `agent_api.exec.external_sandbox.v1`.
- **Rollout/safety**:
  - No behavior change for default backend instances; external sandbox mode remains unreachable
    without explicit host opt-in.

#### S1.T1 — Implement `allow_external_sandbox_exec` gating in the Codex backend

- **Outcome**: Codex exposes the dangerous key only behind explicit host configuration, and R0
  fail-closed behavior is preserved by default.
- **Inputs/outputs**:
  - Input: SEAM-2 brief + threading ES-C03; canonical config contract in
    `docs/specs/universal-agent-api/contract.md`.
  - Output: code changes in `crates/agent_api/src/backends/codex.rs`:
    - add `allow_external_sandbox_exec: bool` to `CodexBackendConfig`,
    - gate `capabilities().ids` insertion of `agent_api.exec.external_sandbox.v1`,
    - gate `supported_extension_keys()` to include/exclude the key.
- **Implementation notes**:
  - Prefer a single shared constant `EXT_EXTERNAL_SANDBOX_V1: &str =
    "agent_api.exec.external_sandbox.v1"` within the module for consistency with other `EXT_*`
    constants.
  - `supported_extension_keys()` returns `&'static [&'static str]`; implement gating by selecting
    between two `'static` arrays (default vs opt-in) based on the config flag.
  - Do not add any mapping/validation logic for the key in this task (SEAM-3 owns that).
- **Acceptance criteria**:
  - Meets all slice acceptance criteria.
  - `capabilities()` and `supported_extension_keys()` are always aligned w.r.t.
    `agent_api.exec.external_sandbox.v1`.
- **Test notes**:
  - Unit tests are added in `S1.T2`.
- **Risk/rollback notes**:
  - Low risk: default behavior is unchanged; rollback is a revert of the enablement commit.

Checklist:
- Implement: update `CodexBackendConfig`, `CodexBackend::capabilities()`, and `CodexHarnessAdapter::supported_extension_keys()`.
- Test: `cargo test -p agent_api --features codex codex_backend_reports_required_capabilities` (and new tests added in `S1.T2`).
- Validate: `rg -n "allow_external_sandbox_exec" crates/agent_api/src/backends/codex.rs` and confirm it gates both surfaces.
- Cleanup: keep capability id string identical across both gating sites.

#### S1.T2 — Add unit tests that pin default advertising and R0 allowlist behavior (Codex)

- **Outcome**: tests prevent accidental default advertising and prevent allowlist/capability drift.
- **Inputs/outputs**:
  - Input: `S1.T1` implementation.
  - Output: test changes in `crates/agent_api/src/backends/codex/tests.rs`.
- **Implementation notes**:
  - Add two small tests:
    - default config does not advertise `agent_api.exec.external_sandbox.v1`,
    - opt-in config advertises it and the adapter allowlist accepts the key.
  - For the R0 fail-closed check, build a minimal `AgentWrapperRunRequest` with a non-empty prompt
    and `extensions[EXT_EXTERNAL_SANDBOX_V1] = true`, then call
    `crate::backend_harness::normalize_request(...)` with a default adapter.
- **Acceptance criteria**:
  - Default backend capabilities do not contain the key.
  - Opt-in backend capabilities contain the key.
  - R0 gating fails closed as `UnsupportedCapability` when opt-in is disabled.
  - When opt-in is enabled, the request passes the allowlist gate (it may still fail later in
    SEAM-3 when mapping/validation is introduced).
- **Test notes**:
  - Keep the tests local and deterministic; do not spawn a real Codex process.
- **Risk/rollback notes**:
  - None (tests only).

Checklist:
- Implement: add/extend tests in `crates/agent_api/src/backends/codex/tests.rs`.
- Test: `cargo test -p agent_api --features codex codex_backend_reports_required_capabilities` and the new tests.
- Validate: ensure the new tests assert both `capabilities().ids` and the harness allowlist behavior.
- Cleanup: keep assertions focused on this seam (avoid pulling in SEAM-3 mapping details).

