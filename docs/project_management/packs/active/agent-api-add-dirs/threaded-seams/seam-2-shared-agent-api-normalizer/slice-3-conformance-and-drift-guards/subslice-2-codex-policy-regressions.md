### S3b — Pin Codex policy handoff regressions

- **User/system value**: Codex downstream work can rely on `policy.add_dirs` and cwd precedence staying stable, without reopening raw extension parsing or re-deriving helper semantics in mapping code.
- **Scope (in/out)**:
  - In:
    - Add Codex-only direct-policy regressions for normalized `add_dirs` attachment.
    - Pin empty-vector absence semantics and effective-cwd precedence inside Codex policy extraction.
    - Prove invalid add-dir input fails before later Codex-specific spawn or fork handling.
  - Out:
    - Claude backend coverage.
    - Codex argv ordering, session-branch placement, or capability advertisement.
- **Acceptance criteria**:
  - Codex direct-policy tests assert that absent input yields `policy.add_dirs == Vec::new()`.
  - Relative input resolution proves `request.working_dir -> config.default_working_dir -> run_start_cwd` precedence before helper invocation.
  - Invalid add-dir payloads fail through policy extraction without later mapping logic rereading `request.extensions["agent_api.exec.add_dirs.v1"]`.
- **Dependencies**:
  - `S2a`
  - `AD-C02`
  - `AD-C03`
  - `AD-C07`
- **Verification**:
  - `cargo test -p agent_api codex`
- **Rollout/safety**:
  - Keep this sub-slice Codex-only and limited to direct-policy tests so SEAM-3 can consume the resulting guardrail without mixing in argv behavior.

#### S3b.T1 — Add Codex direct-policy regressions for add-dir handoff

- **Outcome**: Codex policy extraction is pinned to the normalized handoff contract and cannot quietly drift back toward raw-payload parsing or inconsistent cwd precedence.
- **Files**:
  - `crates/agent_api/src/backends/codex/tests/`

Checklist:
- Implement:
  - add Codex direct-policy cases for absent key, normalized relative path attachment, and invalid-before-fork behavior
  - keep assertions focused on `policy.add_dirs` and effective cwd precedence
- Test:
  - cover request cwd beating default cwd and default cwd beating run-start cwd for relative inputs
  - assert invalid helper errors propagate without Codex-authored path text
- Validate:
  - confirm Codex test coverage does not depend on argv emission or session mapping
  - use repository search as needed to verify later Codex paths are not reopened for raw add-dir parsing

