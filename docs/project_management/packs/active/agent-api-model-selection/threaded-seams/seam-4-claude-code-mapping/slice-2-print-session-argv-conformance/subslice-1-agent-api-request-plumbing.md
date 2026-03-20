### S2a — Agent API Claude request plumbing

- **User/system value**: narrows the model-selection change to Claude's existing backend/harness request path so the universal key reaches Claude-specific request construction without reopening parsing or CLI-shaping concerns.
- **Scope (in/out)**:
  - In:
    - consume the typed `model: Option<String>` handoff from `S1`
    - thread the value through Claude backend/harness request construction inside `crates/agent_api`
    - preserve omission semantics by leaving the request-side model unset when the handoff is `None`
    - keep fresh print, resume, and fork flows on one request-construction path
  - Out:
    - root-flags argv ordering inside `crates/claude_code`
    - negative `--fallback-model` contract publication
    - runtime rejection translation after stream open
- **Acceptance criteria**:
  - Claude request construction receives `Some(trimmed_model_id)` unchanged from `S1` and applies request-side model wiring only for `Some(...)`
  - `None` remains the only representation of "no override" through the agent-api Claude path
  - no new raw parsing, trimming, or manual `--model` assembly appears in `crates/agent_api`
- **Dependencies**:
  - `S1` / `MS-C09` shared typed handoff
  - `MS-C02` absence semantics
  - `MS-C07` Claude mapping contract
- **Verification**:
  - targeted Claude backend/harness tests in `crates/agent_api`
  - regression grep proving `normalize.rs` remains the only raw parse site for `agent_api.config.model.v1`
- **Rollout/safety**:
  - localized to `crates/agent_api` request construction
  - safe first step because argv ordering and contract publication stay deferred to later sub-slices

#### S2a.T1 — Thread `model: Option<String>` through Claude request construction

- **Outcome**: Claude backend/harness code produces a request object that carries model intent for fresh, resume, and fork flows without changing CLI emission behavior directly.
- **Files**:
  - `crates/agent_api/src/backends/claude_code/backend.rs`
  - `crates/agent_api/src/backends/claude_code/harness.rs`
  - `crates/agent_api/src/backends/claude_code/tests/backend_contract.rs`
  - `crates/agent_api/src/backends/claude_code/tests/support.rs`

Checklist:
- Implement:
  - add or finish the request/plumbing field(s) needed for Claude request construction to carry `model: Option<String>`
  - apply request-side model wiring only when the handoff is `Some(...)`
  - keep fresh/resume/fork on the same request-construction path instead of branching into separate model logic
- Test:
  - extend the closest `crates/agent_api` Claude backend/harness tests to cover present and absent model selection
  - include fresh, `"last"`, and explicit-session selectors at the request-construction boundary
- Validate:
  - inspect the final request-construction path rather than asserting on intermediate policy state alone
  - verify no new `agent_api.config.model.v1` parse sites appear outside `normalize.rs`
