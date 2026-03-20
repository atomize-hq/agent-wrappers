### S3a — Runtime rejection translation core

- **User/system value**: narrows late Codex model rejection onto one safe backend-error path, so already-open streams and final completion resolve with the same pinned behavior.
- **Scope (in/out)**:
  - In:
    - classify the runtime model-rejection case after `thread.started`
    - preserve safe redaction while translating that case into `AgentWrapperError::Backend { message }`
    - emit exactly one terminal `AgentWrapperEventKind::Error` event before closure when the stream is already open
    - add or finalize the dedicated fake scenario hook needed to exercise the path end to end
  - Out:
    - focused Codex regression tests outside the minimum scenario needed to exercise this path
    - normative spec publication
- **Acceptance criteria**:
  - runtime rejection after `thread.started` maps to a safe backend error rather than raw transport leakage or a generic non-zero exit
  - completion and the terminal `Error` event use the same safe message
  - no raw model id, stdout, or stderr leaks through the translation path
  - the fake scenario `model_runtime_rejection_after_thread_started` exists and drives this case deterministically
- **Dependencies**:
  - `MS-C04` backend-owned runtime rejection contract
  - landed `S1` exec/resume mapping so the request can carry a valid runtime-selected model
  - landed `S2` fork rejection posture so fork semantics stay out of this path
- **Verification**:
  - targeted scenario execution or test coverage proving `thread.started` precedes the terminal failure
  - targeted runtime assertions for error-event/completion parity and redaction
- **Rollout/safety**:
  - keep detection narrow to model-rejection signals only
  - do not broaden catch-all non-zero exit handling in this sub-slice

#### S3a.T1 — Add the dedicated fake runtime-rejection scenario

- **Outcome**: the fake Codex scenario binary can deterministically emit `thread.started` and then the runtime model-rejection failure needed for harness testing.
- **Files**:
  - `crates/agent_api/src/bin/fake_codex_stream_exec_scenarios_agent_api.rs`

Checklist:
- Implement:
  - add or finalize `model_runtime_rejection_after_thread_started`
  - keep the scenario payload scoped to the late-rejection case only
- Test:
  - exercise the scenario through the existing fake-Codex path
- Validate:
  - confirm the scenario produces `thread.started` before the terminal failure

#### S3a.T2 — Translate the late rejection in the harness/exec runtime path

- **Outcome**: Codex harness and runtime code convert the dedicated late-rejection signal into one safe backend-error path with event/completion parity.
- **Files**:
  - `crates/agent_api/src/backends/codex/exec.rs`
  - `crates/agent_api/src/backends/codex/harness.rs`
  - `crates/agent_api/src/backend_harness/runtime.rs`

Checklist:
- Implement:
  - detect the dedicated runtime model-rejection case narrowly
  - emit exactly one terminal `Error` event when the stream is open
  - resolve completion with the same safe backend message
- Test:
  - add only the minimum targeted runtime coverage needed to prove parity on this path
- Validate:
  - review message formatting and redaction so no raw model id/stdout/stderr escapes
  - confirm unrelated Codex failures do not collapse onto this translation
