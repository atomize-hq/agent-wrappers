---
seam_id: SEAM-5
status: landed
closeout_version: v1
seam_exit_gate:
  source_ref: ../threaded-seams/seam-5-tests/slice-3-seam-exit-gate.md
  status: passed
  promotion_readiness: ready
basis:
  currentness: current
  upstream_closeouts:
    - seam-1-closeout.md
    - seam-2-closeout.md
    - seam-3-closeout.md
    - seam-4-closeout.md
  required_threads:
    - THR-01
    - THR-02
    - THR-03
    - THR-04
    - THR-05
  stale_triggers:
    - capability matrix regeneration is deferred from advertising changes
gates:
  post_exec:
    landing: passed
    closeout: passed
open_remediations: []
---

# Closeout - SEAM-5 Tests

## Seam-exit gate record

- **Source artifact**: `../threaded-seams/seam-5-tests/slice-3-seam-exit-gate.md`
- **Landed evidence**:
  - SEAM-5A (validation ordering + safe template regression suite): commit `8b21444`
    - `crates/agent_api/src/backend_harness/normalize/tests.rs`
  - SEAM-5B (backend mapping + runtime rejection parity suites): already present in repo; validated during SEAM-5 execution
    - Codex:
      - `crates/agent_api/src/backends/codex/tests/model_mapping.rs`
      - `crates/agent_api/src/backends/codex/tests/policy_model_override.rs`
      - `crates/agent_api/src/backends/codex/tests/model_runtime_rejection.rs`
    - Claude Code:
      - `crates/agent_api/src/backends/claude_code/tests/model_mapping.rs`
      - `crates/agent_api/src/backends/claude_code/tests/model_runtime_rejection.rs`
  - Verification:
    - `cargo test -p agent_api --features codex --lib backend_harness::normalize::tests -- --nocapture`
    - `cargo test -p agent_api --features codex codex_ -- --nocapture`
    - `cargo test -p agent_api --features claude_code claude_ -- --nocapture`
- **Contracts published or changed**: no normative contract text changes; SEAM-5 verifies `C-03`, `C-04`, `C-05`, `C-06`, `C-07`, `C-08`, `C-09` remain satisfied.
- **Threads published / advanced**: regression coverage added/validated for `THR-01..THR-05`; thread states in `../threading.md` were already `published` (no state edits required).
- **Review-surface delta**: adds normalize-request ordering coverage for `agent_api.config.model.v1` invalid values and trim-before-map behavior; validates existing backend mapping + runtime parity suites cover the stream-open terminal `Error` requirement.
- **Planned-vs-landed delta**: SEAM-5A required new coverage (commit `8b21444`); SEAM-5B acceptance coverage was already present and was validated rather than re-authored in this seam.
- **Downstream stale triggers raised**: none.
- **Remediation disposition**: none opened.
- **Promotion blockers**: none.
- **Promotion readiness**: ready.

## Post-exec gate disposition

- **Landing gate**: passed
- **Closeout gate**: passed
- **Unresolved remediations**: none
- **Carried-forward remediations**: none
