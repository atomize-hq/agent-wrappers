### S2 — Fork model rejection before app-server transport

- **User/system value**: keeps Codex deterministic across all exposed flows by turning accepted model-selection inputs on fork flows into one pinned, safe backend rejection before any unsupported app-server request can be sent.
- **Scope (in/out)**:
  - In:
    - consume the same normalized `Option<String>` handoff used by S1
    - detect accepted model-selection inputs on fork flows
    - reject those requests before `thread/list`, `thread/fork`, or `turn/start`
    - preserve the exact safe backend message `model override unsupported for codex fork`
  - Out:
    - exec/resume `--model` mapping (S1)
    - midstream/runtime rejection after a run has already started (S3)
    - capability advertising/matrix work (SEAM-2)
- **Acceptance criteria**:
  - accepted model-selection inputs on fork flows fail before any app-server request is issued
  - the surfaced error is `AgentWrapperError::Backend { message: "model override unsupported for codex fork" }`
  - the rejection path is pre-handle and safe: no raw model id, stdout, or stderr leaks
- **Dependencies**:
  - `S1.T1` for the shared `model: Option<String>` policy plumbing
  - `SEAM-2` / `MS-C09` helper contract
  - `MS-C06` Codex fork rejection posture
- **Verification**:
  - fork-focused tests assert no JSON-RPC calls occur when model selection is present
  - harness/backend tests assert the pinned backend error surfaces unchanged
- **Rollout/safety**:
  - safest possible fork posture because it fails before any remote/session mutation
  - localized to Codex fork flows only

#### S2.T1 — Thread normalized model selection into the fork request path

- **Outcome**: fork flow construction knows whether an accepted model override was requested without re-parsing request extensions.
- **Inputs/outputs**:
  - Input: `model: Option<String>` from `S1.T1`
  - Output: updates in `crates/agent_api/src/backends/codex/backend.rs`, `crates/agent_api/src/backends/codex/fork.rs`, and supporting request structs
- **Implementation notes**:
  - add `model: Option<String>` to `ForkFlowRequest`
  - keep the handoff typed and read-only; fork code should not trim or inspect raw extension JSON
- **Acceptance criteria**:
  - fork request construction receives the same typed model selection value as exec/resume
  - no new parser or string-normalization logic appears in fork code
- **Test notes**:
  - extend unit coverage around fork request construction as needed
- **Risk/rollback notes**:
  - low risk; this is plumbing only

Checklist:
- Implement: add the model field to `ForkFlowRequest` and pass it from backend/harness spawn logic.
- Test: run targeted fork/backend tests.
- Validate: confirm the fork path still compiles and uses only typed model state.
- Cleanup: keep fork request structs minimal and purpose-specific.

#### S2.T2 — Enforce the pinned pre-handle safe rejection path for fork flows

- **Outcome**: any accepted model-selection request routed to Codex fork fails early with the pinned safe backend message and no app-server side effects.
- **Inputs/outputs**:
  - Input: `model: Option<String>` on the fork flow path
  - Output: updates in `crates/agent_api/src/backends/codex/fork.rs`, `crates/agent_api/src/backends/codex/harness.rs`, and fork-related tests under `crates/agent_api/src/backends/codex/tests/app_server.rs`
- **Implementation notes**:
  - gate the rejection on `model.is_some()`
  - fail before selector resolution triggers `thread/list` and before any `thread/fork` / `turn/start` request objects are built or sent
  - reuse the existing safe backend-error shaping path instead of creating a one-off surface
- **Acceptance criteria**:
  - fork flows with model selection return the pinned safe backend error before any app-server request
  - existing fork behavior without model selection remains unchanged
- **Test notes**:
  - add tests that assert zero outbound app-server calls plus the exact safe message
  - include both `"last"` and explicit-id selectors if the current tests distinguish them
- **Risk/rollback notes**:
  - low risk: earlier failure is safer than partial fork execution

Checklist:
- Implement: short-circuit fork flows when `model.is_some()` with the pinned backend error.
- Test: add fork tests covering no-call behavior and exact error text.
- Validate: confirm no `thread/list`, `thread/fork`, or `turn/start` traffic is emitted on the rejection path.
- Cleanup: centralize the pinned message constant if the code path currently duplicates strings.
