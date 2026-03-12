# SEAM-5 — Tests

- **Name**: add-dir regression coverage
- **Type**: risk
- **Goal / user value**: prove the same add-dir semantics hold across validation, capability
  advertising, argv mapping, and session flows.

## Scope

- In:
  - Shared normalizer tests.
  - Backend capability tests.
  - Backend argv-shape tests.
  - Effective-working-directory resolution tests.
  - Missing/non-directory failure tests.
  - Resume/fork parity tests.
  - Safe error-message tests that prove raw path values are not leaked.
- Out:
  - End-to-end live CLI smoke tests.

## Primary interfaces (contracts)

- **Validation coverage contract**
  - **Inputs**:
    - malformed or ambiguous `agent_api.exec.add_dirs.v1` payloads
  - **Outputs**:
    - exact safe `InvalidRequest` templates with no raw path leakage

- **Backend mapping coverage contract**
  - **Inputs**:
    - accepted normalized add-dir list
  - **Outputs**:
    - Codex repeated-pair argv and Claude single-group argv are both pinned

- **Session parity coverage contract**
  - **Inputs**:
    - accepted add-dir list on resume/fork requests
  - **Outputs**:
    - Claude flows honor the list with the pinned argv placement
    - Codex fork takes the pinned safe backend rejection path before any app-server request

- **Capability publication contract**
  - **Inputs**:
    - built-in backend capability ids after implementation
  - **Outputs**:
    - `docs/specs/universal-agent-api/capability-matrix.md` is regenerated and includes
      `agent_api.exec.add_dirs.v1` for both built-in backends

## Key invariants / rules

- Tests must check both presence and absence semantics.
- Tests must cover directories outside the working directory to guard against accidental
  containment logic.
- Tests must assert dedup behavior after normalization, not before.
- Tests must assert the exact safe InvalidRequest templates, not just “contains” matches.

## Dependencies

- Blocks: none
- Blocked by: SEAM-2/3/4

## Touch surface

- `crates/agent_api/src/backend_harness/normalize/tests.rs`
- `crates/agent_api/src/backends/codex/tests/**`
- `crates/agent_api/src/backends/claude_code/tests/**`

## Verification

- Targeted runs while iterating:
  - `cargo test -p agent_api`
- Full gate before merge:
  - `cargo run -p xtask -- capability-matrix`
  - `make test`
  - `make preflight`

## Risks / unknowns

- **Risk**: tests may accidentally pin backend-local implementation details instead of the shared
  contract.
- **De-risk plan**: organize tests around the contract registry in `threading.md`, with backend
  tests only asserting backend-specific argv shape, the pinned Codex fork rejection boundary, and
  capability publication.

## Rollout / safety

- No seam is done until regression coverage exists for both built-in backends and the shared
  normalizer, and the generated capability matrix has been refreshed in the same change.
