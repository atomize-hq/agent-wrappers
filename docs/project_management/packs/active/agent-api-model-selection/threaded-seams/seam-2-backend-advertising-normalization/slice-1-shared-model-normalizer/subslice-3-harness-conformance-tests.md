### S1c — Harness conformance tests for ordering, bounds, and typed handoff

- **User/system value**: The shared model-selection contract is pinned with readable harness-level regressions before Codex and Claude mapping seams start depending on it.
- **Scope (in/out)**:
  - In:
    - Add harness-level tests for absence, type errors, whitespace-only inputs, oversize-after-trim failures, and trimmed-success cases.
    - Add one ordering test proving unsupported capability rejection still wins before model parsing when the backend does not admit the key.
    - Add one typed-handoff assertion proving `NormalizedRequest.model_selection` carries the normalized value.
  - Out:
    - Backend-local capability tests owned by `S2`.
    - Runtime rejection fixtures, fake binaries, or integration coverage owned by downstream seams.
    - Capability publication work owned by `S3`.
- **Acceptance criteria**:
  - Harness tests cover absent key, non-string, whitespace-only, oversize-after-trim, and trimmed-success cases.
  - Every invalid case pins the exact safe message `invalid agent_api.config.model.v1`.
  - One ordering test proves unsupported capability still fails before model parsing when the backend does not admit the key.
  - One typed-handoff assertion proves `NormalizedRequest.model_selection` carries the trimmed value on success and `None` on absence.
- **Dependencies**:
  - `S1a`
  - `S1b`
  - `MS-C03`
  - `MS-C09`
- **Verification**:
  - `cargo test -p agent_api --features codex,claude_code backend_harness::normalize`
- **Rollout/safety**:
  - Keep this test-only and helper-local so the regression suite lands before backend exposure or argv-mapping work grows around it.

#### S1.T3 — Add harness tests for ordering, bounds, and typed handoff

- **Outcome**: `backend_harness` owns the full regression matrix for safe model-selection normalization and the typed `NormalizedRequest` handoff.
- **Files**:
  - `crates/agent_api/src/backend_harness/normalize/tests.rs`
  - optionally `crates/agent_api/src/backend_harness/normalize/tests/model_selection.rs`

Checklist:
- Implement:
  - add focused normalization cases for absence, non-string, whitespace-only, oversize-after-trim, and trimmed success
  - add one ordering regression that proves unsupported capability still wins before helper parsing on backends that do not admit the key
  - add one typed-handoff assertion for `NormalizedRequest.model_selection`
- Test:
  - run `cargo test -p agent_api --features codex,claude_code backend_harness::normalize`
  - keep fixtures small and avoid backend process spawning
- Validate:
  - confirm invalid cases never leak the raw model id in assertion text or error messages
  - confirm trim-before-bounds behavior is pinned explicitly
  - confirm typed-handoff assertions cover both success and absence semantics
