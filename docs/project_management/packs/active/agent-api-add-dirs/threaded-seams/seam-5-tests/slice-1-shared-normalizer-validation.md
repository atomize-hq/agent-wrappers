### S1 — Shared normalization + safe validation conformance

- **User/system value**: catches the most likely contract drift early by pinning one shared source
  of truth for trimming, resolution, dedup, exact safe invalid messages, and
  effective-working-directory handoff before backend-specific argv assertions are layered on top.
- **Scope (in/out)**:
  - In:
    - `normalize_add_dirs_v1(...)` tests for:
      - key absence returning `Ok(Vec::new())`,
      - Unicode-whitespace trimming,
      - relative path resolution against the effective working directory,
      - lexical normalization + dedup-after-normalization,
      - directories outside the working directory remaining legal,
      - missing/non-directory rejection with exact safe templates,
      - no raw path leakage in user-visible errors.
    - Backend policy extraction tests that prove Codex and Claude pass the already-selected
      effective working directory into the shared helper rather than re-reading raw extension data
      later.
  - Out:
    - backend argv ordering and selector-branch coverage (handled in `S2`).
    - post-handle runtime rejection parity (handled in `S3`).
- **Acceptance criteria**:
  - Shared-helper tests pin AD-C02/AD-C03/AD-C07 exactly, including exact safe
    `InvalidRequest` message templates.
  - Backend policy tests show the same relative add-dir input resolves against the selected
    effective working directory for both built-in backends.
  - No test leaks a raw path sentinel through a user-visible error string.
- **Dependencies**:
  - SEAM-2 shared normalizer implementation (`AD-C02`).
  - `docs/specs/universal-agent-api/extensions-spec.md` and `threading.md` for the exact safe
    message and normalization rules (`AD-C03`, `AD-C07`).
- **Verification**:
  - `cargo test -p agent_api`
- **Rollout/safety**:
  - Test-only slice. Safe to land before backend mapping seams finish because it targets the shared
    helper and backend policy handoff only.

#### S1.T1 — Add shared normalizer contract tests for add-dir semantics

- **Outcome**: `backend_harness::normalize::normalize_add_dirs_v1(...)` is pinned to the exact
  add-dir contract instead of a looser “contains” match.
- **Inputs/outputs**:
  - Input: `AD-C02`, `AD-C03`, and `AD-C07` from `threading.md` plus the seam brief invariants.
  - Output: new or expanded tests in
    `crates/agent_api/src/backend_harness/normalize/tests.rs` covering:
    - absence returns `Ok(Vec::new())`,
    - trim + resolve + lexical normalize + dedup preserving first occurrence order,
    - legal directories outside the working directory,
    - missing/non-directory failures as exact safe templates,
    - leak sentinels never appearing in the error text.
- **Implementation notes**:
  - Use dedicated directory fixtures so the tests can distinguish pre-normalization duplicates from
    post-normalization duplicates.
  - Assert exact error strings, not substring matches.
- **Acceptance criteria**:
  - The test set fails if any implementation echoes the raw path or changes the safe
    `InvalidRequest` template.
- **Test notes**:
  - Run: `cargo test -p agent_api backend_harness::normalize`.
- **Risk/rollback notes**:
  - None. This is a pure conformance test task.

Checklist:
- Implement: add helper tests for absence, normalization, missing/non-directory failures, and leak
  sentinels.
- Test: `cargo test -p agent_api backend_harness::normalize`.
- Validate: confirm the asserted strings exactly match the safe templates from `AD-C03`.
- Cleanup: remove any redundant pre-normalization assertions that duplicate the contract at the
  wrong layer.

#### S1.T2 — Add backend policy handoff tests for effective-working-directory resolution

- **Outcome**: Codex and Claude backend tests prove relative add-dir inputs are resolved from the
  already-chosen effective working directory during `validate_and_extract_policy(...)`, not later
  during spawn or argv assembly.
- **Inputs/outputs**:
  - Input: `AD-C02` effective-working-directory handoff rule from `threading.md`.
  - Output: targeted tests under:
    - `crates/agent_api/src/backends/codex/tests/**`
    - `crates/agent_api/src/backends/claude_code/tests/**`
    asserting request/default/run-start precedence yields the expected normalized `Vec<PathBuf>` for
    relative add-dir inputs.
- **Implementation notes**:
  - Keep these tests at the policy-extraction layer so they pin the handoff boundary without
    re-testing argv placement.
  - Prefer one Codex case and one Claude case that make the effective-working-directory choice
    observable.
- **Acceptance criteria**:
  - A regression that resolves relatives against the wrong directory fails before `S2` argv tests
    even run.
- **Test notes**:
  - Run: `cargo test -p agent_api --all-features`.
- **Risk/rollback notes**:
  - None. Test-only coverage.

Checklist:
- Implement: add one deterministic Codex handoff test and one deterministic Claude handoff test.
- Test: `cargo test -p agent_api --all-features`.
- Validate: confirm the tests assert the policy-layer normalized paths rather than spawn argv.
- Cleanup: keep helper semantics asserted in `S1.T1`, not duplicated here.
