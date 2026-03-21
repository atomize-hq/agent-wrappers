### S2c — Fallback exclusion and contract publication

- **User/system value**: prevents the universal model key from silently acquiring Claude fallback semantics and leaves reviewers with one canonical ordering/exclusion contract to validate against.
- **Scope (in/out)**:
  - In:
    - add negative assertions that `agent_api.config.model.v1` never drives `.fallback_model(...)` or `--fallback-model`
    - update canonical Claude mapping text to match the final builder/request behavior
    - align the smallest focused test/doc surfaces that SEAM-5B will consume
  - Out:
    - request plumbing inside `crates/agent_api`
    - primary `--model` emission ordering work inside `crates/claude_code`
    - runtime rejection/error-event parity from `S3`
- **Acceptance criteria**:
  - canonical Claude spec text and focused tests pin the same ordering and fallback-exclusion rules
  - a regression that routes the universal key into `.fallback_model(...)` or moves `--model` behind `--fallback-model` fails loudly
  - downstream SEAM-5B work can cite this published contract rather than rediscovering it
- **Dependencies**:
  - `S2a` and `S2b` landed behavior
  - `MS-C07` Claude mapping contract
  - `docs/specs/claude-code-session-mapping-contract.md`
- **Verification**:
  - targeted Claude mapping/root-flags tests
  - spec diff review against the landed request/builder code paths
- **Rollout/safety**:
  - low-risk contract and regression-hardening pass
  - keep this last so docs/tests publish the final already-landed behavior

#### S2c.T1 — Publish fallback exclusion in specs and focused tests

- **Outcome**: the canonical Claude contract and its nearest focused tests explicitly separate universal model selection from fallback-model behavior.
- **Files**:
  - `docs/specs/claude-code-session-mapping-contract.md`
  - `crates/agent_api/src/backends/claude_code/tests/mapping.rs`
  - `crates/claude_code/tests/root_flags_argv.rs`

Checklist:
- Implement:
  - update canonical spec language to state that the universal key only drives `--model`
  - add negative assertions proving the universal key never reaches `.fallback_model(...)` / `--fallback-model`
  - keep doc language anchored to the established builder/request path instead of harness-specific implementation detail
- Test:
  - run the smallest focused Claude mapping/root-flags test set that covers ordering and fallback exclusion together
  - include at least one case where another path sets fallback-model while the universal key still affects only `--model`
- Validate:
  - diff spec wording against the landed code paths in `crates/agent_api` and `crates/claude_code`
  - confirm each original `S2` concern is represented exactly once across `S2a`, `S2b`, and `S2c`
