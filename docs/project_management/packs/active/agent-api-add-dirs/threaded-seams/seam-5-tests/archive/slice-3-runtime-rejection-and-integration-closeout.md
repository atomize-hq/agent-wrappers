### S3 — Runtime rejection parity + integration closeout

- **User/system value**: locks down the most failure-prone cross-backend behavior after a run
  handle already exists, then closes the pack with the generated capability artifact and the full
  acceptance gate in the same change.
- **Scope (in/out)**:
  - In:
    - Dedicated fake-backend runtime-rejection scenarios for every pinned matrix row.
    - Parity tests for handle-returning surfaces:
      - exactly one terminal `AgentWrapperEventKind::Error` event,
      - event message equals the completion backend error message exactly,
      - no raw path/stdout/stderr sentinel leaks.
    - Regenerated `docs/specs/universal-agent-api/capability-matrix.md`.
    - Same-change verification for `cargo run -p xtask -- capability-matrix`, `make test`, and
      `make preflight`.
  - Out:
    - new product behavior; this slice is coverage + generated artifact only.
- **Acceptance criteria**:
  - Every pinned runtime-rejection matrix row has a dedicated scenario id using the safe message
    `add_dirs rejected by runtime`.
  - Every handle-returning surface named in the seam brief has a parity test that asserts one
    terminal `Error` event and completion-identical safe messaging.
  - No raw sentinel (`ADD_DIR_RAW_PATH_SECRET`, `ADD_DIR_STDOUT_SECRET`, `ADD_DIR_STDERR_SECRET`)
    appears in any user-visible event or completion error.
  - The regenerated capability matrix shows `agent_api.exec.add_dirs.v1` for both built-in
    backends in the same change that lands the tests.
- **Dependencies**:
  - SEAM-3 and SEAM-4 finalized behavior for handle-returning surfaces.
  - `AD-C03` safe error posture and the seam brief’s pinned deterministic fixture matrix.
- **Verification**:
  - `cargo test -p agent_api --all-features`
  - `cargo run -p xtask -- capability-matrix`
  - `make test`
  - `make preflight`
- **Rollout/safety**:
  - `S3.T3` should be last because it regenerates the canonical artifact and runs the final gates.

#### S3.T1 — Add dedicated fake-backend runtime-rejection scenarios

- **Outcome**: the fake Codex and Claude fixtures can deterministically exercise the add-dir
  post-handle backend rejection path without reusing generic non-success scenarios.
- **Inputs/outputs**:
  - Input: pinned fixture matrix from `seam-5-tests.md`.
  - Output:
    - `crates/agent_api/src/bin/fake_codex_stream_exec_scenarios_agent_api.rs`
    - `crates/agent_api/src/bin/fake_claude_stream_json_agent_api.rs`
    updated with:
    - the exact scenario ids listed in the seam brief,
    - safe fixture message `add_dirs rejected by runtime`,
    - backend-private sentinel payloads for leak checks.
- **Implementation notes**:
  - Do not reuse existing generic error fixtures; this slice needs the `Backend`-error parity path.
  - Keep the leak sentinels backend-private so only the wrapper-side tests assert redaction.
- **Acceptance criteria**:
  - The runtime-rejection fixtures are deterministic and distinguishable per surface/selector
    branch.
- **Test notes**:
  - Run: `cargo test -p agent_api --all-features`.
- **Risk/rollback notes**:
  - None. Fixture-only work.

Checklist:
- Implement: add all required `add_dirs_runtime_rejection_*` scenario ids to the fake binaries.
- Test: verify each scenario emits the expected pre-failure event before the rejection.
- Validate: confirm the safe fixture message is exactly `add_dirs rejected by runtime`.
- Cleanup: keep secret sentinels private to fixture payloads and stderr only.

#### S3.T2 — Add runtime-rejection parity tests for every handle-returning surface

- **Outcome**: the wrapper-level event/completion contract is pinned for Codex exec/resume and
  Claude fresh/resume/fork, including selector `"last"` / `"id"` branches.
- **Inputs/outputs**:
  - Input: runtime-rejection fixtures from `S3.T1` and `AD-C03`.
  - Output: backend integration tests under:
    - `crates/agent_api/src/backends/codex/tests/**`
    - `crates/agent_api/src/backends/claude_code/tests/**`
    asserting for every matrix row:
    - a run handle was already returned,
    - exactly one terminal `AgentWrapperEventKind::Error` event is emitted while the stream is
      open,
    - `completion` resolves to
      `Err(AgentWrapperError::Backend { message: "add_dirs rejected by runtime" })`,
    - the terminal error event message is exactly `add_dirs rejected by runtime`,
    - no raw path/stdout/stderr sentinels leak into any user-visible event or completion error.
- **Implementation notes**:
  - Keep Codex fork excluded here because its pinned contract rejects before a handle exists.
  - Prefer a small helper for the “exactly one terminal error event + no sentinel leak” assertion
    so the per-surface tests stay readable.
- **Acceptance criteria**:
  - A regression that emits multiple terminal errors, mismatched messages, or leaked sentinel text
    fails deterministically for the affected surface.
- **Test notes**:
  - Run: `cargo test -p agent_api --all-features`.
- **Risk/rollback notes**:
  - None. Pure test coverage.

Checklist:
- Implement: add parity tests for Codex exec/resume and Claude fresh/resume/fork `"last"` / `"id"`.
- Test: `cargo test -p agent_api --all-features`.
- Validate: assert event/completion message equality and exactly-one terminal error semantics.
- Cleanup: centralize repeated no-leak assertions if the test bodies become noisy.

#### S3.T3 — Regenerate capability matrix and run the full integration gate

- **Outcome**: the seam closes with the generated artifact and the required repo-level acceptance
  evidence in the same change.
- **Inputs/outputs**:
  - Input: completed backend capability publication and regression tests from `S2`.
  - Output:
    - regenerated `docs/specs/universal-agent-api/capability-matrix.md`
    - same-change verification evidence for:
      - `cargo run -p xtask -- capability-matrix`
      - `make test`
      - `make preflight`
- **Implementation notes**:
  - Run the capability-matrix generator after the backend advertising tests are green so the
    generated row reflects landed behavior.
  - Treat any drift in the matrix as a release-blocking mismatch, not a follow-up.
- **Acceptance criteria**:
  - The generated matrix includes `agent_api.exec.add_dirs.v1` for both built-in backends, and the
    final repo gates pass without bespoke exclusions.
- **Test notes**:
  - Run the commands above exactly as written.
- **Risk/rollback notes**:
  - If the matrix or full gate fails, the seam is not complete.

Checklist:
- Implement: regenerate `docs/specs/universal-agent-api/capability-matrix.md`.
- Test: run `cargo run -p xtask -- capability-matrix`, `make test`, and `make preflight`.
- Validate: confirm the generated matrix row shows both `claude_code` and `codex` as supported.
- Cleanup: ensure no scratch artifacts or stale generated diffs remain in the change.
