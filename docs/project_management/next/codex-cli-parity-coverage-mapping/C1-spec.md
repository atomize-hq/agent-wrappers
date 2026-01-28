# C1-spec – Per-target Snapshots + Union Builder

## Scope
- Produce committed, per-target snapshots and a committed union snapshot for an upstream `codex` version.
- Extend `xtask codex-snapshot` so it can write per-target snapshots to the committed layout from `cli_manifests/codex/RULES.json`:
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

## Command Interfaces (normative)

All commands below must:
- read `cli_manifests/codex/RULES.json` to obtain:
  - `expected_targets` ordering
  - required target triple
  - platform mapping
  - sorting rules (arrays)
- use deterministic timestamps when `SOURCE_DATE_EPOCH` is set (see `cli_manifests/codex/RULES.json.timestamps`).

### 1) `xtask codex-snapshot` (per-target snapshot mode)

Purpose:
- produce one `UpstreamSnapshotV1` file for a specific upstream `codex` binary + target.

Invocation shape:
- `cargo run -p xtask -- codex-snapshot --codex-binary <PATH> --out-file cli_manifests/codex/snapshots/<version>/<target_triple>.json --capture-raw-help --raw-help-target <target_triple> --supplement cli_manifests/codex/supplement/commands.json`

CLI (new/extended requirements for this feature):
- `--codex-binary <PATH>` (required)
- `--out-file <FILE>` (required for this triad’s committed per-target snapshots)
  - When `--out-file` is set, `--out-dir` must not be used.
- `--capture-raw-help` (required in CI runs for this triad)
- `--raw-help-target <target_triple>` (required when `--capture-raw-help` is set in CI)
  - Raw help must be written under:
    - `cli_manifests/codex/raw_help/<version>/<target_triple>/**`
  - This layout is normative for ADR 0002 conflict evidence.
- `--supplement <FILE>` optional; if set, applies supplements before writing the snapshot.

Determinism requirements for snapshot content:
- Array sorting must follow `cli_manifests/codex/RULES.json.sorting`.
- Snapshot must include the root command entry (`path: []`) so global flags/args are comparable.

### 2) `xtask codex-union`

Purpose:
- merge per-target snapshots for a single upstream `version` into a single union snapshot.

Invocation shape:
- `cargo run -p xtask -- codex-union --root cli_manifests/codex --version <semver>`

CLI:
- `--root <DIR>` (default: `cli_manifests/codex`)
- `--rules <FILE>` (default: `<root>/RULES.json`)
- `--version <SEMVER>` (required)

Inputs:
- For each `target_triple` in `RULES.json.union.expected_targets`, read the per-target file if it exists:
  - `cli_manifests/codex/snapshots/<version>/<target_triple>.json`

Hard errors:
- If the required target per `RULES.json.union.required_target` does not exist, fail (non-zero).

Output:
- Write union snapshot to:
  - `cli_manifests/codex/snapshots/<version>/union.json`

Union merge semantics (normative):
- `expected_targets` must equal `RULES.json.union.expected_targets` in the same order.
- `inputs[]` ordering must follow `RULES.json.sorting.inputs` (expected_targets order).
- `complete` is true only when all expected_targets inputs exist.
- `missing_targets` must be present only when `complete=false` and must list the missing expected targets (sorted in expected_targets order).
- For each command in the union:
  - `available_on` contains every target where the command appeared (sorted in expected_targets order).
  - `flags[].available_on` and `args[].available_on` follow the same rule.
- Canonical identity keys:
  - flags: `key = long if present else short` (examples: `--json` else `-j`)
  - args: `name` from the per-target snapshot arg name
- Conflicts:
  - conflicts are never fatal.
  - conflict entries must be emitted when any of these fields differs across targets for the same unit:
    - command: `usage`
    - flag: `takes_value`, `value_name`, `repeatable`
    - arg: `required`, `variadic`
  - conflict ordering must follow `RULES.json.sorting.conflicts`.

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
