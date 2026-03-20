### S2a — Carry normalized add-dirs through Codex policy extraction

- **User/system value**: Codex gets a single normalized `Vec<PathBuf>` at policy-extraction time, so downstream spawn and session work can consume policy state instead of reopening raw extension parsing.
- **Scope (in/out)**:
  - In:
    - Add `add_dirs: Vec<PathBuf>` to `CodexExecPolicy`.
    - Compute Codex effective cwd inside `CodexHarnessAdapter::validate_and_extract_policy(...)`.
    - Call `normalize_add_dirs_v1(...)` exactly once from Codex policy extraction.
    - Add Codex-only direct-policy tests covering absence, cwd precedence, and safe invalid propagation.
  - Out:
    - Claude backend changes or run-start cwd capture.
    - Codex argv emission, session-branch placement, or fork-specific mapping work owned by downstream seams.
- **Acceptance criteria**:
  - `CodexExecPolicy` carries normalized `add_dirs: Vec<PathBuf>`.
  - Codex effective cwd precedence is `request.working_dir -> config.default_working_dir -> run_start_cwd` before helper invocation.
  - `CodexHarnessAdapter::validate_and_extract_policy(...)` calls the shared helper exactly once and returns `Vec::new()` when the key is absent.
  - Codex direct-policy tests prove request cwd beats default cwd and default cwd beats run-start cwd for relative path resolution.
  - Later Codex spawn/mapping code does not need to reread `request.extensions["agent_api.exec.add_dirs.v1"]`.
- **Dependencies**:
  - `S1`
  - `AD-C02`
  - `AD-C04`
  - `SEAM-1`
- **Verification**:
  - `cargo test -p agent_api codex`
  - Direct policy-extraction tests under `crates/agent_api/src/backends/codex/tests/`
- **Rollout/safety**:
  - Stop at policy extraction and direct-policy validation; do not pull argv mapping or fork rejection behavior into this sub-slice.

#### S2.T1 — Add normalized add-dirs to Codex policy extraction

- **Outcome**: Codex policy extraction owns the effective cwd handoff and publishes a normalized add-dir list for exec, resume, and fork flows to consume later.
- **Files**:
  - `crates/agent_api/src/backends/codex/policy.rs`
  - `crates/agent_api/src/backends/codex/harness.rs`
  - `crates/agent_api/src/backends/codex/tests/`

Checklist:
- Implement:
  - add `add_dirs: Vec<PathBuf>` to `CodexExecPolicy`
  - preserve existing resume/fork parsing, then resolve effective cwd and call `normalize_add_dirs_v1(request.extensions.get("agent_api.exec.add_dirs.v1"), effective_working_dir)`
  - keep raw payload access for this key inside policy extraction only
- Test:
  - add direct-policy cases for absent key, request-vs-default cwd precedence, and safe invalid propagation
  - prove valid relative inputs land on `policy.add_dirs`
- Validate:
  - confirm later Codex files can consume `policy.add_dirs` without new raw extension reads
  - keep invalid add-dir inputs failing before any downstream fork-specific mapping logic
