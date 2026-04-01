---
seam_id: SEAM-3
seam_slug: codex-mapping
status: proposed
execution_horizon: active
plan_version: v1
basis:
  currentness: current
  source_seam_brief: ../../seam-3-codex-mapping.md
  source_scope_ref: ../../scope_brief.md
  upstream_closeouts:
    - ../../governance/seam-2-closeout.md
  required_threads:
    - THR-01
    - THR-02
  stale_triggers:
    - Codex builder/argv ordering contract changes
    - Codex fork transport gains model selection support
gates:
  pre_exec:
    review: pending
    contract: pending
    revalidation: passed
  post_exec:
    landing: pending
    closeout: pending
seam_exit_gate:
  required: true
  planned_location: reserved_final_slice
  status: pending
open_remediations: []
---
# SEAM-3 - Codex backend mapping (Activated)

## Seam brief (source of truth)

- See `../../seam-3-codex-mapping.md`.

## Promotion basis

- Upstream seam exit: `../../governance/seam-2-closeout.md` (seam-exit gate passed; promotion readiness ready).
- Required threads: `THR-01`, `THR-02` are published per `../../threading.md`.

## Next planning step

- Decompose this seam into `review.md` + `slice-*.md` via the v2.3 threaded seam decomposer.
