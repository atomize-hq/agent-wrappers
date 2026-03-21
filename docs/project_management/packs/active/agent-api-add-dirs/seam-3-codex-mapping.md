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
    resume, after any accepted `--model` pair.
  - Enforce the pinned Codex fork rejection path before any app-server request when add_dirs and
    `agent_api.session.fork.v1` are combined.
  - Update the backend-owned Codex exec/resume contract doc alongside the code seam:
    `docs/specs/codex-streaming-exec-contract.md`.
  - Update the fork-only Codex app-server contract doc alongside the fork-rejection wiring:
    `docs/specs/codex-app-server-jsonrpc-contract.md`.
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
    - for accepted add-dir inputs, fork selector `"last"` and selector `"id"` both fail before
      `thread/list` / `thread/fork` / `turn/start` with
      `AgentWrapperError::Backend { message: "add_dirs unsupported for codex fork" }`
    - invalid add-dir payloads on fork fail earlier as `AgentWrapperError::InvalidRequest`, so the
      fork-specific backend rejection path applies only after R0 capability gating and pre-spawn
      validation succeed

## Key invariants / rules

- Capability support is not conditional on path contents once the backend supports the key.
- When the key is absent, Codex emits no `--add-dir`.
- Exec/resume keep any accepted `--model` pair earlier in argv than the repeated `--add-dir`
  emission.
- Resume must not silently ignore accepted directories.
- Fork must not silently ignore accepted directories; the only allowed behavior in the current
  contract is the pinned pre-handle backend rejection path.
- The fork-specific backend rejection is post-validation only: malformed shape, bounds failures, and
  missing/non-directory paths MUST stop earlier as `InvalidRequest`.
- Because the fork rejection happens before selector-specific app-server branching, both selector
  `"last"` and selector `"id"` share the same no-request boundary.
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
- `docs/specs/codex-streaming-exec-contract.md`
- `docs/specs/codex-app-server-jsonrpc-contract.md`
- Existing wrapper dependency surface:
  - `crates/codex/src/builder/mod.rs`

## Verification

- Capability tests prove the key is advertised and fail-closed when missing from older builds.
- Mapping tests prove:
  - absent key emits no `--add-dir`
  - present key emits repeated `--add-dir <DIR>` pairs in order for exec/resume
  - exec/resume keep any accepted `--model` pair before the first emitted `--add-dir`
  - relative paths resolve against the effective working directory actually used by Codex
- Fork tests prove accepted add-dir inputs on selector `"last"` and selector `"id"` are both
  rejected before `thread/list` / `thread/fork` / `turn/start` with the pinned safe backend
  message.
- Fork tests prove malformed/out-of-bounds/missing/non-directory add-dir payloads fail as
  `InvalidRequest` before the fork-specific backend rejection path and before any
  `thread/list` / `thread/fork` / `turn/start` request.

## Risks / unknowns

- **Risk**: future Codex app-server schema revisions may eventually expose a real add-dir transport
  field, which would require a new contract revision instead of silently changing behavior.
- **De-risk plan**: treat the current rejection path as fixed until a new pinned app-server
  contract names the exact wire field(s).

## Rollout / safety

- Land after the shared normalizer so Codex does not grow backend-local path semantics.
