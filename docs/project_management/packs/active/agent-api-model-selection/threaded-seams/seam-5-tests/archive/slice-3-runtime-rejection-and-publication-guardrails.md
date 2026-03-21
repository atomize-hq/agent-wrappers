### S3 — Runtime rejection and capability-publication guardrails

- **User/system value**: proves accepted model-selection inputs still fail safely when a backend
  rejects them after stream open, and keeps the generated capability matrix aligned with whatever
  advertising change landed.
- **Scope (in/out)**:
  - In:
    - Codex runtime rejection coverage using the dedicated fake scenario
      `model_runtime_rejection_after_thread_started`.
    - Claude runtime rejection coverage using the dedicated fake scenario
      `model_runtime_rejection_after_init`.
    - terminal `AgentWrapperEventKind::Error` conformance with the same safe message as the
      completion error.
    - redaction checks proving no raw model id, stdout, or stderr leakage.
    - capability-matrix freshness assertions tied to `cargo run -p xtask -- capability-matrix` once
      advertising changes land.
  - Out:
    - pre-spawn invalid-input behavior (covered in `S1`).
    - deterministic argv placement and no-second-parser handoff (covered in `S2`).
- **Acceptance criteria**:
  - Codex and Claude runtime-rejection tests observe at least one early status event
    (`thread.started` or `system init`) before the terminal failure.
  - Completion and final `Error` event share the same safe/redacted message.
  - Exactly one terminal `AgentWrapperEventKind::Error` is emitted for the runtime-rejection path.
  - Neither surfaced message leaks raw model ids, stdout, or stderr from the fake backends.
  - Capability-matrix assertions fail if the generated artifact is stale relative to landed
    advertising.
- **Dependencies**:
  - `MS-C04` from `SEAM-1`.
  - `MS-C05` and `MS-C08` from `SEAM-2`.
  - final runtime behavior from `SEAM-3` and `SEAM-4`.
- **Verification**:
  - `cargo test -p agent_api codex`
  - `cargo test -p agent_api claude_code`
  - `cargo run -p xtask -- capability-matrix`
  - `make test`
- **Rollout/safety**:
  - Use fake backend scenarios only. Keep publication assertions deterministic and tied to the same
    change that flips advertising.

#### S3.T1 — Codex runtime rejection test with terminal Error-event conformance

- **Outcome**: Codex midstream model rejection is pinned to the safe backend-error contract and the
  single terminal `Error` event rule.
- **Inputs/outputs**:
  - Input: `MS-C04`, `MS-C06`, and the fake Codex scenario
    `model_runtime_rejection_after_thread_started`.
  - Output: Codex runtime/error tests in the existing Codex test surface plus any needed updates to
    `crates/agent_api/src/bin/fake_codex_stream_exec_scenarios_agent_api.rs` to support a stable
    rejection fixture that:
    - emits `thread.started`,
    - rejects the chosen model after stream open,
    - includes secret sentinels in raw output so redaction is testable.
- **Implementation notes**:
  - Assert the completion error message and terminal `Error` event message are identical.
  - Keep the test focused on runtime rejection after acceptance, not pre-spawn invalid input.
- **Acceptance criteria**:
  - Exactly one terminal `AgentWrapperEventKind::Error` is emitted.
  - The surfaced message is safe/redacted and does not contain the raw model id or fake stderr.
- **Test notes**:
  - Run: `cargo test -p agent_api codex`.
- **Risk/rollback notes**:
  - Keep the fake scenario isolated so existing Codex fake-flow tests do not drift.

Checklist:
- Implement:
  - Add a Codex runtime-rejection test that waits for `thread.started` before the failure.
  - Assert identical safe messages in completion and the final `Error` event.
  - Assert exactly one terminal `Error` event and no raw model id/stdout/stderr leakage.
  - Extend the fake Codex scenario only as needed to make redaction and ordering deterministic.
- Test: `cargo test -p agent_api codex`.
- Validate: use a unique secret sentinel in fake output and assert it never surfaces.
- Cleanup: keep the scenario name stable and dedicated to this seam.

#### S3.T2 — Claude runtime rejection test with terminal Error-event conformance

- **Outcome**: Claude post-init model rejection is pinned to the safe backend-error contract and the
  single terminal `Error` event rule.
- **Inputs/outputs**:
  - Input: `MS-C04`, `MS-C07`, and the fake Claude scenario
    `model_runtime_rejection_after_init`.
  - Output: Claude runtime/error tests in the existing Claude test surface plus any needed updates
    to `crates/agent_api/src/bin/fake_claude_stream_json_agent_api.rs` to support a stable
    rejection fixture that:
    - emits `system init`,
    - rejects the chosen model after the stream is live,
    - includes secret sentinels in raw output so redaction is testable.
- **Implementation notes**:
  - Reuse the same assertion shape as Codex where possible so the backend-specific differences stay
    limited to fixture transport details.
  - Keep `--fallback-model` out of this task; that belongs to `S2`.
- **Acceptance criteria**:
  - Exactly one terminal `AgentWrapperEventKind::Error` is emitted.
  - The surfaced message is safe/redacted and does not contain the raw model id or fake stdout/stderr.
- **Test notes**:
  - Run: `cargo test -p agent_api claude_code`.
- **Risk/rollback notes**:
  - Keep the fake scenario isolated so existing Claude stream-json tests do not drift.

Checklist:
- Implement:
  - Add a Claude runtime-rejection test that observes `system init` before the failure.
  - Assert identical safe messages in completion and the final `Error` event.
  - Assert exactly one terminal `Error` event and no raw model id/stdout/stderr leakage.
  - Extend the fake Claude scenario only as needed to make redaction and ordering deterministic.
- Test: `cargo test -p agent_api claude_code`.
- Validate: use a unique secret sentinel in fake output and assert it never surfaces.
- Cleanup: keep the scenario name stable and dedicated to this seam.

#### S3.T3 — Capability-matrix freshness assertion for model-selection advertising

- **Outcome**: the regression suite treats stale capability publication as a failure whenever
  `agent_api.config.model.v1` advertising changes.
- **Inputs/outputs**:
  - Input: `MS-C05`, `MS-C08`, the generated artifact
    `docs/specs/universal-agent-api/capability-matrix.md`, and `cargo run -p xtask -- capability-matrix`.
  - Output: test or validation coverage that:
    - reruns the generator in the same change that flips advertising,
    - asserts the resulting matrix matches the committed file,
    - treats absence of the row as acceptable only until the advertising change actually lands,
    - makes stale diffs merge-blocking.
- **Implementation notes**:
  - Prefer a deterministic artifact check over hand-maintained textual assertions about the matrix
    contents.
  - Tie the expectation to the actual advertising state rather than assuming a permanent row before
    SEAM-2 lands.
- **Acceptance criteria**:
  - A stale committed `capability-matrix.md` fails validation after advertising changes.
- **Test notes**:
  - Run: `cargo run -p xtask -- capability-matrix`; `make test`.
- **Risk/rollback notes**:
  - Avoid introducing a brittle duplicate parser for the matrix; use the generator as the source of
    truth.

Checklist:
- Implement:
  - Add a freshness check or documented validation hook for the generated capability matrix.
  - Ensure the check distinguishes pre-advertising and post-advertising states correctly.
  - Wire the check into the existing validation path used for this feature.
- Test: `cargo run -p xtask -- capability-matrix`; `make test`.
- Validate: confirm stale matrix diffs block the change once advertising lands.
- Cleanup: keep the matrix assertion generator-driven, not hand-maintained.
