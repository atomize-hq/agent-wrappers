### S1a — Shared model-selection constant and helper contract

- **User/system value**: The harness gets one canonical, trim-first parser for `agent_api.config.model.v1`, so invalid requests fail before spawn with the pinned safe message and later seams inherit one trusted normalization contract.
- **Scope (in/out)**:
  - In:
    - Add one crate-local shared constant for `agent_api.config.model.v1`.
    - Add a harness-owned helper in `backend_harness` that trims, validates bounds, and returns `Result<Option<String>, AgentWrapperError>`.
    - Keep raw model-id parsing confined to the shared helper after R0 allowlist validation succeeds.
  - Out:
    - Adding `NormalizedRequest.model_selection`.
    - Backend adapter compile-fix touches beyond the helper call site.
    - Harness regression-suite expansion beyond the minimal compile checks needed to land the helper.
- **Acceptance criteria**:
  - One shared constant is the only crate-local source for `agent_api.config.model.v1`.
  - The helper returns `Ok(None)` for absence, `Ok(Some(trimmed_model_id))` for valid trimmed strings, and `Err(AgentWrapperError::InvalidRequest { message: "invalid agent_api.config.model.v1" })` for non-string, empty-after-trim, and oversize-after-trim inputs.
  - The helper trims before applying the `<= 128` UTF-8 byte bound.
  - No invalid path echoes the raw model id in the error.
- **Dependencies**:
  - `SEAM-1`
  - `MS-C01`
  - `MS-C02`
  - `MS-C03`
- **Verification**:
  - `cargo check -p agent_api --features codex,claude_code`
  - Focused review that raw `agent_api.config.model.v1` parsing exists only in `crates/agent_api/src/backend_harness/normalize.rs`
- **Rollout/safety**:
  - Keep this sub-slice helper-local and internal-only; do not pull typed handoff or backend advertising changes into the same session.

#### S1.T1 — Define the shared model-selection constant and normalizer helper

- **Outcome**: One crate-local key constant and one harness-owned helper pin the v1 trim/bounds/error contract in a single implementation site.
- **Files**:
  - `crates/agent_api/src/lib.rs` or one other crate-local constant home
  - `crates/agent_api/src/backend_harness/normalize.rs`

Checklist:
- Implement:
  - add the shared `agent_api.config.model.v1` constant
  - add `normalize_model_selection_v1(...)` in `backend_harness`
  - read only `request.extensions.get(CAPABILITY_MODEL_SELECTION_V1)` after allowlist validation succeeds
- Test:
  - keep verification at compile/review level in this sub-slice; full regression coverage lands in `S1c`
- Validate:
  - confirm the helper trims before bounds checking
  - confirm every invalid branch resolves to the exact safe message `invalid agent_api.config.model.v1`
  - confirm no backend policy or builder code is called from the helper
