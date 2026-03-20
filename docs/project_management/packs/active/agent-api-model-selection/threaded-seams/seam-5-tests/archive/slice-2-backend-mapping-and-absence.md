### S2 — Backend mapping, absence semantics, and shared-normalizer handoff

- **User/system value**: proves both built-in backends consume one trimmed normalized value,
  preserve default behavior when the key is absent, and emit exactly the argv/rejection behavior
  promised by the backend contracts without re-parsing raw extension payloads.
- **Scope (in/out)**:
  - In:
    - built-in capability advertising assertions for `agent_api.config.model.v1` once deterministic
      support lands.
    - Codex exec/resume mapping with exactly one `--model <trimmed-id>` pair and absence behavior.
    - Codex fork safe pre-handle rejection for accepted model-selection inputs.
    - Claude print/resume/fork mapping with exactly one `--model <trimmed-id>` pair and explicit
      exclusion of `--fallback-model`.
    - regression checks that backend seams consume the shared `Option<String>` handoff rather than
      reading `request.extensions["agent_api.config.model.v1"]` again.
  - Out:
    - post-stream runtime rejection and terminal `Error` event behavior (covered in `S3`).
    - capability-matrix freshness after `xtask` regeneration (covered in `S3`).
- **Acceptance criteria**:
  - Codex and Claude capability tests pin advertising of `agent_api.config.model.v1` only once the
    backend is deterministic.
  - Codex exec/resume mapping emits exactly one `--model <trimmed-id>` pair; absence omits it.
  - Codex fork rejects accepted model-selection inputs before any app-server request with the
    pinned safe backend message.
  - Claude mapping emits exactly one `--model <trimmed-id>` before add-dir, session-selector,
    `--fallback-model`, and final `--verbose` placement points; absence omits it.
  - The universal key never causes `--fallback-model` emission.
  - Backend contract coverage proves no backend-local raw parse of
    `agent_api.config.model.v1` was reintroduced outside `normalize.rs`.
- **Dependencies**:
  - `MS-C05` and `MS-C09` from `SEAM-2`.
  - `MS-C06` from `SEAM-3`.
  - `MS-C07` from `SEAM-4`.
- **Verification**:
  - `cargo test -p agent_api codex`
  - `cargo test -p agent_api claude_code`
- **Rollout/safety**:
  - Tests only. Keep Codex and Claude assertions isolated to their existing test modules to avoid
    cross-backend coordination.

#### S2.T1 — Codex capability, exec/resume mapping, and fork-rejection tests

- **Outcome**: Codex model-selection coverage pins capability advertising, trimmed mapping,
  absence/default behavior, and the pinned fork rejection path.
- **Inputs/outputs**:
  - Input: `MS-C05`, `MS-C06`, `MS-C09`, `docs/specs/codex-streaming-exec-contract.md`, and
    `docs/specs/codex-app-server-jsonrpc-contract.md`.
  - Output: updates in:
    - `crates/agent_api/src/backends/codex/tests/capabilities.rs`
    - `crates/agent_api/src/backends/codex/tests/mapping.rs`
    - `crates/agent_api/src/backends/codex/tests/app_server.rs`
    - `crates/agent_api/src/backends/codex/tests/backend_contract.rs`
    covering:
    - advertised capability inclusion for `agent_api.config.model.v1`,
    - exactly one `--model <trimmed-id>` pair for exec/resume,
    - absence preserving the backend default with no emitted `--model`,
    - fork rejecting accepted values before `thread/list`, `thread/fork`, or `turn/start`,
    - source-level guardrails that Codex mapping reads the normalized handoff rather than the raw
      extension payload.
- **Implementation notes**:
  - Reuse existing mapping/app-server test surfaces instead of creating new integration scaffolding
    unless the current unit surfaces are insufficient.
  - Keep the fork rejection tests clearly pre-handle; post-stream failures belong in `S3`.
- **Acceptance criteria**:
  - Codex tests fail if a second `--model` pair appears, if the raw whitespace-padded value leaks to
    argv, or if the fork path tries to continue into app-server requests.
- **Test notes**:
  - Run: `cargo test -p agent_api codex`.
- **Risk/rollback notes**:
  - None; do not fold midstream runtime failures into this task.

Checklist:
- Implement:
  - Add capability assertions for `agent_api.config.model.v1`.
  - Add exec/resume mapping tests for trimmed success + absence.
  - Add fork rejection tests for the pinned safe backend message.
  - Add or extend source-guard tests proving Codex mapping does not re-parse the raw extension.
- Test: `cargo test -p agent_api codex`.
- Validate: confirm `--model` appears exactly once and only with the trimmed value.
- Cleanup: keep pre-handle fork coverage separate from runtime-rejection coverage.

#### S2.T2 — Claude capability, argv-placement, and no-fallback mapping tests

- **Outcome**: Claude model-selection coverage pins capability advertising, exact argv placement,
  absence/default behavior, and explicit exclusion of `--fallback-model`.
- **Inputs/outputs**:
  - Input: `MS-C05`, `MS-C07`, `MS-C09`, and
    `docs/specs/claude-code-session-mapping-contract.md`.
  - Output: updates in:
    - `crates/agent_api/src/backends/claude_code/tests/capabilities.rs`
    - `crates/agent_api/src/backends/claude_code/tests/mapping.rs`
    - `crates/agent_api/src/backends/claude_code/tests/backend_contract.rs`
    covering:
    - advertised capability inclusion for `agent_api.config.model.v1`,
    - exactly one `--model <trimmed-id>` before add-dir groups, session-selector flags,
      `--fallback-model`, and final `--verbose`,
    - absence preserving the backend default with no emitted `--model`,
    - negative assertions that the universal key never drives `--fallback-model`,
    - source-level guardrails that Claude mapping consumes the normalized handoff rather than the
      raw extension payload.
- **Implementation notes**:
  - Keep the placement assertions aligned with the canonical spec order rather than over-pinning
    unrelated argv tokens.
  - Prefer one positive placement test per flow plus one focused negative `--fallback-model`
    regression.
- **Acceptance criteria**:
  - Claude tests fail if `--model` moves to the right of session/fallback flags or if the
    universal key begins affecting `--fallback-model`.
- **Test notes**:
  - Run: `cargo test -p agent_api claude_code`.
- **Risk/rollback notes**:
  - None; post-init runtime rejection belongs in `S3`.

Checklist:
- Implement:
  - Add capability assertions for `agent_api.config.model.v1`.
  - Add fresh/resume/fork mapping tests for trimmed success + absence.
  - Add negative regression proving the universal key never emits `--fallback-model`.
  - Add or extend source-guard tests proving Claude mapping does not re-parse the raw extension.
- Test: `cargo test -p agent_api claude_code`.
- Validate: confirm `--model` precedes add-dir, session, fallback, and final `--verbose`.
- Cleanup: keep runtime rejection out of this task.

#### S2.T3 — Shared-normalizer conformance guard against backend-local re-parsing

- **Outcome**: the test suite makes the single-parser rule reviewable and hard to regress.
- **Inputs/outputs**:
  - Input: `MS-C09` and the single-parser rule in `threading.md` / `scope_brief.md`.
  - Output: focused source or contract tests in:
    - `crates/agent_api/src/backends/codex/tests/backend_contract.rs`
    - `crates/agent_api/src/backends/claude_code/tests/backend_contract.rs`
    that prove backend mapping code consumes a normalized handoff and does not read
    `agent_api.config.model.v1` directly from `request.extensions`.
- **Implementation notes**:
  - Use the existing `include_str!` contract-test style already present in both backend suites.
  - Keep the assertion narrow: the goal is to prevent a second parser, not to freeze unrelated
    source formatting.
- **Acceptance criteria**:
  - The guard fails if a future diff reintroduces direct raw-extension parsing in backend mapping
    modules.
- **Test notes**:
  - Run: `cargo test -p agent_api codex`; `cargo test -p agent_api claude_code`.
- **Risk/rollback notes**:
  - If source-string assertions become too brittle, convert them to a smaller explicit review hook
    in the same test files rather than dropping the guard entirely.

Checklist:
- Implement: add single-parser regression assertions in the backend contract test modules.
- Test: `cargo test -p agent_api codex`; `cargo test -p agent_api claude_code`.
- Validate: allow `normalize.rs` to own the raw key; disallow backend-local reads.
- Cleanup: avoid broad source snapshots.
