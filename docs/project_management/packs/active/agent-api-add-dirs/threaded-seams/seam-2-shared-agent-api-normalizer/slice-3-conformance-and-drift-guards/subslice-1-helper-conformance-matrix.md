### S3a — Expand the shared helper conformance matrix

- **User/system value**: the seam-owned helper becomes the single trusted source for add-dir normalization because every success and failure branch in `AD-C02` is pinned with readable regression coverage.
- **Scope (in/out)**:
  - In:
    - Expand `backend_harness` helper tests to cover the full verification matrix from the seam brief.
    - Group failures by shape, container, entry, and filesystem/path-normalization behavior.
    - Add no-path-leak assertions for every malformed-entry and filesystem-rejection path.
  - Out:
    - Codex or Claude backend-local policy tests.
    - Any argv, capability, or session-flow assertions owned by downstream seams.
- **Acceptance criteria**:
  - `crates/agent_api/src/backend_harness/normalize/tests.rs` covers non-object payloads, unknown keys, missing `dirs`, non-array `dirs`, bounds failures, non-string entries, trimmed-empty entries, byte-length bounds, relative resolution, lexical normalization, dedup order preservation, missing path rejection, non-directory rejection, absence semantics, and exact safe-message selection.
  - Every failing filesystem fixture asserts that `err.to_string()` omits the rejected raw path text.
  - Success-path cases prove normalized first-occurrence order without backend-specific cwd logic.
- **Dependencies**:
  - `S1`
  - `AD-C02`
  - `AD-C03`
  - `AD-C07`
- **Verification**:
  - `cargo test -p agent_api backend_harness::normalize`
- **Rollout/safety**:
  - Keep this sub-slice helper-local so the exhaustive contract can land before backend-specific regression suites grow around it.

#### S3a.T1 — Cover the full helper normalization and redaction matrix

- **Outcome**: `backend_harness` owns a single exhaustive regression corpus for the shared helper contract, including explicit proof that safe invalid messages never leak raw path text.
- **Files**:
  - `crates/agent_api/src/backend_harness/normalize/tests.rs`

Checklist:
- Implement:
  - add table-driven or grouped helper tests for each top-level object-shape, `dirs` container-shape, entry-validation, and filesystem-normalization case from the seam brief
  - keep absence semantics and dedup-order assertions in this helper-local suite
- Test:
  - use tempdir fixtures for relative resolution, lexical normalization, duplicate collapse, missing-path rejection, and non-directory rejection
  - assert exact safe message families for top-level, container, and indexed failures
- Validate:
  - prove every filesystem rejection path satisfies `!err.to_string().contains(raw_path)`
  - keep backend-specific policy or argv behavior out of this file

