### S2 — Print/session argv conformance and `--fallback-model` exclusion

- **User/system value**: delivers the user-visible Claude behavior for v1: fresh print, resume, and fork flows surface the requested model through the existing print builder while staying out of fallback-model semantics and preserving the pinned argv order.
- **Scope (in/out)**:
  - In:
    - consume the typed `model: Option<String>` handoff from `S1`
    - call `ClaudePrintRequest::model(trimmed_model_id)` only when `Some(...)`
    - preserve omission semantics when `None`
    - prove ordering for fresh print, resume, and fork flows stays before any `--add-dir` group, session-selector flags, `--fallback-model`, and the final `--verbose` token
    - explicitly pin that `agent_api.config.model.v1` never maps to `.fallback_model(...)`
    - update focused spec/test surfaces that describe this mapping
  - Out:
    - runtime rejection translation after stream open (S3)
    - capability advertising/matrix work (SEAM-2)
    - new universal keys or behavior for fallback-model
- **Acceptance criteria**:
  - fresh print, resume, and fork flows emit exactly one `--model <trimmed-id>` pair when the typed handoff is `Some(...)`
  - the same flows emit zero `--model` pairs when the handoff is `None`
  - no universal-key code path calls `.fallback_model(...)` or emits `--fallback-model`
  - argv ordering matches `docs/specs/claude-code-session-mapping-contract.md`
- **Dependencies**:
  - `S1` for the shared `model: Option<String>` handoff
  - `MS-C07` Claude mapping contract
  - `docs/specs/claude-code-session-mapping-contract.md`
- **Verification**:
  - targeted Claude backend and print-builder argv tests for present/absent mapping
  - ordering assertions for resume/fork subsequences and the fallback-model exclusion
- **Rollout/safety**:
  - localized to the existing print/session builder path; no new CLI emission path is introduced
  - safe because omission semantics remain explicit and fallback behavior stays untouched

#### S2.T1 — Map typed model selection through `ClaudePrintRequest::model(...)`

- **Outcome**: accepted model-selection requests change only Claude print/session request construction and surface as exactly one `--model <trimmed-id>`.
- **Inputs/outputs**:
  - Input: `model: Option<String>` from `S1.T1`
  - Output: updates in `crates/agent_api/src/backends/claude_code/harness.rs`, `crates/claude_code/src/commands/print.rs`, and focused tests under `crates/agent_api/src/backends/claude_code/tests/` and `crates/claude_code/tests/root_flags_argv.rs`
- **Implementation notes**:
  - call `print_req.model(trimmed_model_id)` only when `Some(...)`
  - rely on `ClaudePrintRequest::argv()` for final `--model` emission rather than hand-writing argv fragments
  - keep fresh print, resume, and fork flows on the same request-construction path so one placement rule covers all three
- **Acceptance criteria**:
  - print/resume/fork emit one `--model` pair when present, zero when absent
  - no unrelated Claude permission/session semantics change
  - `--model` stays in the root-flags region before the forbidden flag groups
- **Test notes**:
  - add builder/argv layout assertions in the closest existing Claude test modules
  - include fresh print, `"last"` selector, and explicit-id selector coverage
- **Risk/rollback notes**:
  - medium risk because it touches run wiring, but scope is bounded to model handoff only

Checklist:
- Implement: thread `model: Option<String>` into Claude print/session request construction and apply `.model(...)` only for `Some(...)`.
- Test: add fresh/resume/fork argv tests covering present and absent model selection.
- Validate: inspect the final argv path rather than asserting on intermediate policy state alone.
- Cleanup: avoid any manual `--model` string assembly outside the builder/request API.

#### S2.T2 — Pin `--fallback-model` exclusion and publish argv-order conformance

- **Outcome**: the universal model key is permanently separated from Claude's fallback-model knob, and the canonical/backend test surfaces describe the exact argv ordering reviewers should expect.
- **Inputs/outputs**:
  - Input: landed mapping behavior from `S2.T1`
  - Output: updates in `docs/specs/claude-code-session-mapping-contract.md`, `crates/agent_api/src/backends/claude_code/tests/mapping.rs`, and `crates/claude_code/tests/root_flags_argv.rs`
- **Implementation notes**:
  - add negative assertions that the universal key never reaches `.fallback_model(...)` / `--fallback-model`
  - keep the spec language pinned to existing builder ordering rather than documenting ad hoc harness-specific sequencing
  - if the existing spec text already matches the final code, limit doc changes to drift cleanup or explicit verification anchors
- **Acceptance criteria**:
  - canonical Claude spec text and focused tests both pin the same ordering and exclusion rules
  - any drift that moves `--model` to the right of `--fallback-model` or the final `--verbose` token fails loudly
  - SEAM-5B can consume the published ordering/exclusion contract rather than rediscovering it
- **Test notes**:
  - extend the smallest existing test surface that already covers Claude argv ordering
  - include at least one negative case where `fallback_model` is present through another path and the universal key still only drives `--model`
- **Risk/rollback notes**:
  - low risk; this is contract publication and regression hardening

Checklist:
- Implement: update the Claude spec doc and the smallest set of focused argv tests needed to pin the behavior.
- Test: run targeted Claude mapping/root-flags tests.
- Validate: diff the spec language against the final builder and harness code paths.
- Cleanup: remove stale pack wording only if it conflicts with the canonical spec doc.
