# SEAM-5 — Tests

- **Name**: Tests
- **Type**: risk
- **Goal / user value**: Lock the universal model-selection behavior in place so future backend or spec churn cannot
  silently regress validation ordering, trimmed mapping, or backend-error safety.
- **Contract registry cross-refs**: MS-C03, MS-C04, MS-C05, MS-C06, MS-C07, MS-C08, MS-C09 (see `threading.md`)
- **Scope**
  - In:
    - SEAM-5A: R0 unsupported-capability ordering tests plus schema/bounds/InvalidRequest tests
    - SEAM-5B: trim-before-map tests
    - SEAM-5B: absence/no-argv tests
    - SEAM-5B: Codex and Claude backend mapping tests
    - SEAM-5B: runtime rejection and terminal error-event tests
    - SEAM-5B: capability-matrix freshness assertions after advertising changes
  - Out:
    - end-to-end upstream CLI compatibility tests against live model catalogs
    - speculative tests for future universal keys such as fallback-model
- **Primary interfaces (contracts)**
  - Inputs:
    - SEAM-1 through SEAM-4 contracts
    - backend harness normalize/runtime test utilities
  - Outputs:
    - regression suite covering the pinned v1 behavior, split into SEAM-5A and SEAM-5B entry criteria
    - stable failure cases for unsupported, invalid, and runtime-rejected model ids
    - assertions that the SEAM-2-owned regenerated `docs/specs/universal-agent-api/capability-matrix.md` matches the
      landed advertising change
- **Key invariants / rules**:
  - unsupported key must fail before `InvalidRequest`
  - trimming must happen before emptiness and byte-length validation
  - invalid requests must use the exact safe template `invalid agent_api.config.model.v1`
  - absence must preserve backend defaults
  - runtime rejection messages stay safe/redacted
  - stream-open failure path emits exactly one terminal `Error` event
- **Dependencies**
  - Blocks:
    - none
  - Blocked by:
    - SEAM-1 for SEAM-5A
    - SEAM-2 for SEAM-5B
    - SEAM-3 for Codex SEAM-5B cases
    - SEAM-4 for Claude SEAM-5B cases
- **Touch surface**:
  - `crates/agent_api/src/backend_harness/normalize/tests.rs`
  - `crates/agent_api/src/backends/codex/tests/**`
  - `crates/agent_api/src/backends/claude_code/tests/**`
  - any shared runtime/error translation tests under `crates/agent_api/src/backend_harness/runtime/tests/**`
  - `docs/specs/universal-agent-api/capability-matrix.md`
- **Verification**:
  - targeted `cargo test` runs cover all new cases
  - SEAM-5A asserts unsupported-before-InvalidRequest ordering and the exact safe InvalidRequest template
    `invalid agent_api.config.model.v1`
  - Codex stream-open runtime rejection uses
    `crates/agent_api/src/bin/fake_codex_stream_exec_scenarios_agent_api.rs` with scenario
    `model_runtime_rejection_after_thread_started`
  - Claude stream-open runtime rejection uses
    `crates/agent_api/src/bin/fake_claude_stream_json_agent_api.rs` with scenario
    `model_runtime_rejection_after_init`
  - runtime-rejection assertions compare the completion error message and the terminal
    `AgentWrapperEventKind::Error` message and verify that neither surface leaks raw model ids/stdout/stderr
  - `cargo run -p xtask -- capability-matrix` must be rerun in the same change that updates built-in capability sets
  - `make test` passes for the workspace
  - no existing extension-key tests regress in ordering or error type
- **Risks / unknowns**
  - Risk:
    - tests could cover argv construction only and still miss post-stream runtime rejection behavior
  - De-risk plan:
    - split SEAM-5A and SEAM-5B explicitly and use the dedicated fake-codex/fake-claude runtime-failure scenarios
      instead of live model catalogs
- **Rollout / safety**:
  - treat test coverage as merge-blocking for capability advertising
  - add focused cases before broad refactors so failures localize to one seam
