### S3b — Contract publication and focused Claude conformance tests

- **User/system value**: reviewers and SEAM-5B inherit one canonical statement of Claude model
  mapping and runtime-rejection behavior, backed by the smallest focused tests needed to localize
  regressions quickly.
- **Scope (in/out)**:
  - In:
    - Update `docs/specs/claude-code-session-mapping-contract.md` to reflect the final runtime
      rejection posture, `--model` ordering, and explicit `--fallback-model` exclusion.
    - Add the smallest focused `agent_api` and `claude_code` assertions that pin the same behavior
      described by the spec.
    - Remove or replace any stale wording in the seam-local planning docs only if it conflicts with
      the canonical spec text.
  - Out:
    - Runtime classifier or fake-Claude scenario implementation.
    - Shared helper semantics and InvalidRequest messaging owned by earlier seams.
    - Broad matrix publication or cross-backend regression closure from SEAM-5.
- **Acceptance criteria**:
  - The Claude mapping contract spec matches the landed implementation without unresolved drift.
  - Focused backend tests pin runtime parity, `--model` ordering, and the absence of universal-key
    mapping to `--fallback-model`.
  - SEAM-5B can cite these doc/test surfaces directly instead of inferring Claude behavior from
    implementation details.
- **Dependencies**:
  - `MS-C07`
  - `S2`
  - `S3a`
- **Verification**:
  - `cargo test -p agent_api claude_code`
  - `cargo test -p claude_code root_flags_argv`
  - Spec diff review against `threading.md` and the landed Claude code paths
- **Rollout/safety**:
  - Keep publication normative and concise; the pack may restate the contract but must not redefine
    it.
  - Prefer extending existing Claude-focused test files over creating new modules unless the current
    layout cannot express the assertions cleanly.

#### S3.T2 — Publish Claude contract conformance in specs and focused tests

- **Outcome**: the canonical Claude mapping doc and focused regression tests describe the final
  runtime/error posture, ordering guarantees, and fallback-model exclusion in one consistent place.
- **Files**:
  - `docs/specs/claude-code-session-mapping-contract.md`
  - `crates/agent_api/src/backends/claude_code/tests/backend_contract.rs`
  - `crates/agent_api/src/backends/claude_code/tests/mapping.rs`
  - `crates/claude_code/tests/root_flags_argv.rs`

Checklist:
- Implement:
  - publish the final runtime-rejection parity language in the canonical Claude spec
  - extend only the focused backend/argv assertions needed to pin the published behavior
  - keep `--fallback-model` exclusion explicit anywhere `--model` ordering is described
- Test:
  - run the focused Claude backend/runtime tests in `agent_api`
  - run the focused `root_flags_argv` coverage in `claude_code`
- Validate:
  - diff the spec language against the final code paths and `MS-C07` wording
  - confirm the focused tests match the documented behavior without introducing redundant coverage
