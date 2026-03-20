### S1a — Codex policy model handoff

- **User/system value**: narrows the first landing step to typed policy plumbing so Codex can consume SEAM-2's normalized model result without adding a second parser or changing runtime behavior yet.
- **Scope (in/out)**:
  - In:
    - consume the `Result<Option<String>, AgentWrapperError>` handoff from `crates/agent_api/src/backend_harness/normalize.rs`
    - add `model: Option<String>` to the Codex policy/harness/backend request path
    - preserve current exec/resume/fork selector and session mutual-exclusion rules while carrying the new typed field
  - Out:
    - calling `CodexClientBuilder::model(...)`
    - argv ordering assertions and exec/resume spawn verification
    - fork rejection behavior and runtime rejection translation
- **Acceptance criteria**:
  - Codex policy extraction consumes only the shared typed helper output for `agent_api.config.model.v1`
  - `None` remains the sole no-override representation throughout the Codex policy path
  - no Codex module parses, trims, or inspects raw extension JSON for this key
- **Dependencies**:
  - `SEAM-2` / `MS-C09` shared model-normalizer handoff
  - `SEAM-1` / `MS-C02` absence semantics
  - `MS-C06` Codex mapping contract
- **Verification**:
  - targeted Codex policy/harness tests
  - `rg -n "agent_api\\.config\\.model\\.v1" crates/agent_api/src/backends/codex crates/agent_api/src/backend_harness/normalize.rs`
- **Rollout/safety**:
  - lowest-risk entry point because it is typed plumbing only and does not change argv emission on its own

#### S1a.T1 — Adopt SEAM-2's normalized helper output in Codex policy extraction

- **Outcome**: Codex policy construction carries `Option<String>` unchanged from the shared helper into downstream request structs.
- **Files**:
  - `crates/agent_api/src/backend_harness/normalize.rs`
  - `crates/agent_api/src/backends/codex/policy.rs`
  - `crates/agent_api/src/backends/codex/harness.rs`
  - `crates/agent_api/src/backends/codex/backend.rs`

Checklist:
- Implement:
  - add `model: Option<String>` to the Codex policy/request types that fan out into exec and fork flows
  - source that field from the shared helper rather than from raw extension access in Codex code
  - preserve current selector validation and keep the typed value read-only
- Test:
  - extend policy or harness tests to show the helper output reaches Codex policy unchanged
- Validate:
  - confirm `normalize.rs` remains the only raw parse site for `agent_api.config.model.v1`
  - verify no trimming or string normalization logic is introduced in Codex modules
