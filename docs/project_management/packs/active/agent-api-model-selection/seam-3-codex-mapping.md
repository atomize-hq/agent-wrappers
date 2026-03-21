# SEAM-3 — Codex backend mapping

- **Name**: Codex backend mapping
- **Type**: capability
- **Goal / user value**: Make `agent_api.config.model.v1` reliably drive Codex model selection through the existing
  Codex builder/argv path while preserving safe error behavior when Codex rejects the requested model at runtime.
- **Contract registry cross-refs**: MS-C02, MS-C04, MS-C06, MS-C09 (see `threading.md`)
- **Scope**
  - In:
    - consume the normalized effective model id from SEAM-2
    - map present valid value to Codex exec/resume `--model <trimmed-id>`
    - preserve absence behavior by omitting `--model`
    - translate runtime model rejection into safe `AgentWrapperError::Backend`
    - pin the current Codex fork rejection path for accepted model-selection inputs
    - ensure already-open streams emit one terminal `Error` event with the safe message before closing
  - Out:
    - capability advertising / parser ownership
    - model registry or preflight catalog validation
    - unrelated Codex policy or sandbox semantics
- **Primary interfaces (contracts)**
  - Inputs:
    - normalized model selection contract from SEAM-2
    - Codex builder support in `crates/codex/src/builder/mod.rs`
    - run/event lifecycle guarantees from the backend harness
  - Outputs:
    - Codex exec/resume mapping emits `--model <trimmed-id>` when requested
    - Codex fork rejects accepted model-selection inputs before any app-server request with
      `AgentWrapperError::Backend { message: "model override unsupported for codex fork" }`
    - safe/redacted backend error translation for runtime rejection
- **Key invariants / rules**:
  - exactly one `--model` mapping when the key is present and valid
  - no `--model` emission when the key is absent
  - no additional semantics inferred from the model key
  - raw backend stderr must not leak into consumer-facing `Backend` messages
- **Dependencies**
  - Blocks:
    - SEAM-5
  - Blocked by:
    - SEAM-1
    - SEAM-2
- **Touch surface**:
  - `crates/agent_api/src/backends/codex/harness.rs`
  - `crates/agent_api/src/backends/codex/exec.rs`
  - `crates/agent_api/src/backends/codex/fork.rs`
  - `crates/codex/src/builder/mod.rs`
  - `crates/agent_api/src/backend_harness/runtime.rs`
  - `docs/specs/codex-streaming-exec-contract.md`
  - `docs/specs/codex-app-server-jsonrpc-contract.md`
- **Verification**:
  - argv/builder tests prove trimmed valid input maps to Codex `--model`
  - argv layout tests prove the placement rule from `docs/specs/codex-streaming-exec-contract.md`:
    wrapper-owned CLI overrides are applied before `--model <trimmed-id>`, and any accepted capability-guarded
    `--add-dir` emission appears only after that `--model` pair for both exec and resume flows
  - absence tests prove no `--model` is emitted
  - fork tests prove accepted model-selection inputs are rejected before `thread/list` / `thread/fork` /
    `turn/start` with the pinned safe backend message
  - runtime-rejection coverage uses
    `crates/agent_api/src/bin/fake_codex_stream_exec_scenarios_agent_api.rs` with a dedicated
    `model_runtime_rejection_after_thread_started` scenario that emits `thread.started` before the failure
  - runtime rejection tests prove completion resolves as safe `Backend` error and event stream closes with one terminal
    `Error` event when applicable, using the same safe message in both surfaces and no raw model-id/stderr leakage
- **Risks / unknowns**
  - Risk:
    - Codex may reject a syntactically valid model late in the run path, after stream setup
  - De-risk plan:
    - pin translation + terminal-event behavior in tests using the dedicated fake-codex midstream runtime-rejection
      scenario rather than live CLI catalogs
- **Rollout / safety**:
  - merge only with end-to-end tests covering both exec-only and stream-open failure paths that can observe safe error translation
