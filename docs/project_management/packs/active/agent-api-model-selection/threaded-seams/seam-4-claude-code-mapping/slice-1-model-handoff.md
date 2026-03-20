### S1 — Model handoff into Claude policy/spawn wiring

- **User/system value**: gives Claude Code the minimal deterministic scaffold for v1 quickly: accepted model-selection requests reach Claude-specific wiring as typed state, and absent requests preserve current defaults without adding a second parser.
- **Scope (in/out)**:
  - In:
    - consume the SEAM-2 helper output instead of reading `request.extensions["agent_api.config.model.v1"]` in Claude-specific code
    - add a Claude policy/request field for the effective trimmed model id
    - thread that value through backend/harness spawn paths into `ClaudePrintRequest` construction
  - Out:
    - actual `.model(...)` argv mapping and ordering assertions (S2)
    - runtime model rejection translation after the stream has opened (S3)
    - capability advertising and matrix publication (SEAM-2)
- **Acceptance criteria**:
  - SEAM-4 consumes only SEAM-2's typed helper result for this key; no new raw parse sites appear outside `crates/agent_api/src/backend_harness/normalize.rs`.
  - When `agent_api.config.model.v1` is absent, Claude policy/spawn wiring carries `None` and leaves later mapping logic free to omit `.model(...)`.
  - When the key is present and valid, Claude policy/spawn wiring carries `Some(trimmed_model_id)` unchanged into the print/session construction path.
- **Dependencies**:
  - `SEAM-2` / `MS-C09` shared model-normalizer handoff
  - `SEAM-1` / `MS-C02` absence semantics
  - `MS-C07` Claude mapping contract
- **Verification**:
  - targeted Claude backend/unit tests for policy extraction or spawn-request construction
  - regression check that no second parser for `agent_api.config.model.v1` exists outside `normalize.rs`
- **Rollout/safety**:
  - additive and deterministic: only requests carrying the new key change behavior
  - safest first slice because it leaves argv ordering and runtime-rejection behavior unchanged until later slices pin them

#### S1.T1 — Adopt SEAM-2's normalized helper output in Claude policy extraction

- **Outcome**: Claude policy extraction carries `Option<String>` for the effective model id without re-reading raw extension payloads.
- **Inputs/outputs**:
  - Input: `MS-C09` helper from `crates/agent_api/src/backend_harness/normalize.rs`
  - Output: updates in `crates/agent_api/src/backends/claude_code/harness.rs`, `crates/agent_api/src/backends/claude_code/backend.rs`, and any shared policy/request types so Claude receives `model: Option<String>` alongside existing non-interactive and session-selector state
- **Implementation notes**:
  - keep ownership of trimming, bounds checks, and `InvalidRequest { message: "invalid agent_api.config.model.v1" }` in SEAM-2
  - preserve current session resume/fork mutual-exclusion validation and external-sandbox behavior
  - treat `None` as the only legal "no override" representation
- **Acceptance criteria**:
  - Claude code compiles against the shared helper output
  - no Claude module parses or trims `agent_api.config.model.v1` directly
  - the typed handoff reaches the spawn/request construction path unchanged
- **Test notes**:
  - add or extend policy/harness tests to show helper output reaches Claude wiring unchanged
  - validate with `rg -n "agent_api\\.config\\.model\\.v1" crates/agent_api/src/backends/claude_code crates/agent_api/src/backend_harness/normalize.rs`
- **Risk/rollback notes**:
  - low risk if kept as a typed-policy handoff only; rollback is localized to policy plumbing

Checklist:
- Implement: add `model: Option<String>` to the Claude policy/request path and source it from the shared helper output.
- Test: run targeted Claude backend/policy tests.
- Validate: confirm `normalize.rs` remains the only raw parse site for `agent_api.config.model.v1`.
- Cleanup: remove any temporary duplicate plumbing or dead helper code.
