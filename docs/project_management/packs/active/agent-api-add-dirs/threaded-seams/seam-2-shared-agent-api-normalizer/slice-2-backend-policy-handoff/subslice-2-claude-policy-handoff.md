### S2b — Carry normalized add-dirs through Claude policy extraction

- **User/system value**: Claude matches Codex on effective cwd ownership and add-dir normalization, which gives downstream Claude mapping work one policy field to consume instead of backend-local raw payload parsing.
- **Scope (in/out)**:
  - In:
    - Capture `run_start_cwd` in Claude backend entrypoints before constructing the harness adapter.
    - Add `add_dirs: Vec<PathBuf>` to `ClaudeExecPolicy`.
    - Compute Claude effective cwd inside `ClaudeHarnessAdapter::validate_and_extract_policy(...)`.
    - Call `normalize_add_dirs_v1(...)` exactly once from Claude policy extraction.
    - Add Claude-only direct-policy tests covering absence, cwd precedence, and safe invalid propagation.
  - Out:
    - Codex backend changes.
    - Claude argv placement, print-request mapping, or capability advertisement work owned by downstream seams.
- **Acceptance criteria**:
  - `run(...)` and `run_control(...)` capture `std::env::current_dir().ok()` and feed it into the harness adapter as `run_start_cwd`.
  - `ClaudeExecPolicy` carries normalized `add_dirs: Vec<PathBuf>`.
  - Claude effective cwd precedence is `request.working_dir -> config.default_working_dir -> run_start_cwd` before helper invocation.
  - `ClaudeHarnessAdapter::validate_and_extract_policy(...)` calls the shared helper exactly once and returns `Vec::new()` when the key is absent.
  - Claude direct-policy tests prove run-start cwd participates only as the final fallback and that safe invalid helper errors propagate unchanged.
- **Dependencies**:
  - `S1`
  - `AD-C02`
  - `AD-C04`
  - `SEAM-1`
- **Verification**:
  - `cargo test -p agent_api claude_code`
  - Direct policy-extraction tests under `crates/agent_api/src/backends/claude_code/tests/`
- **Rollout/safety**:
  - Keep this sub-slice limited to backend entrypoint capture, policy extraction, and direct-policy tests; do not mix in Claude argv placement.

#### S2.T2 — Add normalized add-dirs and explicit run-start cwd capture to Claude policy extraction

- **Outcome**: Claude policy extraction matches Codex on cwd precedence and helper ownership, giving downstream Claude mapping work a normalized list instead of raw extension payloads.
- **Files**:
  - `crates/agent_api/src/backends/claude_code/backend.rs`
  - `crates/agent_api/src/backends/claude_code/harness.rs`
  - `crates/agent_api/src/backends/claude_code/tests/`

Checklist:
- Implement:
  - add `run_start_cwd: Option<PathBuf>` to `ClaudeHarnessAdapter`
  - capture run-start cwd in `crates/agent_api/src/backends/claude_code/backend.rs` before adapter construction
  - add `add_dirs: Vec<PathBuf>` to `ClaudeExecPolicy`
  - resolve effective cwd and call `normalize_add_dirs_v1(...)` in `ClaudeHarnessAdapter::validate_and_extract_policy(...)`
- Test:
  - add direct-policy cases for absent key, request/default/run-start precedence, and safe invalid propagation
  - prove valid relative inputs land on `policy.add_dirs`
- Validate:
  - confirm later Claude mapping paths do not reread the raw add-dir payload
  - preserve existing non-add-dir parsing order around the new helper call
