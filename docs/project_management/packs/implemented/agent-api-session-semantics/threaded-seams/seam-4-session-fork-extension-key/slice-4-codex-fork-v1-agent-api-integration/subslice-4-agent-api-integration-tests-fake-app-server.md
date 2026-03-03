### S4d — `agent_api` integration tests for Codex fork (`fake_codex_app_server_jsonrpc_agent_api`)

- **User/system value**: Pin Codex fork behavior with deterministic integration tests that validate request shapes, selection behavior, safety, and cancellation without requiring a real `codex` binary.
- **Scope (in/out)**:
  - In:
    - Add a cross-platform fake `codex app-server` JSON-RPC binary that implements the fork subset and asserts request shapes.
    - Add `agent_api` integration tests that:
      - cover selector `"id"` and `"last"` success paths,
      - cover selection failures with pinned messages + terminal `Error` event rule,
      - cover approval-required fail-fast + `$ /cancelRequest`,
      - cover explicit cancellation precedence (`"cancelled"`).
  - Out:
    - Capability advertisement (that is `S4e`).
- **Acceptance criteria**:
  - Tests fail loudly on any drift from the pinned JSON-RPC request shapes and sequencing.
  - Selection failure, approval fail-fast, and cancellation behaviors match the relevant normative specs.
- **Dependencies**:
  - `S4b` core fork flow and `S4c` mapping/safety behaviors.
  - Normative: `docs/specs/codex-app-server-jsonrpc-contract.md` (request/response shapes, cancellation error `-32800`).
  - Normative: `docs/specs/universal-agent-api/extensions-spec.md` (selection failure messages + terminal `Error` rule).
  - Normative: `docs/specs/universal-agent-api/run-protocol-spec.md` (cancellation semantics/precedence).
- **Verification**:
  - `cargo test -p agent_api --features codex`
- **Rollout/safety**:
  - Test-only changes; do not advertise `agent_api.session.fork.v1` yet.

#### S4.T6 — Pin `agent_api` integration tests using a fake app-server JSON-RPC binary

- **Outcome**: Deterministic `agent_api` tests that pin Codex fork behavior without requiring a real Codex CLI.
- **Inputs/outputs**:
  - Inputs: a cross-platform fake `codex` binary that implements the fork subset of `codex app-server` JSON-RPC.
  - Outputs: new integration tests covering success + failure + safety paths.
  - Files (prefer new test file for conflict avoidance):
    - `crates/agent_api/src/bin/fake_codex_app_server_jsonrpc_agent_api.rs` (new)
    - `crates/agent_api/tests/session_fork_v1_codex.rs` (new)
- **Pinned test cases**:
  - selector `"id"`:
    - does not call `thread/list`,
    - calls `thread/fork` with `threadId == <id>`,
    - calls `turn/start` with `threadId == <forked id>` and pinned `input[]` mapping.
  - selector `"last"`:
    - calls `thread/list` with `cwd == <effective working dir>` and uses paging,
    - selects the correct source thread id via max tuple `(updatedAt, createdAt, id)`,
    - calls `thread/fork` with that id.
  - selection failures:
    - empty list → `"no session found"`,
    - unknown id → `"session not found"`,
    - terminal `Error` event rule holds when a stream exists.
  - non-interactive approval fail-fast:
    - fake server emits `codex/event` approval_required during `turn/start`,
    - backend sends `$ /cancelRequest` and fails with `"approval required"`.
  - explicit cancellation:
    - calling cancel sends `$ /cancelRequest` for the in-flight `turn/start` id,
    - completion resolves to `Err(Backend("cancelled"))` and event stream finality matches `run-protocol-spec.md`.
- **Verification**:
  - `cargo test -p agent_api --features codex`

Checklist:
- Implement:
  - Add the fake app-server JSON-RPC binary:
    - scenario-driven (env var) responses for list/fork/start/approval,
    - strict request-shape assertions (fail loudly if contract is violated),
    - `$ /cancelRequest` handling returning JSON-RPC cancelled error `code=-32800`.
  - Add integration tests for each pinned scenario.
