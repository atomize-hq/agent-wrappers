# SEAM-3 — Codex backend support

- **Name**: Codex `agent_api.exec.add_dirs.v1` support
- **Type**: platform
- **Goal / user value**: let Codex exec/resume consume the normalized add-dir set with the pinned
  repeated-flag mapping, while making fork behavior deterministic under the current app-server
  contract.

## Scope

- In:
  - Advertise `agent_api.exec.add_dirs.v1` from the Codex backend once implemented.
  - Add the key to Codex supported-extension allowlists.
  - Thread the normalized directory list through Codex policy/spawn structures.
  - Map the list to repeated `--add-dir <DIR>` pairs using existing wrapper support for exec and
    resume.
  - Enforce the pinned Codex fork rejection path before any app-server request when add_dirs and
    `agent_api.session.fork.v1` are combined.
  - Update the backend-owned Codex app-server contract doc alongside the code seam.
- Out:
  - Shared normalization rules.
  - Claude Code behavior.

## Primary interfaces (contracts)

- **Capability contract**
  - **Inputs**:
    - Codex backend instance after implementation lands
  - **Outputs**:
    - `capabilities().ids` and `supported_extension_keys()` include
      `agent_api.exec.add_dirs.v1`

- **Codex mapping contract**
  - **Inputs**:
    - normalized unique directory list
  - **Outputs**:
    - repeated `--add-dir <DIR>` argv pairs in order for exec/resume

- **Codex session-flow contract**
  - **Inputs**:
    - accepted add-dir list on new run, resume, or fork
  - **Outputs**:
    - exec/resume honor the same effective set
    - fork fails before `thread/list` / `thread/fork` / `turn/start` with
      `AgentWrapperError::Backend { message: "add_dirs unsupported for codex fork" }`

## Key invariants / rules

- Capability support is not conditional on path contents once the backend supports the key.
- When the key is absent, Codex emits no `--add-dir`.
- Resume must not silently ignore accepted directories.
- Fork must not silently ignore accepted directories; the only allowed behavior in the current
  contract is the pinned pre-handle backend rejection path.
- Ordering after dedup must be preserved in argv emission.

## Dependencies

- Blocks: SEAM-5
- Blocked by: SEAM-1/2

## Touch surface

- `crates/agent_api/src/backends/codex/mod.rs`
- `crates/agent_api/src/backends/codex/harness.rs`
- `crates/agent_api/src/backends/codex/policy.rs`
- `crates/agent_api/src/backends/codex/exec.rs`
- `crates/agent_api/src/backends/codex/fork.rs`
- `docs/specs/codex-app-server-jsonrpc-contract.md`
- Existing wrapper dependency surface:
  - `crates/codex/src/builder/mod.rs`

## Verification

- Capability tests prove the key is advertised and fail-closed when missing from older builds.
- Mapping tests prove:
  - absent key emits no `--add-dir`
  - present key emits repeated `--add-dir <DIR>` pairs in order for exec/resume
  - relative paths resolve against the effective working directory actually used by Codex
- Fork tests prove accepted add-dir inputs are rejected before `thread/list` / `thread/fork` /
  `turn/start` with the pinned safe backend message.

## Risks / unknowns

- **Risk**: future Codex app-server schema revisions may eventually expose a real add-dir transport
  field, which would require a new contract revision instead of silently changing behavior.
- **De-risk plan**: treat the current rejection path as fixed until a new pinned app-server
  contract names the exact wire field(s).

## Rollout / safety

- Land after the shared normalizer so Codex does not grow backend-local path semantics.
