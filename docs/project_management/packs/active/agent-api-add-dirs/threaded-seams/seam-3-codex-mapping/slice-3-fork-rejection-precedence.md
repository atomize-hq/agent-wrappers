### S3 — Fork rejection precedence and no-request boundary

- **User/system value**: preserves the current Codex fork truth without silently dropping accepted
  directories by rejecting the unsupported combination early, safely, and identically for both fork
  selector branches.
- **Scope (in/out)**:
  - In:
    - Detect the combination of `policy.fork.is_some()` with a non-empty normalized add-dir list.
    - Return the pinned backend error
      `AgentWrapperError::Backend { message: "add_dirs unsupported for codex fork" }` before any
      `thread/list`, `thread/fork`, or `turn/start` request is sent.
    - Preserve validation precedence: malformed or invalid add-dir inputs still fail earlier as
      `InvalidRequest` during `S1`.
    - Keep `docs/specs/codex-app-server-jsonrpc-contract.md` aligned with the implemented fork
      rejection truth.
  - Out:
    - Exec/resume mapping (S2).
    - Shared normalizer semantics and invalid-request templates (SEAM-1/2).
    - Pack-level selector-branch coverage and integration closeout (SEAM-5).
- **Acceptance criteria**:
  - Accepted add-dir inputs combined with fork selector `"last"` or selector `"id"` fail with the
    pinned safe backend message.
  - That failure happens before any `thread/list`, `thread/fork`, or `turn/start` request.
  - No user-visible event stream item is emitted on this rejection path because no run handle is
    returned.
  - Invalid payloads, bounds failures, missing paths, and non-directory paths still fail earlier as
    `InvalidRequest`, proving validation precedence over the fork-only backend rejection.
- **Dependencies**:
  - `S1.T2` for typed `policy.add_dirs`
  - AD-C08 in `docs/specs/codex-app-server-jsonrpc-contract.md`
  - AD-C03 and AD-C04 from SEAM-1
- **Verification**:
  - `cargo test -p agent_api codex`
  - Full selector-branch and no-request-boundary coverage is owned by SEAM-5.
- **Rollout/safety**:
  - This slice keeps the fork path fail-closed until a future app-server contract revision exposes
    a real wire field for add-dir transport.

#### S3.T1 — Short-circuit accepted add-dir fork requests before app-server startup work

- **Outcome**: Codex fork runs reject accepted add-dir inputs immediately at the backend boundary,
  after validation succeeds but before any app-server request can be emitted.
- **Inputs/outputs**:
  - Input: `policy.fork` and `policy.add_dirs` from `S1.T2`; AD-C08 from `threading.md`.
  - Output:
    - `crates/agent_api/src/backends/codex/harness.rs`
    - `crates/agent_api/src/backends/codex/fork.rs` only if minor shaping is needed for clearer
      unreachable assumptions
- **Implementation notes**:
  - Keep the rejection in the harness/backend boundary that decides between exec/resume and fork
    flows so selector `"last"` and selector `"id"` share exactly the same pre-request boundary.
  - Use the pinned safe backend message verbatim: `add_dirs unsupported for codex fork`.
  - Do not return a run handle or emit any startup failure event stream item for this path.
- **Acceptance criteria**:
  - Accepted add-dir + fork is rejected with the pinned backend message.
  - Both selector branches hit the same rejection path before app-server calls.
  - Invalid add-dir payloads still fail earlier during policy extraction as `InvalidRequest`.
- **Test notes**:
  - Run `cargo test -p agent_api codex`.
  - SEAM-5 will add the explicit `"last"` and `"id"` no-request-boundary assertions.
- **Risk/rollback notes**:
  - Low risk; this is a fail-closed path that prevents silent capability drift.

Checklist:
- Implement: reject `policy.fork.is_some() && !policy.add_dirs.is_empty()` before entering the
  fork spawn path.
- Test: `cargo test -p agent_api codex`.
- Validate: confirm the rejection path cannot emit `thread/list`, `thread/fork`, or `turn/start`.
- Cleanup: keep the pinned message literal centralized if there is already a Codex fork rejection
  constant.

#### S3.T2 — Pin the fork rejection precedence in the app-server contract doc

- **Outcome**: the fork-only Codex contract doc makes the unsupported add-dir-on-fork behavior and
  precedence rules explicit enough for SEAM-5 to pin without inference.
- **Inputs/outputs**:
  - Input: AD-C08 from `threading.md`; the implemented fork rejection boundary from `S3.T1`.
  - Output:
    - `docs/specs/codex-app-server-jsonrpc-contract.md`
- **Implementation notes**:
  - Keep the doc explicit that:
    - accepted add-dir payloads on fork are rejected as backend errors,
    - malformed or invalid payloads fail earlier as `InvalidRequest`,
    - neither path may send `thread/list`, `thread/fork`, or `turn/start`.
  - Update the doc in the same change as the code if the exact rejection locus or phrasing moves.
- **Acceptance criteria**:
  - The app-server contract doc matches the implemented rejection boundary exactly.
  - There is no remaining ambiguity about validation-vs-rejection precedence.
- **Test notes**:
  - No standalone doc-only test; the doc exists so SEAM-5 can write deterministic assertions from
    one authoritative source.
- **Risk/rollback notes**:
  - Low risk; the main risk is allowing drift between the code and the pinned contract.

Checklist:
- Implement: update `docs/specs/codex-app-server-jsonrpc-contract.md` in lockstep with the fork
  rejection code.
- Test: `cargo test -p agent_api codex`.
- Validate: confirm the doc names the pinned backend message and the no-request boundary.
- Cleanup: remove any wording that implies Codex fork can silently ignore accepted add-dir inputs.
