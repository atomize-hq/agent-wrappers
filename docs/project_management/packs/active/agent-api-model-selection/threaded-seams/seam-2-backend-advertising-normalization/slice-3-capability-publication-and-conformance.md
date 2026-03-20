# S3 — Capability publication + conformance gate

- **User/system value**: Makes the public capability inventory truthful and reviewable once model-selection support is real, so hosts, reviewers, and SEAM-5 all consume the same published backend posture without stale matrix drift.
- **Scope (in/out)**:
  - In:
    - regenerate the generated capability matrix in the same change as the built-in advertising flip
    - treat matrix drift and second-parser drift as merge-blocking conformance failures
    - pin the integration validation steps that WS-INT and SEAM-5 consume
  - Out:
    - backend runtime rejection fixtures and backend-error event assertions
    - argv-order tests for `--model`, `--add-dir`, session selectors, or `--fallback-model`
- **Acceptance criteria**:
  - `docs/specs/universal-agent-api/capability-matrix.md` is regenerated in the same change that flips `agent_api.config.model.v1` advertising.
  - The generated matrix posture matches the final built-in `capabilities()` posture for Codex and Claude Code.
  - Merge validation includes a focused review that raw parsing of `agent_api.config.model.v1` still exists only in `crates/agent_api/src/backend_harness/normalize.rs`.
  - SEAM-5 can consume the published matrix and the final capability posture without special-case interpretation.
- **Dependencies**:
  - S2
  - `MS-C08`
  - the deterministic mapping outputs from `MS-C06` and `MS-C07` must already be present in the integration change that lands this slice
- **Verification**:
  - `cargo run -p xtask -- capability-matrix`
  - `cargo test -p agent_api --features codex,claude_code`
  - focused repo search for `agent_api.config.model.v1` under `crates/agent_api/src`
- **Rollout/safety**:
  - Do not hand-edit `capability-matrix.md`.
  - Treat stale matrix diffs and new raw parser sites as merge blockers, not follow-up chores.

## Atomic Tasks

#### S3.T1 — Regenerate and review the capability matrix in the same advertising-flip change

- **Outcome**: The generated matrix publishes the final built-in model-selection posture without drift from runtime capability code.
- **Inputs/outputs**:
  - Input:
    - `docs/specs/universal-agent-api/capability-matrix.md`
    - `crates/agent_api/src/backends/codex/backend.rs`
    - `crates/agent_api/src/backends/claude_code/backend.rs`
  - Output:
    - `docs/specs/universal-agent-api/capability-matrix.md`
- **Implementation notes**:
  - Run `cargo run -p xtask -- capability-matrix` in the same branch/PR that flips built-in advertising.
  - Review the diff specifically for the `agent_api.config.*` bucket and the `agent_api.config.model.v1` row.
  - Do not queue matrix regeneration as a later cleanup task; stale output blocks merge by contract.
- **Acceptance criteria**:
  - The generated matrix diff matches the final `capabilities()` posture of both built-in backends.
  - No manual edits are required after the xtask run.
- **Test notes**:
  - Pair with `cargo test -p agent_api --features codex,claude_code` before merge.
- **Risk/rollback notes**:
  - Low: generated artifact only, but required for truthful publication.

Checklist:
- Implement: run `cargo run -p xtask -- capability-matrix`.
- Test: `cargo test -p agent_api --features codex,claude_code`.
- Validate: review the generated `agent_api.config.model.v1` row against backend `capabilities()` code.
- Cleanup: commit the regenerated matrix with the advertising change.

#### S3.T2 — Enforce the single-parser and truthful-publication merge gate

- **Outcome**: The final integration review has explicit, repeatable checks for the two seam-critical invariants: one raw parser and one truthful published capability posture.
- **Inputs/outputs**:
  - Input:
    - `crates/agent_api/src/backend_harness/normalize.rs`
    - `crates/agent_api/src/backends/codex/`
    - `crates/agent_api/src/backends/claude_code/`
    - `docs/project_management/packs/active/agent-api-model-selection/threading.md`
  - Output:
    - merge validation evidence for the change set (review + command output), plus any follow-up test updates if the check fails
- **Implementation notes**:
  - Review the repo search for `agent_api.config.model.v1` and classify every hit:
    - allowed: shared constant definitions, capability advertising, tests, docs
    - forbidden: a second raw parse/validation site outside `crates/agent_api/src/backend_harness/normalize.rs`
  - Confirm the public advertising flip matches the deterministic-support evidence from SEAM-3 / SEAM-4:
    - Codex: exec/resume map, fork safe rejection
    - Claude Code: print exec/resume/fork map and no `--fallback-model` implication
  - If review reveals a mismatch, fix the code or keep advertising false; do not waive the invariant.
- **Acceptance criteria**:
  - No forbidden raw parser sites remain.
  - Public advertising is enabled only when the downstream mapping evidence is already in the same integration stack.
- **Test notes**:
  - Suggested validation command:
    - `rg -n "agent_api\\.config\\.model\\.v1" crates/agent_api/src docs/specs/universal-agent-api`
- **Risk/rollback notes**:
  - High if skipped: a second parser or early advertising flip would create spec-visible drift.

Checklist:
- Implement: run the repo search and classify every match in review.
- Test: rerun targeted backend and harness tests if any parser/advertising fix is needed.
- Validate: confirm no mapping module reads raw `request.extensions["agent_api.config.model.v1"]`.
- Cleanup: keep the final diff small enough that the single-parser rule is easy to audit.
