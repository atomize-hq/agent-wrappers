### S3 — Runtime rejection parity + integration closeout

- This slice was decomposed because it bundles backend-private fake fixture work, backend-private
  runtime-rejection parity tests, and the canonical capability-matrix/full-gate closeout in one
  pass, which is too large for a single Codex session.
- Archived original: `archive/slice-3-runtime-rejection-and-integration-closeout.md`
- Sub-slice directory: `slice-3-runtime-rejection-and-integration-closeout/`

#### Audit Result

- `slice-3-runtime-rejection-and-integration-closeout.md`: Oversized
- Reasons:
  - bundles fake-binary fixture additions, wrapper-level parity assertions, and canonical artifact
    regeneration/final acceptance gates
  - spans both built-in backend test surfaces plus generated docs/specs output
  - requires coordinated work across the fake Codex binary, fake Claude binary, Codex tests,
    Claude tests, and repo-level closeout commands

#### Decomposition Plan

- `S3a` carries the Codex half of original `S3.T1`: add dedicated runtime-rejection fixtures for
  Codex exec/resume handle-returning surfaces.
- `S3b` carries the Claude half of original `S3.T1`: add dedicated runtime-rejection fixtures for
  Claude fresh/resume/fork handle-returning surfaces.
- `S3c` carries the Codex half of original `S3.T2`: pin exactly-one-terminal-error and no-leak
  parity for Codex exec/resume flows.
- `S3d` carries the Claude half of original `S3.T2`: pin exactly-one-terminal-error and no-leak
  parity for Claude fresh/resume/fork flows.
- `S3e` carries original `S3.T3`: regenerate the capability matrix and run the final repo gates
  after all runtime-rejection coverage is green.

#### Sub-slices

- `slice-3-runtime-rejection-and-integration-closeout/subslice-1-codex-runtime-rejection-fixtures.md`
  - `S3a`: add dedicated `add_dirs_runtime_rejection_*` scenarios to the fake Codex binary for
    exec and both resume selector branches.
- `slice-3-runtime-rejection-and-integration-closeout/subslice-2-claude-runtime-rejection-fixtures.md`
  - `S3b`: add dedicated `add_dirs_runtime_rejection_*` scenarios to the fake Claude binary for
    fresh, resume, and fork selector branches.
- `slice-3-runtime-rejection-and-integration-closeout/subslice-3-codex-runtime-rejection-parity.md`
  - `S3c`: add Codex wrapper-level parity tests proving one terminal error event, completion
    message equality, and no sentinel leakage for exec/resume flows.
- `slice-3-runtime-rejection-and-integration-closeout/subslice-4-claude-runtime-rejection-parity.md`
  - `S3d`: add Claude wrapper-level parity tests proving one terminal error event, completion
    message equality, and no sentinel leakage for fresh/resume/fork flows.
- `slice-3-runtime-rejection-and-integration-closeout/subslice-5-capability-matrix-and-final-gate.md`
  - `S3e`: regenerate `docs/specs/universal-agent-api/capability-matrix.md` and run the required
    closeout commands after `S2`, `S3c`, and `S3d` are complete.
