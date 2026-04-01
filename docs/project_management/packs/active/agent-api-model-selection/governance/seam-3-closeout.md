---
seam_id: SEAM-3
status: landed
closeout_version: v0
seam_exit_gate:
  source_ref: "commits 653c6e8, 7b3a8a2, 3f8b649"
  status: passed
  promotion_readiness: ready
basis:
  currentness: current
  upstream_closeouts: []
  required_threads: []
  stale_triggers: []
gates:
  post_exec:
    landing: passed
    closeout: passed
open_remediations: []
---

# Closeout - SEAM-3 Codex backend mapping

## Seam-exit gate record

- **Source artifact**: `threaded-seams/seam-3-codex-mapping/slice-4-seam-exit-gate.md`; `threaded-seams/seam-3-codex-mapping/seam.md`
- **Landed evidence**: commits `653c6e8` (S1), `7b3a8a2` (S2), `3f8b649` (S3)
- **Contracts published or changed**: `C-06` (Codex mapping deterministic behavior + fork rejection + runtime rejection parity)
- **Threads published / advanced**: `THR-04` (`threading.md` updated with the commit references)
- **Review-surface delta**:
  - exec/resume mapping tests: `crates/agent_api/src/backends/codex/tests/model_mapping.rs`
  - fork pre-spawn rejection guard + tests: `crates/agent_api/src/backends/codex/tests/policy_model_override.rs`
  - runtime rejection parity tests + fake-codex scenario: `crates/agent_api/src/backends/codex/tests/model_runtime_rejection.rs`; `crates/agent_api/src/bin/fake_codex_stream_exec_scenarios_agent_api.rs`
- **Planned-vs-landed delta**: Codex backend `supported_extension_keys` and `capabilities()` advertising were intentionally not flipped in SEAM-3; SEAM-3 only makes the mapping deterministic *when the normalized typed model id is present*.
- **Downstream stale triggers raised**: none beyond the existing Codex builder/argv and fork transport triggers recorded in the seam basis
- **Remediation disposition**: none; `governance/remediation-log.md` remains unchanged
- **Promotion blockers**: none
- **Promotion readiness**: ready

## Post-exec gate disposition

- **Landing gate**: passed
- **Closeout gate**: passed
- **Unresolved remediations**: none
- **Carried-forward remediations**: none
