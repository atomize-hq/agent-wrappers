# C1-spec – Per-target Snapshots + Union Builder

## Scope
- Produce committed, per-target snapshots and a committed union snapshot for an upstream `codex` version.
- Extend `xtask codex-snapshot` so it can write per-target snapshots to the committed layout from `RULES.json.storage.committed.snapshots.per_target`:
  - `cli_manifests/codex/snapshots/<version>/<target_triple>.json`
- Add a new command `xtask codex-union` to merge available per-target snapshots into:
  - `cli_manifests/codex/snapshots/<version>/union.json` (schema `UpstreamSnapshotUnionV2`)
- Union merge semantics must follow ADR 0002 + `RULES.json`:
  - `expected_targets` is authoritative (from `RULES.json.union.expected_targets`).
  - Hard fail if the required target (Linux) snapshot is missing.
  - If some non-required targets are missing, emit a union with `complete=false` and `missing_targets[]`.
  - Conflicts are recorded (not fatal) with per-target observed values and optional evidence refs (paths relative to CI raw help artifacts).
- Determinism requirements:
  - Stable sort ordering for `commands`, `flags`, `args`, and `conflicts`.
  - `current.json` is not written/modified by this triad (promotion is handled later).

## Acceptance Criteria
- Running the pipeline locally (pointing at real snapshots) can produce:
  - `cli_manifests/codex/snapshots/<version>/<target_triple>.json`
  - `cli_manifests/codex/snapshots/<version>/union.json`
- Generated union snapshots validate against `cli_manifests/codex/SCHEMA.json` (`UpstreamSnapshotUnionV2`).
- When a non-required target snapshot is missing, union output sets `complete=false` and includes `missing_targets[]`.
- When the required target snapshot is missing, union generation fails (non-zero) with a clear error.

## Out of Scope
- Wrapper coverage generation.
- Coverage reports and `versions/<version>.json` updates.
- CI workflow changes.

