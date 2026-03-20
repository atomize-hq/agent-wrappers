### S3 — Lock conformance with exhaustive edge-case and handoff regression coverage

- **User/system value**: downstream seams can trust the shared helper and backend handoff contract because the full normalization/error matrix is pinned once, in the seam that owns it.
- **Scope (in/out)**:
  - In:
    - Expand helper tests to cover the full seam-owned verification matrix from the seam brief.
    - Add backend-local regression tests that pin policy attachment, absence semantics, and working-directory precedence for both built-in backends.
    - Add explicit no-path-leak assertions around all filesystem-failure and malformed-entry cases.
  - Out:
    - Backend argv order, runtime rejection parity, and capability publication.
    - Any new CLI fixture binaries for downstream backend mapping seams.
- **Acceptance criteria**:
  - Helper tests cover: non-object, unknown key, missing `dirs`, non-array `dirs`, array bounds, non-string entries, trimmed empty entries, byte-length bounds, relative resolution, lexical normalization, dedup order preservation, missing path rejection, non-directory rejection, absence semantics, and safe message selection with no raw path leakage.
  - Codex and Claude direct-policy tests pin that absent input yields `Vec::new()` and valid relative paths resolve through the backend-owned effective cwd ladder.
  - The test suite makes it difficult for SEAM-3/4 to reintroduce raw-payload parsing or divergent cwd precedence.
- **Dependencies**:
  - `S1`
  - `S2`
  - `AD-C02`
  - `AD-C03`
  - `AD-C07`
- **Verification**:
  - `cargo test -p agent_api backend_harness::normalize`
  - `cargo test -p agent_api codex`
  - `cargo test -p agent_api claude_code`
- **Rollout/safety**:
  - Tests only. This slice is the guardrail that lets later seams move faster without reopening normalization semantics.

## Atomic Tasks

#### S3.T1 — Expand the helper-level conformance matrix in `backend_harness`

- **Outcome**: the seam-owned helper has exhaustive regression coverage for every normalization rule and safe-message branch promised by `AD-C02`.
- **Inputs/outputs**:
  - Input: helper implementation from `S1`.
  - Output: expanded tests in `crates/agent_api/src/backend_harness/normalize/tests.rs`.
- **Implementation notes**:
  - Group tests by failure surface: top-level object shape, `dirs` container shape, per-entry validation, and filesystem/path normalization behavior.
  - Use tempdir fixtures for relative resolution and lexical normalization cases.
  - Assert on exact safe message strings and explicitly prove that rejected raw path text never appears in `err.to_string()`.
- **Acceptance criteria**:
  - Every verification bullet from the seam brief is represented in helper tests.
  - The test corpus proves both success-path normalization and failure-path redaction.
- **Test notes**:
  - Prefer table-driven helpers where it reduces repetition, but keep each safe-message class readable.
- **Risk/rollback notes**:
  - None; this is pure regression coverage.

Checklist:
- Implement: add the full helper test matrix in `crates/agent_api/src/backend_harness/normalize/tests.rs`.
- Test: malformed-shape, bounds, resolution, normalization, dedup, and filesystem cases.
- Validate: assert `!err.to_string().contains(raw_path)` for every filesystem-failure fixture.
- Cleanup: keep helper tests independent from backend-specific argv behavior.

#### S3.T2 — Add backend handoff regressions for Codex and Claude policy extraction

- **Outcome**: backend-local tests pin the exact handoff shape `SEAM-3/4` depend on: normalized `Vec<PathBuf>`, empty-vector absence semantics, and consistent effective cwd precedence.
- **Inputs/outputs**:
  - Input: policy adoption from `S2`.
  - Output: direct-policy tests under `crates/agent_api/src/backends/codex/tests/` and `crates/agent_api/src/backends/claude_code/tests/`.
- **Implementation notes**:
  - For Codex, assert that `validate_and_extract_policy(...)` attaches normalized `add_dirs` and that invalid add-dir input fails before any fork-specific handling.
  - For Claude, assert that `validate_and_extract_policy(...)` attaches normalized `add_dirs` and that run-start cwd participates only as the last fallback.
  - Keep these tests scoped to policy extraction; argv placement stays in SEAM-3/4.
- **Acceptance criteria**:
  - Both backends have direct tests covering absence, relative resolution precedence, and safe invalid propagation.
  - The tests reference `policy.add_dirs`, not raw extension payload inspection.
- **Test notes**:
  - Place tests alongside existing direct-policy modules where possible so the seam stays easy to audit.
- **Risk/rollback notes**:
  - None; this is regression coverage for the handoff contract only.

Checklist:
- Implement: add Codex direct-policy tests for empty-vector absence, cwd precedence, and invalid-before-fork behavior.
- Implement: add Claude direct-policy tests for empty-vector absence, cwd precedence, and invalid propagation.
- Test: run targeted backend test modules after adding the new cases.
- Validate: use `rg -n "agent_api\\.exec\\.add_dirs\\.v1" crates/agent_api/src/backends` to confirm parsing stays in policy extraction only.
- Cleanup: keep SEAM-3/4 mapping assertions out of these tests.
