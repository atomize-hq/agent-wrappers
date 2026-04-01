---
seam_id: SEAM-2
seam_slug: backend-advertising-normalization
status: closed
execution_horizon: future
plan_version: v1
basis:
  currentness: current
  source_seam_brief: ../../seam-2-backend-advertising-normalization.md
  source_scope_ref: ../../scope_brief.md
  upstream_closeouts: []
  required_threads:
    - THR-01
  stale_triggers:
    - shared helper signature or validation rules change after downstream mapping starts
gates:
  pre_exec:
    review: passed
    contract: passed
    revalidation: passed
  post_exec:
    landing: passed
    closeout: passed
seam_exit_gate:
  required: true
  planned_location: S4
  status: passed
open_remediations: []
---
# SEAM-2 - Backend advertising + normalization hook

## Seam Brief (Restated)

- **Goal / value**: ensure both built-in backends expose `agent_api.config.model.v1` consistently while enforcing a single raw-parse site and a single typed handoff (`Option<String>`) for the effective trimmed model id.
- **Type**: integration
- **Scope**
  - In:
    - implement the shared model-selection normalizer in `crates/agent_api/src/backend_harness/normalize.rs`
    - plumb the typed normalized result into backend harness surfaces so SEAM-3/4 can consume it without re-parsing
    - flip built-in capability advertising only when deterministic outcomes exist for all exposed run flows
    - regenerate `docs/specs/universal-agent-api/capability-matrix.md` in the same change as the advertising flip
  - Out:
    - backend-specific argv insertion details (SEAM-3 / SEAM-4)
    - runtime rejection translation details (SEAM-3 / SEAM-4)
- **Touch surface**:
  - `crates/agent_api/src/backend_harness/normalize.rs`
  - `crates/agent_api/src/backends/codex/backend.rs`
  - `crates/agent_api/src/backends/claude_code/backend.rs`
  - backend adapter surfaces consumed by SEAM-3/SEAM-4
- **Verification**:
  - one raw-parse site only (repo search for `agent_api.config.model.v1`)
  - unit tests for absence/non-string/empty/oversize/trimmed-success outcomes
  - capability matrix regenerated in the same change as advertising flip
- **Basis posture**:
  - Currentness: provisional
  - Upstream closeouts assumed: none
  - Required threads: `THR-01`
  - Stale triggers: shared helper changes after downstream mapping starts
- **Threading constraints**
  - Upstream blockers: `THR-01` (SEAM-1 gate record published)
  - Downstream blocked seams: `SEAM-3`, `SEAM-4`, `SEAM-5`
  - Contracts produced: `C-05`, `C-08`, `C-09`
  - Contracts consumed: `C-01`, `C-03`

## Review bundle

- `review.md` is the authoritative artifact for `gates.pre_exec.review`
- Revalidation: SEAM-1's gate record is now published with stable commit references (THR-01); `gates.pre_exec.revalidation` has passed.

## Seam-exit gate plan

- **Planned location**: `S4` (`slice-4-seam-exit-gate.md`)
- **Why this seam needs an explicit exit gate**: SEAM-2 is the handoff point where "spec text" becomes "runnable typed contract + truthful capability publication", which downstream seams and promotion should be able to trust without re-deriving.
- **Expected contracts to publish**: `C-09`, `C-05`, `C-08`
- **Expected threads to publish / advance**: `THR-02`, `THR-03`
- **Likely downstream stale triggers**:
  - helper signature changes after SEAM-3/4 starts
  - advertising flip without matrix regeneration
- **Expected closeout evidence**:
  - links to merged diff (or PR) that flips advertising and adds the shared helper tests
  - recorded `rg` output showing no extra parse sites
  - recorded `xtask capability-matrix` output committed

## Slice index

- `S1` -> `slice-1-shared-model-normalizer.md`
- `S2` -> `slice-2-backend-exposure-gates.md`
- `S3` -> `slice-3-capability-publication-and-conformance.md`
- `S4` -> `slice-4-seam-exit-gate.md`

## Governance pointers

- Pack remediation log: `../../governance/remediation-log.md`
- Seam closeout: `../../governance/seam-2-closeout.md`
