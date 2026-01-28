# C0-spec – Deterministic Validators (SCHEMA + RULES)

## Scope
- Implement a deterministic validator command: `cargo run -p xtask -- codex-validate`.
- The validator must enforce the normative contracts in:
  - `cli_manifests/codex/SCHEMA.json`
  - `cli_manifests/codex/RULES.json`
  - `cli_manifests/codex/VALIDATOR_SPEC.md`
  - `cli_manifests/codex/VERSION_METADATA_SCHEMA.json`
- The validator must be offline (no network) and deterministic (stable output ordering for failures).
- Required validations (minimum) are defined by this spec (below) and must match `cli_manifests/codex/VALIDATOR_SPEC.md`.

## Command Interface (normative)

### `xtask codex-validate`

Invocation:
- `cargo run -p xtask -- codex-validate --root cli_manifests/codex`

CLI:
- `--root <DIR>` (default: `cli_manifests/codex`)
- `--rules <FILE>` (default: `<root>/RULES.json`)
- `--schema <FILE>` (default: `<root>/SCHEMA.json`)
- `--version-schema <FILE>` (default: `<root>/VERSION_METADATA_SCHEMA.json`)

Exit codes:
- `0` on success
- non-zero on any validation error

Output requirements:
- Print one error per line to stderr.
- Error list ordering must be deterministic:
  - sort by `(path, unit, command_path, key_or_name, field)` where applicable.

## Deterministic Timestamp Rule (normative)

Any timestamps emitted by tooling in this feature (including validators writing “updated_at” during repair flows) must use:
- `SOURCE_DATE_EPOCH` (seconds since unix epoch) when set → render as RFC3339 `...Z`.
- Otherwise: current time (RFC3339).

## Validation Rules (normative)

### 1) Pointer files (must exist and be well-formed)

The validator must enforce `cli_manifests/codex/RULES.json` pointer requirements:
- `cli_manifests/codex/min_supported.txt` exists and is strict semver (`MAJOR.MINOR.PATCH`) with a trailing newline.
- `cli_manifests/codex/latest_validated.txt` exists and is strict semver (`MAJOR.MINOR.PATCH`) with a trailing newline.
- For every target in `RULES.json.union.expected_targets`, these pointer files exist (single line + newline, value is `none` or strict semver):
  - `cli_manifests/codex/pointers/latest_validated/<target_triple>.txt`
  - `cli_manifests/codex/pointers/latest_supported/<target_triple>.txt`
- `cli_manifests/codex/latest_validated.txt` must equal:
  - `cli_manifests/codex/pointers/latest_validated/x86_64-unknown-linux-musl.txt`
  and must not be `none`.

### 2) Version set to validate (deterministic expansion)

The validator must compute `versions_to_validate` as the union of:
- Every `<semver>` value present in pointer files (excluding `none`).
- Every `<semver>` filename under `cli_manifests/codex/versions/`:
  - `cli_manifests/codex/versions/<version>.json`

Sorting for validation order:
- Compare versions by numeric `(major, minor, patch)` ascending; validate in that order.

### 3) Per-version required files

For each `version` in `versions_to_validate`, the validator must require:
- `cli_manifests/codex/versions/<version>.json` exists and validates against `cli_manifests/codex/VERSION_METADATA_SCHEMA.json`.
- `cli_manifests/codex/snapshots/<version>/union.json` exists and validates against `cli_manifests/codex/SCHEMA.json` (`UpstreamSnapshotUnionV2`).
- For each `input.target_triple` listed in `snapshots/<version>/union.json.inputs[]`:
  - `cli_manifests/codex/snapshots/<version>/<target_triple>.json` exists and validates against `cli_manifests/codex/SCHEMA.json` (`UpstreamSnapshotV1`).

### 4) `current.json` pointer invariants

The validator must enforce:
- `cli_manifests/codex/current.json` exists and validates against `cli_manifests/codex/SCHEMA.json` (`UpstreamSnapshotUnionV2`).
- `current.json` is byte-for-byte identical to:
  - `cli_manifests/codex/snapshots/<latest_validated>/union.json`
- `current.json.binary.semantic_version` equals `cli_manifests/codex/latest_validated.txt`.

### 5) Wrapper coverage file

The validator must require:
- `cli_manifests/codex/wrapper_coverage.json` exists and validates against `cli_manifests/codex/SCHEMA.json` (`WrapperCoverageManifestV1`).

Additionally, wrapper coverage resolution invariants must be enforced as hard errors per `cli_manifests/codex/RULES.json.wrapper_coverage.validation`:
- overlapping scopes are manifest-invalid,
- multiple matching entries for a single unit+target resolution are manifest-invalid.

### 6) Report file requirements depend on version status

Let `status` be `cli_manifests/codex/versions/<version>.json.status`.

- If `status == "snapshotted"`:
  - reports are not required.
- If `status ∈ {"reported","validated","supported"}`:
  - `cli_manifests/codex/reports/<version>/coverage.any.json` must exist and validate against `cli_manifests/codex/SCHEMA.json` (`CoverageReportV1`).
  - For each `target_triple` present in `snapshots/<version>/union.json.inputs[].target_triple`:
    - `cli_manifests/codex/reports/<version>/coverage.<target_triple>.json` must exist and validate against `cli_manifests/codex/SCHEMA.json` (`CoverageReportV1`).
  - If `snapshots/<version>/union.json.complete == true`:
    - `cli_manifests/codex/reports/<version>/coverage.all.json` must exist and validate against `cli_manifests/codex/SCHEMA.json` (`CoverageReportV1`).
  - If `snapshots/<version>/union.json.complete == false`:
    - `coverage.all.json` must not be required and must not be used as a gate.

## Acceptance Criteria
- `cargo run -p xtask -- codex-validate --root cli_manifests/codex` exits 0 on a compliant workspace.
- On failure, `codex-validate` exits non-zero and prints a deterministic list of errors that includes:
  - path(s),
  - unit key (command path / flag key / arg name),
  - target triple (when applicable),
  - and matching entry indexes for overlap/multi-match errors (per `RULES.json.wrapper_coverage.validation.error_message_requirements`).

## Out of Scope
- No changes to how snapshots/reports are generated (only validation tooling in this triad).
- No CI workflow changes in this triad.
