---
seam_id: SEAM-1
seam_slug: core-extension-contract
status: exec-ready
execution_horizon: active
plan_version: v1
basis:
  currentness: current
  source_seam_brief: ../../seam-1-core-extension-contract.md
  source_scope_ref: ../../scope_brief.md
  upstream_closeouts: []
  required_threads:
    - THR-01
  stale_triggers:
    - any canonical spec or registry delta for agent_api.config.model.v1 semantics
gates:
  pre_exec:
    review: passed
    contract: passed
    revalidation: passed
  post_exec:
    landing: pending
    closeout: pending
seam_exit_gate:
  required: true
  planned_location: S3
  status: pending
open_remediations: []
---
# SEAM-1 - Core extension key contract

## Seam Brief (Restated)

- **Goal / value**: keep `agent_api.config.model.v1` pinned to one verified canonical contract so downstream seams can implement safely against a single source of truth.
- **Type**: integration
- **Scope**
  - In:
    - verify that canonical specs + ADR + pack restatements agree on v1 semantics (C-01..C-04)
    - update canonical specs first if drift is found, then sync ADR + pack in the same change
    - publish a downstream-citable verification record with a commit/PR reference (not a local HEAD note)
  - Out:
    - backend advertising, normalization implementation, or argv wiring
- **Touch surface**:
  - `docs/specs/universal-agent-api/extensions-spec.md`
  - `docs/specs/universal-agent-api/capabilities-schema-spec.md`
  - `docs/specs/universal-agent-api/contract.md`
  - `docs/specs/universal-agent-api/run-protocol-spec.md`
  - `docs/adr/0020-universal-agent-api-model-selection.md`
  - this pack
- **Verification**:
  - For a seam that produces owned contracts, this seam's pre-exec readiness is about making the contract text concrete and synchronized across canonical sources.
  - Publication or acceptance of the owned contract artifact is recorded as evidence in the verification record and later closeout, not treated as an external prerequisite.
- **Basis posture**:
  - Currentness: current
  - Upstream closeouts assumed: none
  - Required threads: `THR-01`
  - Stale triggers: canonical spec/reg entry deltas after the verification record is published
- **Threading constraints**
  - Upstream blockers: none
  - Downstream blocked seams: `SEAM-2`, `SEAM-3`, `SEAM-4`, `SEAM-5`
  - Contracts produced: `C-01`, `C-02`, `C-03`, `C-04`
  - Contracts consumed: none (canonical docs are evidence inputs)

## Review bundle

- `review.md` is the authoritative artifact for `gates.pre_exec.review`

## Seam-exit gate plan

- **Planned location**: `S3` (`slice-3-seam-exit-gate.md`)
- **Why this seam needs an explicit exit gate**: downstream seams must be able to cite a single published verification record before implementing or advertising the capability.
- **Expected contracts to publish**: `C-01`, `C-02`, `C-03`, `C-04`
- **Expected threads to publish / advance**: `THR-01`
- **Likely downstream stale triggers**:
  - canonical spec text changes without re-running the verification pass
  - pack/ADR restatement drift after canonical changes
- **Expected closeout evidence**:
  - a verification record entry that cites a commit/PR reference
  - links to any canonical doc edits (if drift was found)

## Slice index

- `S1` -> `slice-1-canonical-drift-verification.md`
- `S2` -> `slice-2-adr-pack-sync-and-gate-publication.md`
- `S3` -> `slice-3-seam-exit-gate.md`

## Governance pointers

- Pack remediation log: `../../governance/remediation-log.md`
- Seam closeout: `../../governance/seam-1-closeout.md`
