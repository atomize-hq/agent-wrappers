### S3b — Runtime rejection backend conformance tests

- **User/system value**: locks the final SEAM-3 backend behavior behind focused regression tests so SEAM-5B can consume stable fixtures instead of rediscovering Codex-specific edge cases.
- **Scope (in/out)**:
  - In:
    - add focused Codex backend tests that pin runtime translation parity after `S3a`
    - extend existing mapping/fork/backend-contract coverage only where needed to keep the full SEAM-3 posture reviewable in one test layer
    - prove message parity, exact error text, and zero raw leakage across the test surfaces already owned by Codex
  - Out:
    - runtime classification implementation itself
    - normative spec edits
- **Acceptance criteria**:
  - focused tests pin runtime rejection after `thread.started` across completion and terminal `Error` event surfaces
  - existing Codex test modules cover the final exec/resume mapping, fork rejection posture, and late runtime translation without creating redundant new suites
  - assertions check exact safe-message behavior and redaction, not just failure type
- **Dependencies**:
  - `S3a` runtime translation core
  - landed `S1` and `S2` behavior for mapping and fork posture
- **Verification**:
  - targeted `cargo test` runs for the touched Codex test modules
  - spot-check that SEAM-5B can reference these tests directly
- **Rollout/safety**:
  - keep changes inside existing Codex test organization unless a new helper is strictly required
  - do not edit canonical specs in this sub-slice

#### S3b.T1 — Pin runtime translation behavior in focused Codex tests

- **Outcome**: runtime rejection parity is asserted in the narrowest existing Codex test surfaces.
- **Files**:
  - `crates/agent_api/src/backends/codex/tests/backend_contract.rs`
  - `crates/agent_api/src/backends/codex/tests/app_server.rs`

Checklist:
- Implement:
  - add assertions for `thread.started` followed by one terminal `Error` event and matching backend completion message
  - keep the assertions focused on redaction and parity
- Test:
  - run targeted backend/app-server test cases for the new scenario
- Validate:
  - confirm only one terminal `Error` event is observed

#### S3b.T2 — Keep the full SEAM-3 mapping posture regression-safe in existing test modules

- **Outcome**: the final SEAM-3 behavior remains reviewable in the existing Codex test layout, covering runtime translation together with mapping and fork posture at the regression layer.
- **Files**:
  - `crates/agent_api/src/backends/codex/tests/mapping.rs`
  - `crates/agent_api/src/backends/codex/tests/backend_contract.rs`
  - `crates/agent_api/src/backends/codex/tests/app_server.rs`

Checklist:
- Implement:
  - extend the smallest set of existing tests needed to pin exec/resume `--model` behavior, fork rejection, and late runtime translation together
- Test:
  - run the targeted Codex test modules touched above
- Validate:
  - confirm there is no redundant new test module for behavior already owned by `mapping.rs`, `app_server.rs`, or `backend_contract.rs`
