### S1 — Pre-spawn validation ordering and invalid-request contract

- **User/system value**: proves the universal key fails closed before spawn and that all invalid
  inputs collapse to one safe, deterministic pre-spawn contract before any backend-specific mapping
  runs.
- **Scope (in/out)**:
  - In:
    - R0 ordering: unsupported `agent_api.config.model.v1` fails as
      `UnsupportedCapability` before validation.
    - Schema/bounds coverage for string-only, trim-before-empty, and trim-before-byte-length
      validation.
    - Exact `InvalidRequest` template pinning:
      `invalid agent_api.config.model.v1`.
    - No-spawn proof for invalid requests inside the harness normalization path.
  - Out:
    - backend capability advertising and matrix publication (covered later).
    - backend-specific argv placement, fork rejection, or runtime error translation (covered in
      `S2` and `S3`).
- **Acceptance criteria**:
  - Unsupported key tests fail as `AgentWrapperError::UnsupportedCapability` before any policy
    validation and do not leak the raw extension value.
  - Non-string, whitespace-only, and oversize-after-trim inputs fail as
    `AgentWrapperError::InvalidRequest { message: "invalid agent_api.config.model.v1" }`.
  - Trimmed valid inputs are accepted only after trimming, so whitespace-padded values that trim to
    a valid id do not fail as empty or oversize.
  - Tests prove invalid requests fail before spawn.
- **Dependencies**:
  - `MS-C03` from `SEAM-1`.
  - Harness normalize entrypoint in `crates/agent_api/src/backend_harness/normalize.rs`.
- **Verification**:
  - `cargo test -p agent_api backend_harness::normalize`
- **Rollout/safety**:
  - Tests only. This slice is safe to land first and should remain narrowly focused on pre-spawn
    contract ordering.

#### S1.T1 — Add R0 fail-closed ordering regression for the model key

- **Outcome**: the harness normalize path is pinned to reject unsupported
  `agent_api.config.model.v1` before calling backend policy validation.
- **Inputs/outputs**:
  - Input: `MS-C03`, `threading.md`, existing `PanicOnPolicyAdapter` test pattern in
    `crates/agent_api/src/backend_harness/normalize/tests.rs`.
  - Output: new or updated normalize tests that:
    - submit a request containing `agent_api.config.model.v1` plus a secret sentinel value,
    - keep the key unsupported in `supported_extension_keys()`,
    - assert `AgentWrapperError::UnsupportedCapability { capability: "agent_api.config.model.v1" }`,
    - assert the secret sentinel does not appear in the surfaced error.
- **Implementation notes**:
  - Follow the existing harness ordering pattern rather than introducing backend-specific fixtures.
  - Keep the test scoped to ordering; do not mix in mapping assertions here.
- **Acceptance criteria**:
  - The test fails if `validate_and_extract_policy()` is invoked for the unsupported key.
- **Test notes**:
  - Run: `cargo test -p agent_api backend_harness::normalize`.
- **Risk/rollback notes**:
  - None; this is a pure regression guard.

Checklist:
- Implement: add an unsupported-key ordering test for `agent_api.config.model.v1`.
- Test: `cargo test -p agent_api backend_harness::normalize`.
- Validate: confirm the failure is `UnsupportedCapability`, not `InvalidRequest`.
- Cleanup: keep the secret sentinel unique so the redaction assertion stays unambiguous.

#### S1.T2 — Add trim-first schema/bounds tests with exact safe InvalidRequest template

- **Outcome**: invalid model-selection inputs are pinned to the trim-first schema contract and the
  exact safe `InvalidRequest` message.
- **Inputs/outputs**:
  - Input: `MS-C03` and the pinned message in `threading.md` / `scope_brief.md`.
  - Output: normalize tests in `crates/agent_api/src/backend_harness/normalize/tests.rs` covering:
    - non-string payloads,
    - whitespace-only strings,
    - oversize-after-trim strings,
    - trimmed success cases where surrounding whitespace is removed before emptiness/length checks,
    - invalid requests returning the exact message `invalid agent_api.config.model.v1`.
- **Implementation notes**:
  - Use byte-length test cases, not character-count assumptions, so the UTF-8 bound remains pinned.
  - Where useful, pair a would-fail-untrimmed case with a valid trimmed case to prove ordering.
- **Acceptance criteria**:
  - Every invalid case resolves to the same exact safe template and does not echo the raw model id.
  - At least one success case proves trimming happens before validation.
- **Test notes**:
  - Run: `cargo test -p agent_api backend_harness::normalize`.
- **Risk/rollback notes**:
  - None; keep the test scope pre-spawn only.

Checklist:
- Implement: add non-string, empty-after-trim, oversize-after-trim, and trimmed-success cases.
- Test: `cargo test -p agent_api backend_harness::normalize`.
- Validate: assert the exact message `invalid agent_api.config.model.v1` in every invalid case.
- Cleanup: avoid backend-specific assertions in this slice.
