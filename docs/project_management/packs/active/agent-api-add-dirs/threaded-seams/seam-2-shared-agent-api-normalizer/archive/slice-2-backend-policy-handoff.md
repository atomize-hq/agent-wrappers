### S2 — Wire the shared helper into Codex and Claude policy extraction

- **User/system value**: both built-in backends consume one normalized `Vec<PathBuf>` from policy extraction, so SEAM-3 and SEAM-4 can focus purely on mapping and session behavior instead of path semantics.
- **Scope (in/out)**:
  - In:
    - Extend backend policy types to carry the normalized add-dir list.
    - Make Codex and Claude compute their effective working directory inside `validate_and_extract_policy(...)`, call `normalize_add_dirs_v1(...)`, and store the result on policy.
    - Capture Claude run-start cwd in backend entrypoints so request/default/run-start precedence is explicit before add-dir validation.
    - Add direct policy-extraction tests that prove the normalized list is attached to policy and that the absent-key case still yields `Vec::new()`.
  - Out:
    - Capability advertising for `agent_api.exec.add_dirs.v1`.
    - CLI argv placement, app-server mapping, or runtime rejection handling.
    - Capability-matrix regeneration and integration closeout.
- **Acceptance criteria**:
  - `CodexExecPolicy` carries `Vec<PathBuf>` representing the normalized add-dir set.
  - `ClaudeExecPolicy` carries `Vec<PathBuf>` representing the normalized add-dir set.
  - Codex effective cwd precedence is `request.working_dir -> config.default_working_dir -> run_start_cwd` before helper invocation.
  - Claude effective cwd precedence is `request.working_dir -> config.default_working_dir -> run_start_cwd` before helper invocation.
  - Each backend calls the helper exactly once during policy extraction and does not reread `request.extensions["agent_api.exec.add_dirs.v1"]` in later spawn/mapping code.
  - When the key is absent, both policies carry an empty vector.
- **Dependencies**:
  - `S1`
  - `AD-C02`
  - `AD-C04`
  - `SEAM-1`
- **Verification**:
  - Direct policy-extraction tests under `crates/agent_api/src/backends/codex/tests/`
  - Direct policy-extraction tests under `crates/agent_api/src/backends/claude_code/tests/`
- **Rollout/safety**:
  - This slice still stops short of argv mapping; downstream backend seams can adopt the policy field without reopening extension parsing.

## Atomic Tasks

#### S2.T1 — Add normalized add-dirs to Codex policy extraction

- **Outcome**: Codex policy extraction owns the effective cwd handoff and publishes a normalized add-dir list for exec/resume/fork flows to consume later.
- **Inputs/outputs**:
  - Input: `S1` helper export and existing Codex precedence inputs (`request.working_dir`, `config.default_working_dir`, `run_start_cwd`).
  - Output: code changes in `crates/agent_api/src/backends/codex/policy.rs` and `crates/agent_api/src/backends/codex/harness.rs`.
  - Output: direct Codex policy tests under `crates/agent_api/src/backends/codex/tests/`.
- **Implementation notes**:
  - Add `add_dirs: Vec<PathBuf>` to `CodexExecPolicy`.
  - In `CodexHarnessAdapter::validate_and_extract_policy(...)`, preserve existing resume/fork parsing, then resolve the effective working directory and call `normalize_add_dirs_v1(request.extensions.get("agent_api.exec.add_dirs.v1"), effective_working_dir)`.
  - Keep the helper call in policy extraction, not in exec/fork spawn code.
  - Follow the pack’s rule that invalid add-dir inputs fail as `InvalidRequest` before any Codex fork-specific rejection logic in SEAM-3.
- **Acceptance criteria**:
  - `validate_and_extract_policy(...)` returns normalized `add_dirs` for valid payloads and an empty vector when the key is absent.
  - Codex spawn paths have a policy field to consume later and no longer need to inspect raw add-dir extension payloads.
  - Direct tests prove request cwd beats default cwd and default cwd beats run-start cwd for relative resolution.
- **Test notes**:
  - Extend an existing direct-policy test module such as `crates/agent_api/src/backends/codex/tests/session_handle.rs`, or add a focused peer module if that keeps the scope cleaner.
- **Risk/rollback notes**:
  - Fork precedence is sensitive; do not move the existing fork rejection logic into this seam.

Checklist:
- Implement: add `add_dirs: Vec<PathBuf>` to `CodexExecPolicy`.
- Implement: resolve effective cwd and call `normalize_add_dirs_v1(...)` inside `CodexHarnessAdapter::validate_and_extract_policy(...)`.
- Test: direct policy extraction for absent key, request-vs-default cwd precedence, and safe invalid propagation.
- Validate: confirm later Codex files only consume `policy.add_dirs` once SEAM-3 lands.
- Cleanup: keep all raw payload access for this key inside policy extraction.

#### S2.T2 — Add normalized add-dirs and explicit run-start cwd capture to Claude policy extraction

- **Outcome**: Claude policy extraction matches Codex on cwd precedence and helper ownership, giving SEAM-4 a normalized list instead of raw extension payloads.
- **Inputs/outputs**:
  - Input: `S1` helper export and the pack’s pinned Claude cwd precedence requirement.
  - Output: code changes in `crates/agent_api/src/backends/claude_code/backend.rs` and `crates/agent_api/src/backends/claude_code/harness.rs`.
  - Output: direct Claude policy tests under `crates/agent_api/src/backends/claude_code/tests/`.
- **Implementation notes**:
  - Add `run_start_cwd: Option<PathBuf>` to `ClaudeHarnessAdapter` and feed it from the backend `run(...)` / `run_control(...)` entrypoints via `std::env::current_dir().ok()`.
  - Add `add_dirs: Vec<PathBuf>` to `ClaudeExecPolicy`.
  - In `ClaudeHarnessAdapter::validate_and_extract_policy(...)`, keep non-interactive/external-sandbox/session parsing intact, then compute `request.working_dir -> config.default_working_dir -> run_start_cwd` and call `normalize_add_dirs_v1(...)`.
  - Do not mix this slice with `ClaudePrintRequest` argv placement; that belongs to SEAM-4.
- **Acceptance criteria**:
  - Claude policy extraction returns normalized `add_dirs` and `Vec::new()` on absence.
  - `run(...)` and `run_control(...)` capture run-start cwd before constructing the harness adapter.
  - Direct tests prove the same cwd precedence ladder as Codex and safe invalid-message propagation from the helper.
- **Test notes**:
  - Add focused tests in an existing Claude backend test module or a new peer module if needed; keep them policy-extraction only.
- **Risk/rollback notes**:
  - Avoid changing current non-add-dir Claude behavior; only the cwd capture and policy field should be new.

Checklist:
- Implement: add `run_start_cwd` to `ClaudeHarnessAdapter` and capture it in `crates/agent_api/src/backends/claude_code/backend.rs`.
- Implement: add `add_dirs: Vec<PathBuf>` to `ClaudeExecPolicy`.
- Implement: resolve effective cwd and call `normalize_add_dirs_v1(...)` in `ClaudeHarnessAdapter::validate_and_extract_policy(...)`.
- Test: direct policy extraction for absent key, request/default/run-start precedence, and safe invalid propagation.
- Validate: confirm no later Claude mapping path rereads the raw add-dir payload.
- Cleanup: preserve existing resume/fork parsing order around the new helper call.
