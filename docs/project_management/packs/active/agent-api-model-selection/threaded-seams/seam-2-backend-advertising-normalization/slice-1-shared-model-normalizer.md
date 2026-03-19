# S1 — Shared model normalizer + normalized-request handoff

- **User/system value**: Gives the harness one canonical parser for `agent_api.config.model.v1`, so invalid requests fail before spawn with the pinned safe message and downstream seams receive a typed trimmed model id instead of raw extension JSON.
- **Scope (in/out)**:
  - In:
    - add one crate-local shared constant for `agent_api.config.model.v1`
    - add a harness-owned helper that normalizes the model-selection value after R0 gating
    - thread the normalized `Option<String>` into `NormalizedRequest`
    - add focused harness tests for valid, absent, and invalid shapes/bounds
  - Out:
    - built-in backend advertising flips
    - Codex / Claude builder `.model(...)` calls
    - runtime rejection translation and fake-binary integration fixtures
- **Acceptance criteria**:
  - The helper contract matches `MS-C09` exactly:
    - `Ok(None)` when the key is absent
    - `Ok(Some(trimmed_model_id))` when the key is a valid string after trimming
    - `Err(AgentWrapperError::InvalidRequest { message: "invalid agent_api.config.model.v1" })` for non-string, empty-after-trim, and oversize-after-trim inputs
  - Model normalization runs only after the harness allowlist accepted the key and before backend spawn.
  - `NormalizedRequest` carries a typed model-selection field so later seams can consume the normalized value without re-reading `request.extensions`.
  - No test or implementation path leaks the raw model id into an `InvalidRequest` message.
- **Dependencies**:
  - `MS-C01`, `MS-C02`, and `MS-C03` from SEAM-1
  - existing `backend_harness::normalize_request(...)` lifecycle in `crates/agent_api/src/backend_harness/normalize.rs`
- **Verification**:
  - `cargo test -p agent_api --features codex,claude_code backend_harness::normalize`
  - focused review that the only raw `agent_api.config.model.v1` parse site lives in `crates/agent_api/src/backend_harness/normalize.rs`
- **Rollout/safety**:
  - Internal-only handoff first; no public advertising change in this slice.
  - Preserve current fail-closed ordering: unsupported keys still die at R0 before model parsing.

## Atomic Tasks

#### S1.T1 — Define the shared model-selection constant and normalizer helper

- **Outcome**: One crate-local key constant and one helper implement the pinned v1 trim/bounds/error contract in the harness.
- **Inputs/outputs**:
  - Input:
    - `docs/specs/universal-agent-api/extensions-spec.md`
    - `docs/project_management/packs/active/agent-api-model-selection/threading.md`
  - Output:
    - `crates/agent_api/src/lib.rs` or another single crate-local constant home for `agent_api.config.model.v1`
    - `crates/agent_api/src/backend_harness/normalize.rs`
- **Implementation notes**:
  - Prefer one shared constant that both the harness and backend capability code import, rather than duplicating the string literal.
  - The helper should read only `request.extensions.get(CAPABILITY_MODEL_SELECTION_V1)` after `validate_extension_keys_fail_closed(...)` succeeds.
  - Trim leading/trailing Unicode whitespace, then validate:
    - non-empty after trim
    - `<= 128` UTF-8 bytes after trim
  - Every invalid case must map to the exact safe message `invalid agent_api.config.model.v1`.
  - The helper must not call backend policy extractors or builder code.
- **Acceptance criteria**:
  - Shared helper returns `Result<Option<String>, AgentWrapperError>` with the pinned outcomes.
  - Raw model ids do not appear in any error path.
- **Test notes**:
  - Covered in S1.T3.
- **Risk/rollback notes**:
  - Internal-only; easy to roll back by removing the helper if the contract changes before downstream adoption.

Checklist:
- Implement: add the shared key constant and `normalize_model_selection_v1(...)`.
- Test: absent, trimmed success, non-string, empty-after-trim, and oversize-after-trim cases.
- Validate: `cargo test -p agent_api --features codex,claude_code backend_harness::normalize`.
- Cleanup: keep the helper private to `backend_harness`.

#### S1.T2 — Carry the normalized model id on `NormalizedRequest`

- **Outcome**: Downstream backend mapping code receives a typed `Option<String>` on the harness-owned normalized request instead of touching raw extension payloads.
- **Inputs/outputs**:
  - Input:
    - `crates/agent_api/src/backend_harness/contract.rs`
    - `crates/agent_api/src/backend_harness/normalize.rs`
  - Output:
    - `crates/agent_api/src/backend_harness/contract.rs`
    - `crates/agent_api/src/backend_harness/normalize.rs`
    - any compile-fix touches required in `crates/agent_api/src/backends/{codex,claude_code}/harness.rs`
- **Implementation notes**:
  - Add a backend-neutral field such as `model_selection: Option<String>` to `NormalizedRequest<P>`.
  - Populate it exactly once in `normalize_request(...)` from the shared helper.
  - Keep `validate_and_extract_policy(...)` backend-owned and model-agnostic in this seam; those hooks must not become a second parser.
  - The harness-owned field should be the only value later slices refer to for model selection.
- **Acceptance criteria**:
  - `NormalizedRequest` exposes the typed normalized model id to both built-in adapters.
  - No backend policy extractor needs to re-trim, re-parse, or re-read `request.extensions["agent_api.config.model.v1"]`.
- **Test notes**:
  - Covered by a normalization test that inspects the returned `NormalizedRequest`.
- **Risk/rollback notes**:
  - Internal struct change only; low risk if all adapter call sites compile.

Checklist:
- Implement: add the typed field to `NormalizedRequest` and populate it in `normalize_request(...)`.
- Test: one success case asserts the normalized request carries the trimmed model id; one absence case asserts `None`.
- Validate: `cargo check -p agent_api --features codex,claude_code`.
- Cleanup: keep the field name backend-neutral and document it inline.

#### S1.T3 — Add harness tests for ordering, bounds, and typed handoff

- **Outcome**: The shared helper contract is pinned by harness-level unit tests before backend mapping seams adopt it.
- **Inputs/outputs**:
  - Input:
    - existing harness tests in `crates/agent_api/src/backend_harness/normalize/tests.rs`
  - Output:
    - `crates/agent_api/src/backend_harness/normalize/tests.rs`
    - optionally `crates/agent_api/src/backend_harness/normalize/tests/model_selection.rs`
- **Implementation notes**:
  - Cover the required matrix:
    - absent key -> `Ok(None)`
    - non-string -> pinned InvalidRequest
    - whitespace-only -> pinned InvalidRequest
    - oversize-after-trim -> pinned InvalidRequest
    - trimmed success -> `Some(trimmed_model_id)`
  - Keep an ordering test proving unsupported capability still wins before model parsing when the adapter does not admit the key.
  - Add a test that inspects `NormalizedRequest.model_selection` (or the final field name) so downstream seams inherit a pinned typed contract.
- **Acceptance criteria**:
  - Harness tests fail if any invalid case changes message text, if trim-before-bounds regresses, or if the typed handoff disappears.
- **Test notes**:
  - Run: `cargo test -p agent_api --features codex,claude_code backend_harness::normalize`.
- **Risk/rollback notes**:
  - Tests-only; safe.

Checklist:
- Implement: add model-selection normalization tests and one typed-handoff assertion.
- Test: `cargo test -p agent_api --features codex,claude_code backend_harness::normalize`.
- Validate: confirm the tests never spawn a backend process.
- Cleanup: keep fixtures small and explicit.
