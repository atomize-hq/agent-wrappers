### S2b — Claude mapping, absence behavior, and fallback guardrails

- **User/system value**: proves the Claude backend consumes one trimmed normalized value,
  advertises support only when deterministic, preserves default behavior when the key is absent,
  and never lets the universal key drive `--fallback-model`.
- **Scope (in/out)**:
  - In:
    - Claude capability advertising assertions for `agent_api.config.model.v1`.
    - fresh/resume/fork mapping with exactly one `--model <trimmed-id>` pair.
    - argv placement before add-dir groups, session-selector flags, `--fallback-model`, and final
      `--verbose`.
    - absence/default behavior with no emitted `--model`.
    - negative regression proving the universal key never emits `--fallback-model`.
  - Out:
    - Codex mapping and fork rejection coverage.
    - shared single-parser source guards beyond any Claude-local references needed for this task.
    - post-init runtime rejection and terminal `Error` event behavior in `S3`.
- **Acceptance criteria**:
  - Claude capability tests pin `agent_api.config.model.v1` advertising only in the deterministic
    support state.
  - Mapping tests emit exactly one `--model <trimmed-id>` before add-dir, session, fallback, and
    final `--verbose` placement points.
  - Absence omits `--model`.
  - The universal key never causes `--fallback-model` emission.
- **Dependencies**:
  - `MS-C05` and `MS-C09` from `SEAM-2`.
  - `MS-C07` from `SEAM-4`.
  - `docs/specs/claude-code-session-mapping-contract.md`.
- **Verification**:
  - `cargo test -p agent_api claude_code`
- **Rollout/safety**:
  - Tests only. Keep assertions aligned to canonical argv order without over-pinning unrelated
    tokens.

#### S2b.T1 — Claude capability, argv-placement, and no-fallback mapping tests

- **Outcome**: Claude model-selection coverage pins capability advertising, exact argv placement,
  absence/default behavior, and explicit exclusion of `--fallback-model`.
- **Files**:
  - `crates/agent_api/src/backends/claude_code/tests/capabilities.rs`
  - `crates/agent_api/src/backends/claude_code/tests/mapping.rs`

Checklist:
- Implement:
  - Add capability assertions for `agent_api.config.model.v1`.
  - Add fresh/resume/fork mapping tests for trimmed success and absence.
  - Add a focused negative regression proving the universal key never emits `--fallback-model`.
- Test:
  - `cargo test -p agent_api claude_code`
- Validate:
  - Confirm `--model` precedes add-dir, session, fallback, and final `--verbose`.
  - Confirm absence preserves backend defaults with no emitted `--model`.
