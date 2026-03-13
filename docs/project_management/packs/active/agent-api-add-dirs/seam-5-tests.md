# SEAM-5 — Tests

- **Name**: add-dir regression coverage
- **Type**: risk
- **Goal / user value**: prove the same add-dir semantics hold across validation, capability
  advertising, argv mapping, and session flows.

## Scope

- In:
  - Shared normalizer tests.
  - Backend capability tests.
  - Backend argv-shape tests.
  - Effective-working-directory resolution tests.
  - Missing/non-directory failure tests.
  - Resume/fork selector-branch tests.
  - Fork invalid-input precedence tests.
  - Post-handle runtime rejection parity tests.
  - Safe error-message tests that prove raw path values are not leaked.
  - Capability-matrix regeneration and final integration closeout after the backend seams land.
- Out:
  - End-to-end live CLI smoke tests.

## Primary interfaces (contracts)

- **Validation coverage contract**
  - **Inputs**:
    - malformed or ambiguous `agent_api.exec.add_dirs.v1` payloads
  - **Outputs**:
    - exact safe `InvalidRequest` templates with no raw path leakage

- **Backend mapping coverage contract**
  - **Inputs**:
    - accepted normalized add-dir list
  - **Outputs**:
    - Codex repeated-pair argv and Claude single-group argv are both pinned
    - Codex proves any accepted `--model` pair stays before emitted `--add-dir`
    - Claude proves any accepted `--model` pair stays before the `--add-dir` group and that the
      group stays before the final `--verbose` token

- **Session parity coverage contract**
  - **Inputs**:
    - accepted add-dir list on resume/fork requests
  - **Outputs**:
    - Claude resume selector `"last"` honors the list with the `--continue` argv branch
    - Claude resume selector `"id"` honors the list with the `--resume <ID>` argv branch
    - Claude fork selector `"last"` honors the list with the `--continue --fork-session` argv
      branch
    - Claude fork selector `"id"` honors the list with the `--fork-session --resume <ID>` argv
      branch
    - Codex fork selector `"last"` and selector `"id"` each take the pinned safe backend rejection
      path before any app-server request
    - invalid fork + add-dir combinations fail as `InvalidRequest` before the Codex-specific backend
      rejection path

- **Runtime rejection parity coverage contract**
  - **Inputs**:
    - accepted add-dir list on a surface that already returned an `AgentWrapperRunHandle`
  - **Outputs**:
    - the still-open events stream emits exactly one terminal `AgentWrapperEventKind::Error` event
    - the terminal event message exactly matches the `AgentWrapperError::Backend { message }`
      surfaced through completion
    - coverage names the handle-returning surfaces that must exercise this branch: Codex exec,
      Codex resume, Claude fresh run, Claude resume selector `"last"` / `"id"`, and Claude fork
      selector `"last"` / `"id"`; Codex fork is excluded because its pinned contract rejects before
      a handle is returned

- **Capability publication contract**
  - **Inputs**:
    - built-in backend capability ids after implementation
  - **Outputs**:
    - `docs/specs/universal-agent-api/capability-matrix.md` is regenerated and includes
      `agent_api.exec.add_dirs.v1` for both built-in backends

## Key invariants / rules

- Tests must check both presence and absence semantics.
- Tests must cover directories outside the working directory to guard against accidental
  containment logic.
- Tests must assert dedup behavior after normalization, not before.
- Tests must assert the exact safe InvalidRequest templates, not just “contains” matches.
- Tests must assert selector-branch-specific behavior when the canonical backend contracts use
  different argv subsequences or app-server request paths.
- Tests must assert that post-handle runtime rejection emits exactly one terminal error event whose
  message exactly matches the completion backend error message.
- Tests must use dedicated add-dir runtime-rejection fixtures; they MUST NOT reuse Claude's
  existing `*_generic_error` scenarios or Codex's generic non-zero-exit fixtures because those
  exercise non-success completion behavior rather than the add-dir `Backend` error contract.

## Dependencies

- Blocks: none
- Blocked by: SEAM-2/3/4

## Touch surface

- `crates/agent_api/src/backend_harness/normalize/tests.rs`
- `crates/agent_api/src/backends/codex/tests/**`
- `crates/agent_api/src/backends/claude_code/tests/**`

## Verification

- Targeted runs while iterating:
  - `cargo test -p agent_api` for shared normalizer-only coverage
  - `cargo test -p agent_api --all-features` for backend mapping + session-flow coverage
- Required acceptance checks inside the targeted/full suites:
  - Claude resume selector `"last"` / `"id"` and fork selector `"last"` / `"id"` each get an
    explicit add-dir argv-placement assertion.
  - Codex fork selector `"last"` / `"id"` each prove accepted-input rejection before
    `thread/list` / `thread/fork` / `turn/start`.
  - Codex fork with invalid add-dir payloads proves `InvalidRequest` wins before the fork-specific
    backend rejection path.
  - Handle-returning runtime rejection coverage proves exactly one terminal
    `AgentWrapperEventKind::Error` event with a completion-identical safe/redacted message.

## Pinned deterministic fixture matrix

| Surface | Fake backend owner | Scenario id | Observable pre-failure event |
| --- | --- | --- | --- |
| Codex exec | `crates/agent_api/src/bin/fake_codex_stream_exec_scenarios_agent_api.rs` | `add_dirs_runtime_rejection_exec` | one `thread.started` event before the rejection |
| Codex resume selector `"last"` | `crates/agent_api/src/bin/fake_codex_stream_exec_scenarios_agent_api.rs` | `add_dirs_runtime_rejection_resume_last` | one `thread.resumed` event before the rejection |
| Codex resume selector `"id"` | `crates/agent_api/src/bin/fake_codex_stream_exec_scenarios_agent_api.rs` | `add_dirs_runtime_rejection_resume_id` | one `thread.resumed` event before the rejection |
| Claude fresh run | `crates/agent_api/src/bin/fake_claude_stream_json_agent_api.rs` | `add_dirs_runtime_rejection_fresh` | the first `system_init` fixture line before the rejection |
| Claude resume selector `"last"` | `crates/agent_api/src/bin/fake_claude_stream_json_agent_api.rs` | `add_dirs_runtime_rejection_resume_last` | the first `system_init` fixture line before the rejection |
| Claude resume selector `"id"` | `crates/agent_api/src/bin/fake_claude_stream_json_agent_api.rs` | `add_dirs_runtime_rejection_resume_id` | the first `system_init` fixture line before the rejection |
| Claude fork selector `"last"` | `crates/agent_api/src/bin/fake_claude_stream_json_agent_api.rs` | `add_dirs_runtime_rejection_fork_last` | the first `system_init` fixture line before the rejection |
| Claude fork selector `"id"` | `crates/agent_api/src/bin/fake_claude_stream_json_agent_api.rs` | `add_dirs_runtime_rejection_fork_id` | the first `system_init` fixture line before the rejection |

- Every runtime-rejection fixture in the matrix above MUST use the safe fixture message
  `add_dirs rejected by runtime`.
- Every runtime-rejection fixture MUST keep leak sentinels strictly backend-private:
  `ADD_DIR_RAW_PATH_SECRET`, `ADD_DIR_STDOUT_SECRET`, and `ADD_DIR_STDERR_SECRET` may appear in
  fake-backend-private payloads or stderr, but parity tests MUST assert that none of them appear in
  any `AgentWrapperEvent.message`, `AgentWrapperEvent.text`, or `AgentWrapperError::Backend { message }`.
- Each parity test covering a matrix row MUST assert all of the following:
  - the run already returned an `AgentWrapperRunHandle`,
  - exactly one terminal `AgentWrapperEventKind::Error` event is emitted while the stream is open,
  - `completion` resolves to `Err(AgentWrapperError::Backend { message: "add_dirs rejected by runtime" })`,
  - the terminal error event message is exactly `add_dirs rejected by runtime`, and
  - no raw path/stdout/stderr sentinel leaks through any user-visible event or completion error.
- Full gate before merge:
  - `cargo run -p xtask -- capability-matrix`
  - `make test`
  - `make preflight`

## Risks / unknowns

- **Risk**: tests may accidentally pin backend-local implementation details instead of the shared
  contract.
- **De-risk plan**: organize tests around the contract registry in `threading.md`, with backend
  tests only asserting backend-specific argv shape, selector-branch differences that are already
  owned by backend contracts, the pinned Codex fork rejection boundary/precedence rules, runtime
  rejection parity, and capability publication.

## Rollout / safety

- No seam is done until regression coverage exists for both built-in backends and the shared
  normalizer, and the generated capability matrix has been refreshed in the same change.
