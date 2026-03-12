# SEAM-3 — Codex backend support

- **Name**: Codex `agent_api.exec.add_dirs.v1` support
- **Type**: platform
- **Goal / user value**: let Codex runs, resumes, and forks consume the normalized add-dir set
  with the pinned repeated-flag mapping.

## Scope

- In:
  - Advertise `agent_api.exec.add_dirs.v1` from the Codex backend once implemented.
  - Add the key to Codex supported-extension allowlists.
  - Thread the normalized directory list through Codex policy/spawn structures.
  - Map the list to repeated `--add-dir <DIR>` pairs using existing wrapper support.
  - Preserve or safely reject the same directory set for resume/fork flows.
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
    - repeated `--add-dir <DIR>` argv pairs in order

- **Codex session-flow contract**
  - **Inputs**:
    - accepted add-dir list on new run, resume, or fork
  - **Outputs**:
    - same effective set is honored, or a safe backend error is emitted

## Key invariants / rules

- Capability support is not conditional on path contents once the backend supports the key.
- When the key is absent, Codex emits no `--add-dir`.
- Resume and fork must not silently ignore accepted directories.
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
- Existing wrapper dependency surface:
  - `crates/codex/src/builder/mod.rs`

## Verification

- Capability tests prove the key is advertised and fail-closed when missing from older builds.
- Mapping tests prove:
  - absent key emits no `--add-dir`
  - present key emits repeated `--add-dir <DIR>` pairs in order
  - relative paths resolve against the effective working directory actually used by Codex
- Resume/fork tests prove accepted add-dir inputs are honored or safely rejected.

## Risks / unknowns

- **Risk**: the Codex fork/app-server flow may not accept add-dir state the same way exec/resume
  does.
- **De-risk plan**: spike the fork transport first using the existing fake app-server harness; if
  parity is impossible, pin the exact safe rejection path before broad implementation.

## Rollout / safety

- Land after the shared normalizer so Codex does not grow backend-local path semantics.
