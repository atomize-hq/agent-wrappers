### S1 — Publish `normalize_add_dirs_v1(...)` as the shared normalization contract

- **User/system value**: downstream backend seams get one backend-neutral helper that produces the accepted add-dir set and exact safe invalid-message shapes, eliminating backend-local drift before any argv work starts.
- **Scope (in/out)**:
  - In:
    - Add `normalize_add_dirs_v1(raw: Option<&serde_json::Value>, effective_working_dir: &Path) -> Result<Vec<PathBuf>, AgentWrapperError>` in `crates/agent_api/src/backend_harness/normalize.rs`.
    - Re-export the helper from `crates/agent_api/src/backend_harness/mod.rs` so backend adapters can call it.
    - Enforce the closed object schema, `dirs` requiredness, array/item bounds, Unicode trimming, relative resolution against the provided effective working directory, lexical normalization, `exists && is_dir`, dedup preserving first occurrence order, and safe invalid-message selection.
    - Add focused helper-level tests that prove the exported contract works before backend adoption begins.
  - Out:
    - Computing the effective working directory from request/config/run-start fallback.
    - Any policy-struct changes in Codex or Claude.
    - Any argv emission or session-branch mapping assertions.
- **Acceptance criteria**:
  - `crates/agent_api/src/backend_harness/normalize.rs` exports `normalize_add_dirs_v1(...)` with the seam-pinned signature.
  - When `raw` is `None`, the helper returns `Ok(Vec::new())`.
  - Invalid inputs use only the safe templates `invalid agent_api.exec.add_dirs.v1`, `invalid agent_api.exec.add_dirs.v1.dirs`, or `invalid agent_api.exec.add_dirs.v1.dirs[<i>]`.
  - Relative inputs resolve from the supplied `effective_working_dir`; the helper does not inspect request/config defaults on its own.
  - The helper-level tests cover at least one success path and one invalid path for each safe-message shape.
- **Dependencies**:
  - `SEAM-1`
  - `AD-C01`
  - `AD-C03`
  - `AD-C07`
- **Verification**:
  - `cargo test -p agent_api backend_harness::normalize`
  - Focused helper tests in `crates/agent_api/src/backend_harness/normalize/tests.rs`
- **Rollout/safety**:
  - No backend behavior changes yet; this slice only publishes the reusable contract and its direct tests.

## Atomic Tasks

#### S1.T1 — Implement schema parsing and safe invalid-message routing

- **Outcome**: the shared helper accepts only the pinned closed-object v1 shape and reports failures with the exact safe message families owned by SEAM-1.
- **Inputs/outputs**:
  - Input: `AD-C01` schema definition and `AD-C03` safe message templates from `threading.md`.
  - Output: parsing/validation logic in `crates/agent_api/src/backend_harness/normalize.rs`.
  - Output: helper export from `crates/agent_api/src/backend_harness/mod.rs`.
- **Implementation notes**:
  - Reject non-object payloads at the top-level key.
  - Enforce a closed object with only `dirs`.
  - Require `dirs` to be an array of `1..=16` strings.
  - Enforce per-entry Unicode trim, non-empty-after-trim, and `<= 1024` UTF-8 bytes after trimming.
  - Map top-level/object errors to `invalid agent_api.exec.add_dirs.v1`, `dirs` container errors to `invalid agent_api.exec.add_dirs.v1.dirs`, and per-entry failures to `invalid agent_api.exec.add_dirs.v1.dirs[<i>]`.
- **Acceptance criteria**:
  - All malformed payload classes route to one of the exact safe template families.
  - No helper error for this key includes raw path text or arbitrary backend-authored prose.
  - The helper is callable from other backend modules through `crate::backend_harness`.
- **Test notes**:
  - Add focused unit tests for non-object, unknown-key, missing-`dirs`, non-array, length-bound, non-string, trimmed-empty, and byte-bound failures.
- **Risk/rollback notes**:
  - Keep the helper private to the crate boundary; rollback is isolated to the helper export and its tests.

Checklist:
- Implement: add the helper signature, parsing helpers, and safe error routing in `crates/agent_api/src/backend_harness/normalize.rs`.
- Implement: re-export the helper from `crates/agent_api/src/backend_harness/mod.rs`.
- Test: helper unit cases for each safe-message family.
- Validate: confirm `rg -n "invalid agent_api\\.exec\\.add_dirs\\.v1" crates/agent_api/src/backend_harness` only shows the pinned templates.
- Cleanup: keep path text out of all `InvalidRequest` branches for this key.

#### S1.T2 — Implement resolution, normalization, dedup, and focused success-path tests

- **Outcome**: the shared helper returns the exact normalized `Vec<PathBuf>` contract that downstream backends will consume.
- **Inputs/outputs**:
  - Input: `effective_working_dir: &Path` from the backend adapter layer.
  - Output: path-resolution and normalization logic in `crates/agent_api/src/backend_harness/normalize.rs`.
  - Output: success-path helper tests in `crates/agent_api/src/backend_harness/normalize/tests.rs`.
- **Implementation notes**:
  - Resolve relative entries against the supplied effective working directory.
  - Apply lexical normalization only; do not canonicalize, expand env vars, or resolve symlinks.
  - Validate `exists && is_dir` before returning.
  - Deduplicate after normalization while preserving the first occurrence order.
- **Acceptance criteria**:
  - Success cases return `Vec<PathBuf>` in normalized first-occurrence order.
  - Duplicate paths collapse after normalization without becoming an error.
  - Missing paths and non-directory paths fail with safe indexed messages.
- **Test notes**:
  - Use temp directories to cover relative resolution, lexical normalization, dedup order preservation, and missing/non-directory rejection.
  - Keep the test matrix helper-local; backend-policy adoption belongs to `S2`.
- **Risk/rollback notes**:
  - Filesystem checks are the main leak risk; assert on safe messages, not just error kinds.

Checklist:
- Implement: relative resolution, lexical normalization, `exists && is_dir`, and order-preserving dedup.
- Test: success-path tempdir fixtures plus missing/non-directory rejection.
- Validate: verify `None` input still returns `Ok(Vec::new())`.
- Cleanup: avoid baking backend-specific cwd precedence into the helper.
