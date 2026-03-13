# SEAM-4 — Claude Code backend support

- **Name**: Claude Code `agent_api.exec.add_dirs.v1` support
- **Type**: platform
- **Goal / user value**: let Claude Code runs, resumes, and forks consume the normalized add-dir
  set with the pinned variadic flag mapping.

## Scope

- In:
  - Advertise `agent_api.exec.add_dirs.v1` from the Claude Code backend once implemented.
  - Add the key to Claude supported-extension allowlists.
  - Thread the normalized directory list through Claude policy/spawn structures.
  - Map the list to one `--add-dir <DIR...>` argv group using existing wrapper support.
  - Pin the argv placement in the backend-owned Claude session mapping contract doc.
  - Preserve the same accepted directory set for resume and fork flows.
- Out:
  - Shared normalization rules.
  - Codex behavior.

## Primary interfaces (contracts)

- **Capability contract**
  - **Inputs**:
    - Claude Code backend instance after implementation lands
  - **Outputs**:
    - `capabilities().ids` and `supported_extension_keys()` include
      `agent_api.exec.add_dirs.v1`

- **Claude mapping contract**
  - **Inputs**:
    - normalized unique directory list
  - **Outputs**:
    - one `--add-dir <DIR...>` argv group in normalized order, after any accepted `--model` pair,
      before session-selector flags, and before the final `--verbose` token and prompt token

- **Claude session-flow contract**
  - **Inputs**:
    - accepted add-dir list on new run, resume, or fork
  - **Outputs**:
    - the same effective set is honored on new run, resume selector `"last"`, resume selector
      `"id"`, fork selector `"last"`, and fork selector `"id"`

## Key invariants / rules

- Capability support is not conditional on path contents once the backend supports the key.
- When the key is absent, Claude emits no `--add-dir`.
- Resume and fork must not silently ignore accepted directories.
- If the installed CLI/runtime cannot honor accepted add-dir inputs for a supported run surface,
  the backend MUST take the owner-doc runtime rejection path (`AgentWrapperError::Backend { message }`)
  with a safe/redacted message.
- If that runtime rejection occurs after a run handle and still-open events stream already exist,
  the backend MUST emit exactly one terminal `AgentWrapperEventKind::Error` event with the same
  safe/redacted message later surfaced through completion.
- The backend must emit one variadic group, not repeated `--add-dir` flags.
- The variadic group must appear after any accepted `--model` pair and before `--continue`,
  `--fork-session`, `--resume`, the final `--verbose` token, and the final prompt token.

## Dependencies

- Blocks: SEAM-5
- Blocked by: SEAM-1/2

## Touch surface

- `crates/agent_api/src/backends/claude_code/mod.rs`
- `crates/agent_api/src/backends/claude_code/harness.rs`
- `crates/agent_api/src/backends/claude_code/backend.rs`
- `docs/specs/claude-code-session-mapping-contract.md`
- Existing wrapper dependency surface:
  - `crates/claude_code/src/commands/print.rs`

## Verification

- Capability tests prove the key is advertised and fail-closed when unsupported.
- Mapping tests prove:
  - absent key emits no `--add-dir`
  - present key emits exactly one `--add-dir <DIR...>` group in order
  - the add-dir group appears after any accepted `--model` pair and before `--continue`,
    `--fork-session`, `--resume`, the final `--verbose` token, and the final prompt token
  - relative paths resolve against the effective working directory actually used by Claude Code
- Resume selector `"last"` tests prove the argv contains `--continue` after the variadic add-dir
  group and before the final `--verbose` token/prompt.
- Resume selector `"id"` tests prove the argv contains `--resume <ID>` after the variadic add-dir
  group and before the final `--verbose` token/prompt.
- Fork selector `"last"` tests prove the argv contains `--continue --fork-session` after the
  variadic add-dir group and before the final `--verbose` token/prompt.
- Fork selector `"id"` tests prove the argv contains `--fork-session --resume <ID>` after the
  variadic add-dir group and before the final `--verbose` token/prompt.
- Runtime rejection parity tests prove any supported Claude surface that returns a handle and later
  cannot honor the accepted add-dir set emits exactly one terminal `AgentWrapperEventKind::Error`
  event with the same safe/redacted message carried by completion.
- Post-handle add-dir runtime rejection for Claude is owned only by
  `crates/agent_api/src/bin/fake_claude_stream_json_agent_api.rs`. The add-dir parity scenarios in
  that file MUST be `add_dirs_runtime_rejection_fresh`,
  `add_dirs_runtime_rejection_resume_last`, `add_dirs_runtime_rejection_resume_id`,
  `add_dirs_runtime_rejection_fork_last`, and `add_dirs_runtime_rejection_fork_id`.
- Each add-dir runtime-rejection scenario MUST emit at least the first `system_init` fixture line
  before the failure so the handle-returning precondition is observable, then terminate on the
  backend-owned safe message `add_dirs rejected by runtime`.
- The existing `resume_*_generic_error` and `fork_*_generic_error` scenarios remain evidence for
  generic non-zero exit redaction only; they MUST NOT be reused for add-dir parity because they do
  not satisfy the required `AgentWrapperError::Backend { message }` completion contract.

## Risks / unknowns

- **Risk**: Claude’s session-oriented print flags may accept add-dir differently for resume/fork
  than for a fresh run.
- **De-risk plan**: pin resume/fork CLI behavior in fake-stream tests before broad refactoring. If
  the installed CLI/runtime cannot honor the accepted list for a supported run surface, the backend
  MUST take the owner-doc runtime rejection path (`AgentWrapperError::Backend { message }`) with a
  safe/redacted message (it MUST NOT implement per-environment capability gating unless the
  canonical Universal Agent API specs explicitly introduce that behavior).

## Rollout / safety

- Land after the shared normalizer so Claude does not grow backend-local path semantics.
