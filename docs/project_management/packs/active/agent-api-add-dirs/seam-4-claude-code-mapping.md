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
  - Preserve or safely reject the same directory set for resume/fork flows.
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
    - one `--add-dir <DIR...>` argv group in normalized order, before session-selector flags and
      before the final prompt token

- **Claude session-flow contract**
  - **Inputs**:
    - accepted add-dir list on new run, resume, or fork
  - **Outputs**:
    - same effective set is honored, or a safe backend error is emitted

## Key invariants / rules

- Capability support is not conditional on path contents once the backend supports the key.
- When the key is absent, Claude emits no `--add-dir`.
- Resume and fork must not silently ignore accepted directories.
- The backend must emit one variadic group, not repeated `--add-dir` flags.
- The variadic group must appear before `--continue`, `--fork-session`, `--resume`, and the final
  prompt token.

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
  - the add-dir group appears before `--continue`, `--fork-session`, `--resume`, and the final
    prompt token
  - relative paths resolve against the effective working directory actually used by Claude Code
- Resume/fork tests prove accepted add-dir inputs are honored or safely rejected.

## Risks / unknowns

- **Risk**: Claude’s session-oriented print flags may accept add-dir differently for resume/fork
  than for a fresh run.
- **De-risk plan**: pin resume/fork CLI behavior in fake-stream tests before broad refactoring so
  backend error handling stays deterministic if the CLI surface rejects the accepted list.

## Rollout / safety

- Land after the shared normalizer so Claude does not grow backend-local path semantics.
