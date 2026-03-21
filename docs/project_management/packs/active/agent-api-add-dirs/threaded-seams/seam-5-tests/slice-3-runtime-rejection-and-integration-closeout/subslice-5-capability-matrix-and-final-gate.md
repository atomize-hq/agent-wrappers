### S3e — Capability matrix and final gate closeout

- **User/system value**: the seam closes only when the generated capability artifact and the full
  repo acceptance gates reflect the landed add-dir behavior for both built-in backends in the same
  change.
- **Scope (in/out)**:
  - In:
    - Regenerate `docs/specs/universal-agent-api/capability-matrix.md`.
    - Run `cargo run -p xtask -- capability-matrix`.
    - Run `make test`.
    - Run `make preflight`.
    - Confirm the generated matrix includes `agent_api.exec.add_dirs.v1` for both `claude_code`
      and `codex`.
    - Check that the change leaves no scratch artifacts or stale generated diffs behind.
  - Out:
    - New backend behavior or additional regression cases.
    - Fixture work or wrapper parity test authoring.
- **Acceptance criteria**:
  - The generated capability matrix shows `agent_api.exec.add_dirs.v1` for both built-in backends.
  - `cargo run -p xtask -- capability-matrix`, `make test`, and `make preflight` all pass without
    bespoke exclusions.
  - Any matrix drift is treated as release-blocking and resolved in the same change.
- **Dependencies**:
  - `S2`
  - `S3c`
  - `S3d`
  - `docs/specs/universal-agent-api/capability-matrix.md`
- **Verification**:
  - `cargo run -p xtask -- capability-matrix`
  - `make test`
  - `make preflight`
- **Rollout/safety**:
  - Last sub-slice in the seam. Do not start until backend capability tests and runtime-rejection
    parity tests are already green.

#### S3e.T1 — Regenerate capability matrix and run final acceptance gates

- **Outcome**: the seam ends with the canonical matrix and repo-level gate evidence aligned to the
  final built-in backend behavior.
- **Files**:
  - `docs/specs/universal-agent-api/capability-matrix.md`

Checklist:
- Implement:
  - regenerate `docs/specs/universal-agent-api/capability-matrix.md` after backend tests are green
  - keep the generated diff in the same change as the runtime-rejection coverage
- Test:
  - run `cargo run -p xtask -- capability-matrix`
  - run `make test`
  - run `make preflight`
- Validate:
  - confirm `agent_api.exec.add_dirs.v1` appears for both `claude_code` and `codex`
  - confirm no scratch artifacts or stale generated files remain in the seam closeout change
