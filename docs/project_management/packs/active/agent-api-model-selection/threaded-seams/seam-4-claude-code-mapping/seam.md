---
seam_id: SEAM-4
seam_slug: claude-code-mapping
status: closed
execution_horizon: future
plan_version: v1
basis:
  currentness: current
  source_seam_brief: ../../seam-4-claude-code-mapping.md
  source_scope_ref: ../../scope_brief.md
  upstream_closeouts:
    - ../../governance/seam-2-closeout.md
    - ../../governance/seam-3-closeout.md
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
# SEAM-4 - Claude Code backend mapping (Activated)

## Seam brief (source of truth)

- See `../../seam-4-claude-code-mapping.md`.

## Promotion basis

- Upstream seam exit: `../../governance/seam-3-closeout.md` (seam-exit gate passed; promotion readiness ready).
- Required threads: `THR-01`, `THR-02` are published per `../../threading.md`.

## Next planning step

- Execute `slice-*.md` sequentially (S1..S4), then complete the dedicated `seam-exit-gate` slice.
*** Add File: docs/project_management/packs/active/agent-api-model-selection/threaded-seams/seam-4-claude-code-mapping/review.md
---
seam_id: SEAM-4
review_phase: pre_exec
execution_horizon: active
basis_ref: seam.md#basis
---
# Review Bundle - SEAM-4 Claude Code backend mapping

This artifact feeds `gates.pre_exec.review`.
`../../review_surfaces.md` is pack orientation only.

## Falsification questions

- Can any Claude Code flow still drop an accepted model id silently (especially for session/resume flows)?
- Can the universal model-selection key map to `--fallback-model` or any other secondary override?
- Can argv ordering drift so `--model <trimmed-id>` appears after `--add-dir`, session flags, or `--fallback-model`?

## Pre-exec findings

None yet.

## Pre-exec gate disposition

- **Review gate**: pending
- **Contract gate**: pending
- **Revalidation gate**: passed (SEAM-1/SEAM-2/SEAM-3 closeouts published)
- **Opened remediations**: none
*** Add File: docs/project_management/packs/active/agent-api-model-selection/threaded-seams/seam-4-claude-code-mapping/slice-1-model-handoff.md
---
slice_id: S1
seam_id: SEAM-4
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
  - THR-05
contracts_produced:
  - C-07
contracts_consumed:
  - C-02
  - C-09
open_remediations: []
candidate_subslices: []
---
### S1 - Claude model handoff and argv mapping

- **User/system value**: makes model selection work for the Claude Code print/session flows via the existing request/argv path, with deterministic ordering and without any new raw parsing.
- **Scope (in/out)**:
  - In:
    - consume typed `Option<String>` from SEAM-2 (`C-09`)
    - thread it into Claude Code request/build mapping
    - prove exactly one `--model <trimmed-id>` emission and correct ordering
    - explicitly exclude `--fallback-model` from this universal key
  - Out:
    - capability advertising / matrix publication (SEAM-2)
- **Acceptance criteria**:
  - `Some(id)` emits exactly one `--model <id>` pair
  - `None` emits no `--model`
  - ordering follows `docs/specs/claude-code-session-mapping-contract.md`
  - no raw parse sites exist outside SEAM-2's helper
- **Dependencies**: `THR-02` (typed helper), `C-09`, `C-02`
- **Verification**: targeted argv tests for print/session ordering + fallback exclusion

#### S1.T1 - Plumb typed model selection into Claude request/argv calls

- **Outcome**: Claude request/build path consumes `Option<String>` and emits `--model <trimmed-id>` only when `Some`.
- **Thread/contract refs**: `THR-05`, `C-09`, `C-07`
- **Acceptance criteria**: mapping code never inspects raw `request.extensions`.
*** Add File: docs/project_management/packs/active/agent-api-model-selection/threaded-seams/seam-4-claude-code-mapping/slice-2-print-session-argv-conformance.md
---
slice_id: S2
seam_id: SEAM-4
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
  - THR-05
contracts_produced:
  - C-07
contracts_consumed:
  - C-02
  - C-07
  - C-09
open_remediations: []
candidate_subslices: []
---
### S2 - Print/session argv conformance (ordering + fallback exclusion)

- **User/system value**: prevents drift by pinning argv ordering and explicitly proving that the universal key does not map to `--fallback-model`.
- **Acceptance criteria**:
  - `--model <trimmed-id>` appears in the root-flags region, before any `--add-dir` group, session-selector flags, `--fallback-model`, and the final prompt token
  - the universal key never maps to `--fallback-model`
- **Verification**: focused tests that inspect the emitted argv shape for both print + session flows.
*** Add File: docs/project_management/packs/active/agent-api-model-selection/threaded-seams/seam-4-claude-code-mapping/slice-3-runtime-rejection-conformance.md
---
slice_id: S3
seam_id: SEAM-4
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
  - THR-05
contracts_produced:
  - C-07
contracts_consumed:
  - C-04
  - C-09
open_remediations: []
candidate_subslices: []
---
### S3 - Runtime rejection conformance (Claude)

- **User/system value**: ensures syntactically-valid but runtime-rejected model ids fail safely and consistently (completion + terminal Error event parity) even when the stream is already open.
- **Acceptance criteria**:
  - completion error message and terminal Error event message match byte-for-byte
  - no raw model ids or stderr leaks into consumer-visible errors
- **Verification**: use a deterministic fake-claude scenario that fails after the stream begins.
*** Add File: docs/project_management/packs/active/agent-api-model-selection/threaded-seams/seam-4-claude-code-mapping/slice-4-seam-exit-gate.md
---
slice_id: S4
seam_id: SEAM-4
slice_kind: seam_exit_gate
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
  - THR-05
contracts_produced: []
contracts_consumed:
  - C-07
open_remediations: []
candidate_subslices: []
---
### S4 - Seam-exit gate (SEAM-4)

This is the dedicated final seam-exit slice for SEAM-4. It does not hide unfinished feature delivery work.

- **Purpose**: record landed Claude Code mapping truth and publish the signal SEAM-5 and downstream promotion will consume.
- **Planned landed evidence**:
  - mapping commit/PR link
  - links to argv ordering + fallback-exclusion tests
  - runtime rejection parity tests (completion + terminal Error event)
- **Contracts expected to publish or change**: `C-07` (and any Claude Code contract doc updates)
- **Threads expected to advance**: `THR-05`
- **Promotion readiness statement**:
  - downstream promotion is blocked unless SEAM-4 closeout records `seam_exit_gate.status: passed` and `promotion_readiness: ready`

Checklist:
- Validate: closeout file updated: `../../governance/seam-4-closeout.md`
- Validate: remediation log updated if needed: `../../governance/remediation-log.md`
