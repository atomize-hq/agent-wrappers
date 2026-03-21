### S1b — Typed `NormalizedRequest` model-selection handoff

- **User/system value**: Built-in adapters receive a typed normalized `Option<String>` from the harness, so downstream seams can map model selection without reopening raw extension parsing.
- **Scope (in/out)**:
  - In:
    - Add a backend-neutral `model_selection: Option<String>` field to `NormalizedRequest`.
    - Populate the field exactly once from the shared helper during `normalize_request(...)`.
    - Apply any compile-surface updates required in built-in harness adapters so they consume the normalized field later instead of `request.extensions`.
  - Out:
    - Any backend-local raw parser, trimming, or validation logic.
    - `--model` argv emission and runtime rejection behavior owned by downstream seams.
    - Capability exposure or advertising work owned by `S2`/`S3`.
- **Acceptance criteria**:
  - `NormalizedRequest` exposes a backend-neutral typed model-selection field.
  - `normalize_request(...)` is the only place that populates the field.
  - Codex and Claude harness modules compile with the new field available for downstream consumption, without adding a second parser.
  - Later seams can consume the typed handoff without rereading `request.extensions["agent_api.config.model.v1"]`.
- **Dependencies**:
  - `S1a`
  - `MS-C09`
- **Verification**:
  - `cargo check -p agent_api --features codex,claude_code`
  - Focused review of `crates/agent_api/src/backends/{codex,claude_code}/harness.rs` for absence of new raw parser logic
- **Rollout/safety**:
  - Keep the field backend-neutral and stop at typed plumbing; do not fold in backend-specific mapping behavior in this sub-slice.

#### S1.T2 — Carry the normalized model id on `NormalizedRequest`

- **Outcome**: The harness publishes one typed `Option<String>` handoff so built-in adapters can rely on normalized state instead of raw extension payloads.
- **Files**:
  - `crates/agent_api/src/backend_harness/contract.rs`
  - `crates/agent_api/src/backend_harness/normalize.rs`
  - `crates/agent_api/src/backends/codex/harness.rs`
  - `crates/agent_api/src/backends/claude_code/harness.rs`

Checklist:
- Implement:
  - add the backend-neutral `model_selection: Option<String>` field to `NormalizedRequest`
  - populate it exactly once in `normalize_request(...)` via the shared helper from `S1a`
  - make only the minimal harness compile-surface changes needed for downstream seams to read the typed field later
- Test:
  - keep runtime assertions for the typed handoff in `S1c`
- Validate:
  - confirm backend policy extractors remain model-agnostic in this seam
  - confirm no built-in harness module re-trims, re-parses, or rereads the raw extension payload
  - confirm the new field name stays backend-neutral
