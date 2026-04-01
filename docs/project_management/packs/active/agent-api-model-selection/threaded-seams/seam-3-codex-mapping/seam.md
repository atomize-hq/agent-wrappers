---
seam_id: SEAM-3
seam_slug: codex-mapping
status: closed
execution_horizon: future
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
# SEAM-3 - Codex backend mapping (Activated)

## Seam brief (source of truth)

- See `../../seam-3-codex-mapping.md`.

## Promotion basis

- Upstream seam exit: `../../governance/seam-2-closeout.md` (seam-exit gate passed; promotion readiness ready).
- Required threads: `THR-01`, `THR-02` are published per `../../threading.md`.

## Next planning step

- Execute `slice-*.md` sequentially (S1..S4), then complete the dedicated `seam-exit-gate` slice.
