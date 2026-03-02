# SEAM-3 — Backend termination responsibilities (CA-C03)

This seam pins what built-in backends must provide to support explicit cancellation.

## Definitions (v1, normative)

- **Termination hook**: the backend-owned action invoked by the harness when
  `AgentWrapperCancelHandle::cancel()` is called.
- **Best-effort termination**: the termination hook requests that the spawned CLI process stop
  executing and reach process exit, without making any new consumer-visible outputs observable.

## Requirements (v1, normative)

Applicability:
- A built-in backend MUST implement these requirements if it advertises `agent_api.control.cancel.v1`.

Minimum termination behavior (testable):
- The backend MUST provide a termination hook that attempts to terminate the spawned CLI process.
- The termination hook MUST be:
  - idempotent (safe to call multiple times), and
  - non-blocking (it MUST return without awaiting process exit or stream draining).
- The termination hook MUST NOT depend on consumer-side drop semantics (dropping `events` / dropping
  the run handle).

Observable behavior constraints:
- Termination MUST NOT change the pinned cancellation completion outcome (`"cancelled"`) defined by:
  - `docs/specs/universal-agent-api/run-protocol-spec.md`, and
  - `SEAM-1` / `CA-C01` in this pack.
- Termination MUST NOT cause raw backend stdout/stderr (or raw JSONL lines) to appear in:
  - `AgentWrapperEvent.message`, `AgentWrapperEvent.text`, or `AgentWrapperEvent.data`, or
  - `AgentWrapperError::Backend.message`.

Failure handling:
- If the termination hook fails to signal/kill the process (implementation-defined reasons), the
  backend MUST:
  - continue to obey the redaction constraints above, and
  - continue to obey SEAM-2 driver semantics (no deadlocks; draining continues).
- Termination failures MAY be logged internally, but MUST NOT be surfaced via non-pinned
  consumer-visible error strings.

Time bounds:
- “Good enough termination” is defined by SEAM-4’s pinned integration test bounds:
  - `CANCEL_TERMINATION_TIMEOUT` in `threaded-seams/seam-4-tests/slice-1-explicit-cancel-integration.md`
  - (regression safety) `DROP_COMPLETION_TIMEOUT` in `threaded-seams/seam-4-tests/slice-2-drop-regression.md`
- Built-in backends MUST satisfy those tests on the supported platforms defined by SEAM-4 (`seam-4-tests.md`).

## Backend notes (informative)

- Codex wrapper uses `kill_on_drop(true)` on spawned commands. One implementation approach for
  best-effort termination is to ensure the child handle is dropped when cancellation is requested.
- Claude Code wrapper similarly kills on drop for spawned commands in its process runner.
