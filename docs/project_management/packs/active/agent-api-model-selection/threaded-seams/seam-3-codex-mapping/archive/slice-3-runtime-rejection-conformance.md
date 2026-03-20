### S3 — Runtime rejection conformance and contract publication

- **User/system value**: finishes the seam by making late Codex model rejection safe and testable, so downstream tests and reviewers can rely on one pinned behavior across completion, event streaming, and canonical backend docs.
- **Scope (in/out)**:
  - In:
    - classify runtime model rejection after the stream is already open
    - translate that failure into safe `AgentWrapperError::Backend { message }`
    - emit exactly one terminal `AgentWrapperEventKind::Error` event with the same safe message before closure when a stream exists
    - update `docs/specs/codex-streaming-exec-contract.md` and `docs/specs/codex-app-server-jsonrpc-contract.md` to match the final implementation
    - add or stage the fake-Codex scenario/test hooks SEAM-5B will consume
  - Out:
    - shared validation helper behavior and InvalidRequest messaging (SEAM-2 / SEAM-1)
    - cross-backend matrix assertions (SEAM-5)
- **Acceptance criteria**:
  - late model rejection after `thread.started` resolves as a safe backend error rather than raw transport leakage or a silent non-zero exit
  - completion and final `Error` event use the same safe message
  - no raw model id, stdout, or stderr leaks through either surface
  - canonical Codex spec docs describe the final exec/resume ordering and fork rejection posture without drift
- **Dependencies**:
  - `MS-C04` backend-owned runtime rejection contract
  - `S1` for exec/resume mapping behavior
  - `S2` for the fork rejection posture that the docs must also reflect
- **Verification**:
  - fake-Codex integration scenario that emits `thread.started` before the terminal failure
  - targeted backend runtime tests for completion/error-event parity
  - spec diff review against the landed code paths
- **Rollout/safety**:
  - no rollout toggle required; this slice only narrows failures onto safe, pinned paths
  - ship with tests/docs together so reviewers can validate the behavior in one change set

#### S3.T1 — Pin runtime model-rejection translation in the Codex harness/exec path

- **Outcome**: Codex runtime rejection after stream open becomes one safe backend-error path with completion/event parity.
- **Inputs/outputs**:
  - Input: `MS-C04` and the dedicated fake scenario requirement from the seam brief
  - Output: updates in `crates/agent_api/src/backends/codex/exec.rs`, `crates/agent_api/src/backends/codex/harness.rs`, `crates/agent_api/src/backend_harness/runtime.rs` as needed, plus `crates/agent_api/src/bin/fake_codex_stream_exec_scenarios_agent_api.rs`
- **Implementation notes**:
  - detect the model-runtime-rejection case narrowly; do not collapse unrelated non-zero exits onto this path
  - preserve existing safe redaction helpers instead of surfacing raw stderr
  - ensure already-open streams emit one terminal `Error` event before closure and then resolve completion with the same message
- **Acceptance criteria**:
  - runtime rejection after `thread.started` maps to `AgentWrapperError::Backend { message }`
  - exactly one terminal `Error` event is emitted when a stream is open
  - both surfaces use the same safe message
- **Test notes**:
  - add fake-scenario coverage for `model_runtime_rejection_after_thread_started`
  - assert message parity and redaction, not just failure type
- **Risk/rollback notes**:
  - medium risk because it touches error classification; keep detection scoped to model-rejection signals only

Checklist:
- Implement: add or finalize the dedicated fake runtime-rejection scenario and the corresponding translation path.
- Test: add streaming runtime tests that observe `thread.started`, the terminal `Error` event, and backend completion parity.
- Validate: review message formatting paths to ensure no raw model id/stdout/stderr escapes.
- Cleanup: avoid broad catch-all translation logic that could mask unrelated Codex failures.

#### S3.T2 — Publish Codex contract conformance in specs and backend tests

- **Outcome**: the normative docs and focused backend tests describe and pin the final SEAM-3 behavior for reviewers and for SEAM-5B follow-on work.
- **Inputs/outputs**:
  - Input: landed behavior from `S1`, `S2`, and `S3.T1`
  - Output: updates in `docs/specs/codex-streaming-exec-contract.md`, `docs/specs/codex-app-server-jsonrpc-contract.md`, and targeted tests under `crates/agent_api/src/backends/codex/tests/`
- **Implementation notes**:
  - document exec/resume ordering around `--model`
  - document the fork pre-handle rejection posture explicitly as part of the app-server subset contract
  - keep docs normative and concise; ADRs or pack files can reference them but not override them
- **Acceptance criteria**:
  - spec docs match the final implementation without unresolved drift
  - backend tests pin the exact behavior the docs describe
  - SEAM-5B can reference these tests/docs rather than rediscovering SEAM-3 behavior
- **Test notes**:
  - add or extend `mapping.rs`, `app_server.rs`, and `backend_contract.rs` coverage rather than creating redundant test modules unless the existing organization is insufficient
- **Risk/rollback notes**:
  - low risk; this is conformance publication and regression hardening

Checklist:
- Implement: update the two Codex spec docs and the smallest set of focused backend tests needed to pin the behavior.
- Test: run targeted Codex tests that cover mapping, fork rejection, and runtime translation.
- Validate: diff the spec language against the final code paths and threading contract IDs.
- Cleanup: remove stale wording from pack docs only if it conflicts with the canonical spec docs.
