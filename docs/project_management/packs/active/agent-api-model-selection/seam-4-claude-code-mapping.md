---
seam_id: SEAM-4
seam_slug: claude-code-mapping
type: capability
status: closed
execution_horizon: future
plan_version: v1
basis:
  currentness: current
  source_scope_ref: scope_brief.md
  source_scope_version: v1
  upstream_closeouts:
    - governance/seam-2-closeout.md
    - governance/seam-3-closeout.md
  required_threads:
    - THR-01
    - THR-02
  stale_triggers:
    - Claude argv ordering contract changes
    - new universal keys touch fallback-model semantics
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

# SEAM-4 - Claude Code backend mapping

- **Name**: Claude Code backend mapping
- **Type**: capability
- **Goal / user value**: Make `agent_api.config.model.v1` reliably drive Claude Code print-mode model selection through
  the existing request/argv path without conflating it with Claude-specific fallback-model behavior.
- **Contract registry cross-refs**: MS-C02, MS-C04, MS-C07, MS-C09 (see `threading.md`)
- **Scope**
  - In:
    - consume the normalized effective model id from SEAM-2
    - map present valid value to Claude `--model <trimmed-id>`
    - preserve absence behavior by omitting `--model`
    - explicitly exclude `--fallback-model` and other secondary overrides from this key
    - translate runtime model rejection into safe `AgentWrapperError::Backend`
    - ensure already-open streams emit one terminal `Error` event with the safe message before closing
  - Out:
    - capability advertising / parser ownership
    - new universal key for fallback-model
    - wrapper-owned validation against Claude model catalogs
- **Primary interfaces (contracts)**
  - Inputs:
    - normalized model selection contract from SEAM-2
    - Claude request support in `crates/claude_code/src/commands/print.rs`
    - run/event lifecycle guarantees from the backend harness
  - Outputs:
    - Claude print mapping emits `--model <trimmed-id>` when requested
    - no `--fallback-model` mapping from this key
    - safe/redacted backend error translation for runtime rejection
- **Key invariants / rules**:
  - exactly one `--model` mapping when the key is present and valid
  - no `--model` emission when the key is absent
  - no `--fallback-model` emission from this universal key
  - raw backend stderr must not leak into consumer-facing `Backend` messages
- **Dependencies**
  - Blocks:
    - SEAM-5
  - Blocked by:
    - SEAM-1
    - SEAM-2
- **Touch surface**:
  - `crates/agent_api/src/backends/claude_code/harness.rs`
  - `crates/agent_api/src/backends/claude_code/mapping.rs`
  - `crates/claude_code/src/commands/print.rs`
  - `crates/agent_api/src/backend_harness/runtime.rs`
  - `docs/specs/claude-code-session-mapping-contract.md`
- **Verification**:
  - argv/request tests prove trimmed valid input maps to Claude `--model`
  - absence tests prove no `--model` is emitted
  - session argv tests prove `--model <trimmed-id>` appears before any `--add-dir` group,
    `--continue` / `--fork-session` / `--resume`, any `--fallback-model`, and the final
    `--verbose` token, matching the pinned placement rules in
    `docs/specs/claude-code-session-mapping-contract.md`
  - regression coverage spans fresh print, resume, and fork session argv flows so any
    implementation that moves `--model <trimmed-id>` to the right of the final `--verbose` token
    fails before SEAM-5 consumes the mapping
  - regression tests prove the universal key never emits `--fallback-model`
  - runtime-rejection coverage uses `crates/agent_api/src/bin/fake_claude_stream_json_agent_api.rs` with a dedicated
    `model_runtime_rejection_after_init` scenario that emits `system init` before the terminal failure
  - runtime rejection tests prove completion resolves as safe `Backend` error and event stream closes with one terminal
    `Error` event when applicable, using the same safe message in both surfaces and no raw model-id/stderr leakage
- **Risks / unknowns**
  - Risk:
    - Claude already exposes a separate fallback-model knob, which creates drift risk if the universal key is wired too
      loosely into print request construction
  - De-risk plan:
    - pin dedicated negative tests proving the universal key affects only `--model`, and drive runtime rejection
      through the fake-claude post-init failure scenario rather than live model catalogs
- **Rollout / safety**:
  - do not advertise the capability until the print mapping and exclusion tests both pass
