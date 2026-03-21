### S2b — Claude print argv ordering

- **User/system value**: makes the user-visible Claude CLI behavior deterministic by pinning exactly where `--model <trimmed-id>` appears for fresh, resume, and fork print flows.
- **Scope (in/out)**:
  - In:
    - use the existing `ClaudePrintRequest::model(...)` and `argv()` path in `crates/claude_code`
    - ensure model-present flows emit exactly one `--model <trimmed-id>` pair
    - ensure model-absent flows emit zero `--model` pairs
    - pin ordering before any `--add-dir` group, session-selector flags, and the final `--verbose` token
  - Out:
    - typed model handoff plumbing in `crates/agent_api`
    - universal-key negative fallback-model assertions
    - canonical spec publication
- **Acceptance criteria**:
  - fresh print, resume via `"last"`, and resume/fork via explicit session id all produce the same single-pair `--model` behavior when model is present
  - the same flows omit `--model` entirely when model is absent
  - ordering assertions fail if `--model` drifts rightward past session-selector flags, any `--add-dir` cluster, or the final `--verbose`
- **Dependencies**:
  - `S2a` request plumbing
  - `MS-C07` Claude mapping contract
  - `docs/specs/claude-code-session-mapping-contract.md`
- **Verification**:
  - targeted `crates/claude_code` builder/root-flags argv tests
  - focused spot-check that `ClaudePrintRequest::argv()` remains the only emission path for `--model`
- **Rollout/safety**:
  - bounded to `crates/claude_code` builder/argv behavior
  - safe because it reuses the established print builder instead of introducing a new CLI assembly path

#### S2b.T1 — Pin `--model` emission and ordering in Claude print argv tests

- **Outcome**: `crates/claude_code` owns the single source of truth for `--model` placement, and regressions in fresh/resume/fork argv shape fail in the nearest test layer.
- **Files**:
  - `crates/claude_code/src/commands/print.rs`
  - `crates/claude_code/tests/root_flags_argv.rs`

Checklist:
- Implement:
  - ensure `ClaudePrintRequest::model(...)` is the only builder hook that controls model argv emission
  - keep `--model` in the root-flags region before session selectors, `--add-dir`, and final `--verbose`
- Test:
  - add or extend fresh/resume/fork root-flags tests for present and absent model selection
  - cover both `"last"` and explicit-id selectors so one ordering rule governs both resume styles
- Validate:
  - assert on final argv token order, not just builder field state
  - confirm the builder emits exactly one `--model` pair in model-present cases
