### S3 — Runtime rejection conformance and contract publication

- **User/system value**: finishes the seam by making late Claude model rejection safe and testable, so downstream tests and reviewers can rely on one pinned behavior across completion, event streaming, and canonical backend docs.
- **Scope (in/out)**:
  - In:
    - classify runtime model rejection after the stream is already open
    - translate that failure into safe `AgentWrapperError::Backend { message }`
    - emit exactly one terminal `AgentWrapperEventKind::Error` event with the same safe message before closure when a stream exists
    - keep resume/fork selection-failure behavior (`no session found` / `session not found`) distinct from model rejection
    - update `docs/specs/claude-code-session-mapping-contract.md` and focused backend tests to match the final implementation
    - add or stage the fake-Claude scenario/test hooks SEAM-5B will consume
  - Out:
    - shared validation helper behavior and InvalidRequest messaging (SEAM-2 / SEAM-1)
    - cross-backend matrix assertions (SEAM-5)
- **Acceptance criteria**:
  - late model rejection after `system init` resolves as a safe backend error rather than raw transport leakage or a silent non-zero exit
  - completion and final `Error` event use the same safe message
  - no raw model id, stdout, or stderr leaks through either surface
  - canonical Claude spec docs and focused tests describe the final runtime/error posture without drift
- **Dependencies**:
  - `MS-C04` backend-owned runtime rejection contract
  - `S1` for typed model handoff
  - `S2` for the final mapping/ordering behavior the docs must also reflect
- **Verification**:
  - fake-Claude integration scenario that emits `system init` before the terminal failure
  - targeted backend runtime tests for completion/error-event parity
  - spec diff review against the landed code paths
- **Rollout/safety**:
  - no rollout toggle required; this slice only narrows failures onto safe, pinned paths
  - ship with tests/docs together so reviewers can validate the behavior in one change set

#### S3.T1 — Pin runtime model-rejection translation in the Claude harness/event tail

- **Outcome**: Claude runtime rejection after stream open becomes one safe backend-error path with completion/event parity.
- **Inputs/outputs**:
  - Input: `MS-C04` and the dedicated fake scenario requirement from the seam brief
  - Output: updates in `crates/agent_api/src/backends/claude_code/harness.rs`, `crates/agent_api/src/backends/claude_code/mapping.rs`, `crates/agent_api/src/backend_harness/runtime.rs` as needed, plus `crates/agent_api/src/bin/fake_claude_stream_json_agent_api.rs`
- **Implementation notes**:
  - detect the model-runtime-rejection case narrowly; do not collapse resume/fork selection failures or unrelated non-zero exits onto this path
  - preserve existing safe redaction helpers instead of surfacing raw stderr or raw model ids
  - ensure already-open streams emit one terminal `Error` event before closure and then resolve completion with the same message
- **Acceptance criteria**:
  - runtime rejection after `system init` maps to `AgentWrapperError::Backend { message }`
  - exactly one terminal `Error` event is emitted when a stream is open
  - both surfaces use the same safe/redacted message
  - existing `no session found` / `session not found` behavior remains intact for selection failures
- **Test notes**:
  - add fake-scenario coverage for `model_runtime_rejection_after_init`
  - assert message parity and redaction, not just failure type
- **Risk/rollback notes**:
  - medium risk because it touches error classification; keep detection scoped to model-rejection signals only

Checklist:
- Implement: add or finalize the dedicated fake runtime-rejection scenario and the corresponding translation path.
- Test: add streaming runtime tests that observe `system init`, the terminal `Error` event, and backend completion parity.
- Validate: review message formatting paths to ensure no raw model id/stdout/stderr escapes.
- Cleanup: avoid broad catch-all translation logic that could mask unrelated Claude failures.

#### S3.T2 — Publish Claude contract conformance in specs and focused backend tests

- **Outcome**: the normative docs and focused backend tests describe and pin the final SEAM-4 behavior for reviewers and for SEAM-5B follow-on work.
- **Inputs/outputs**:
  - Input: landed behavior from `S2` and `S3.T1`
  - Output: updates in `docs/specs/claude-code-session-mapping-contract.md`, targeted tests under `crates/agent_api/src/backends/claude_code/tests/`, and any supporting argv assertions in `crates/claude_code/tests/root_flags_argv.rs`
- **Implementation notes**:
  - document runtime rejection parity and the `--fallback-model` exclusion alongside the final argv ordering
  - keep docs normative and concise; pack files can reference them but not override them
  - add the smallest focused test set that future maintainers can use to localize regressions before SEAM-5B runs broader coverage
- **Acceptance criteria**:
  - spec docs match the final implementation without unresolved drift
  - focused backend tests pin the exact behavior the docs describe
  - SEAM-5B can reference these tests/docs rather than rediscovering SEAM-4 behavior
- **Test notes**:
  - extend `backend_contract.rs`, `mapping.rs`, and `root_flags_argv.rs` only where each already matches the behavior under test; avoid redundant new modules unless existing organization is insufficient
- **Risk/rollback notes**:
  - low risk; this is conformance publication and regression hardening

Checklist:
- Implement: update the Claude spec doc and the smallest set of focused backend tests needed to pin ordering, fallback exclusion, and runtime parity.
- Test: run targeted Claude backend/runtime/root-flags tests.
- Validate: diff the spec language against the final code paths and threading contract IDs.
- Cleanup: remove stale wording from pack docs only if it conflicts with the canonical spec doc.
