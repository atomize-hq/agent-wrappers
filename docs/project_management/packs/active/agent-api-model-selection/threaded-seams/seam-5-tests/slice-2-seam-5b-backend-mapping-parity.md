---
slice_id: S2
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
  - THR-04
  - THR-05
contracts_produced: []
contracts_consumed:
  - C-02
  - C-04
  - C-06
  - C-07
  - C-09
open_remediations: []
candidate_subslices: []
---
### S2 - SEAM-5B: Backend mapping + runtime rejection parity suite

- **User/system value**: makes mapping drift visible immediately by pinning argv ordering and runtime rejection parity for both built-in backends.
- **Acceptance criteria**:
  - Codex exec/resume mapping emits exactly one `--model <trimmed-id>` and ordering matches contract docs
  - Codex fork rejects accepted model overrides pre-spawn with the pinned safe `Backend` error
  - Claude Code mapping emits exactly one `--model <trimmed-id>` and ordering/fallback exclusion matches contract docs
  - runtime model rejection after the stream starts yields a safe/redacted `Backend` error and exactly one terminal `Error` event with the same message
- **Verification**: targeted backend tests under `crates/agent_api/src/backends/codex/tests/**` and `crates/agent_api/src/backends/claude_code/tests/**`.

