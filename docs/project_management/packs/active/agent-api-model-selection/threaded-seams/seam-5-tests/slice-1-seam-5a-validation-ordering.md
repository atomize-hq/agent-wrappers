---
slice_id: S1
seam_id: SEAM-5
slice_kind: delivery
execution_horizon: active
status: decomposed
plan_version: v1
basis:
  currentness: current
  basis_ref: seam.md#basis
  stale_triggers: []
gates:
  pre_exec:
    review: inherited
    contract: inherited
    revalidation: inherited
  post_exec:
    landing: pending
    closeout: pending
threads:
  - THR-01
  - THR-02
contracts_produced: []
contracts_consumed:
  - C-01
  - C-02
  - C-03
  - C-04
  - C-09
open_remediations: []
candidate_subslices: []
---
### S1 - SEAM-5A: R0 + schema/InvalidRequest regression suite

- **User/system value**: locks the v1 validation and error taxonomy so callers cannot regress into unsafe message leakage or inconsistent ordering.
- **Acceptance criteria**:
  - unsupported key fails as `UnsupportedCapability` before any `InvalidRequest` work
  - all invalid `agent_api.config.model.v1` payloads fail as `InvalidRequest { message: "invalid agent_api.config.model.v1" }` without echoing the raw model id
  - trim-before-validate and trim-before-map behavior is pinned via tests
- **Verification**: targeted normalize/validation tests under `crates/agent_api/src/backend_harness/normalize/tests.rs`.

