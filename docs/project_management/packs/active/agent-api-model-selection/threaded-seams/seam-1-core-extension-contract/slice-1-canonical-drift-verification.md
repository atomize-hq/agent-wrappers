---
slice_id: S1
seam_id: SEAM-1
slice_kind: delivery
execution_horizon: active
status: exec-ready
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
contracts_produced:
  - C-01
  - C-02
  - C-03
  - C-04
contracts_consumed: []
open_remediations: []
candidate_subslices: []
---
### S1 - Canonical drift verification (C-01..C-04)

- **User/system value**: ensures downstream seams implement against one canonical truth and do not inherit ambiguous or contradictory doc guidance.
- **Scope (in/out)**:
  - In:
    - compare canonical owner spec + registry entry + inherited run lifecycle/error baselines
    - detect any mismatch against ADR/pack restatements
    - if mismatch exists, update canonical specs first, then sync ADR + pack
  - Out:
    - backend code changes
- **Acceptance criteria**:
  - mismatch-free alignment across the compared sources for v1 semantics (trim/bounds/absence, invalid template, runtime rejection posture)
  - any drift is resolved with canonical-first edits
- **Dependencies**: none
- **Verification**:
  - record a repeatable comparison scope (files + headings)
  - append a pass/fail entry under `../../seam-1-core-extension-contract.md`
- **Rollout/safety**: treat unresolved drift as blocking for SEAM-2 and beyond.

#### S1.T1 - Run and record the drift comparison

- **Outcome**: a clear pass/fail result plus the exact compared sources list.
- **Thread/contract refs**: `THR-01`, `C-01..C-04`
- **Acceptance criteria**: record includes date, verifier, compared sources, and pass/fail.

Checklist:
- Validate: compared sources list matches the seam brief
- Validate: result is recorded under the pack seam brief (`../../seam-1-core-extension-contract.md`)
