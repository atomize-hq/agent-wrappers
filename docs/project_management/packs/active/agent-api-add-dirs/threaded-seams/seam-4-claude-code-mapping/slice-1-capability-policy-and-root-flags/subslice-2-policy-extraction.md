### S1b — Claude policy extraction for normalized add dirs

- **User/system value**: Claude converts accepted add-dir input into backend policy state exactly
  once, using the same effective working directory and validation contract as the shared
  normalizer.
- **Scope (in/out)**:
  - In:
    - Extend `ClaudeExecPolicy` with normalized `add_dirs: Vec<PathBuf>`.
    - Resolve the effective working directory from request, backend default, or run-start cwd.
    - Call `normalize_add_dirs_v1(...)` once during `validate_and_extract_policy(...)`.
    - Add backend-local tests for absent-key and effective-working-dir behavior.
  - Out:
    - Capability advertisement surfaces.
    - Fresh-run argv emission and ordering docs.
    - Resume/fork branch parity and runtime rejection handling.
- **Acceptance criteria**:
  - Absent add-dir input yields `Vec::new()` with no backend-local fallback behavior.
  - Relative add-dir paths resolve against the effective working directory Claude will actually
    use.
  - Downstream Claude spawn code reads only the policy field, not raw request extension payload.
  - Invalid or non-directory input continues to surface the shared safe `InvalidRequest` posture.
- **Dependencies**:
  - Consumes `AD-C02`, `AD-C03`, and the request/default-working-dir precedence from the seam
    inputs.
  - Depends on `S1a` for eventual merge because the key should not be implemented invisibly for
    long.
- **Verification**:
  - Backend-local unit coverage around `validate_and_extract_policy(...)`
  - `cargo test -p agent_api claude_code`
- **Rollout/safety**:
  - Keep the effective-working-dir handoff explicit in both code and tests.
  - Delete backend-local raw-extension reads as part of the same change so there is one data path.

#### S1b.T1 — Normalize and store add dirs on Claude policy state

- **Outcome**: Claude policy extraction owns the single source of truth for normalized add-dir
  state used by later argv-building code.
- **Files**:
  - `crates/agent_api/src/backends/claude_code/harness.rs`
  - `crates/agent_api/src/backends/claude_code/backend.rs`
  - `crates/agent_api/src/backends/claude_code/tests/backend_contract.rs`

Checklist:
- Implement:
  - Add `add_dirs: Vec<PathBuf>` to `ClaudeExecPolicy`.
  - Thread the effective working directory into `normalize_add_dirs_v1(...)`.
  - Remove any downstream rereads of `request.extensions["agent_api.exec.add_dirs.v1"]`.
- Test:
  - Add Claude backend coverage for absent-key extraction.
  - Add a case that proves relative add dirs resolve against the effective working directory.
  - Add a case that proves invalid inputs still flow through the shared `InvalidRequest` shape.
- Validate:
  - Confirm the shared helper is called exactly once in `validate_and_extract_policy(...)`.
  - Confirm later Claude wiring can consume only policy state.
