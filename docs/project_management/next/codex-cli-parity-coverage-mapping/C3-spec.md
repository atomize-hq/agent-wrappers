# C3-spec – Coverage Reports + Version Metadata + Retention Pruning

## Scope
- Implement deterministic coverage report generation comparing:
  - `cli_manifests/codex/snapshots/<version>/union.json`
  - `cli_manifests/codex/wrapper_coverage.json`
- Output reports must follow `RULES.json.report.file_naming`:
  - `cli_manifests/codex/reports/<version>/coverage.any.json`
  - `cli_manifests/codex/reports/<version>/coverage.<target_triple>.json` (one per included target; see below)
  - `cli_manifests/codex/reports/<version>/coverage.all.json` only if union `complete=true` (error otherwise)
- Implement per-version metadata materialization at:
  - `cli_manifests/codex/versions/<version>.json` (schema `VERSION_METADATA_SCHEMA.json`)
  - Status transitions and gate logic must follow `RULES.json.version_metadata`.
- Implement deterministic retention pruning as a mechanical tool (no LLM decisions), per `RULES.json.storage.retention`:
  - compute keep-set from pointers + last N validated versions,
  - delete out-of-window `snapshots/<version>/` and `reports/<version>/` directories only (never touch raw help artifacts).

## Command Interfaces (normative)

### 1) `xtask codex-report`

Purpose:
- generate deterministic coverage reports for a specific upstream version.

Invocation:
- `cargo run -p xtask -- codex-report --root cli_manifests/codex --version <semver>`

CLI:
- `--root <DIR>` (default: `cli_manifests/codex`)
- `--rules <FILE>` (default: `<root>/RULES.json`)
- `--version <SEMVER>` (required)

Inputs (must exist):
- `cli_manifests/codex/snapshots/<version>/union.json`
- `cli_manifests/codex/wrapper_coverage.json`

Outputs (must be written deterministically, pretty JSON + trailing newline):
- Always:
  - `cli_manifests/codex/reports/<version>/coverage.any.json`
- For each `target_triple` present in `snapshots/<version>/union.json.inputs[].target_triple`:
  - `cli_manifests/codex/reports/<version>/coverage.<target_triple>.json`
- Only when `snapshots/<version>/union.json.complete == true`:
  - `cli_manifests/codex/reports/<version>/coverage.all.json`
- When `complete == false`:
  - `coverage.all.json` must not be generated (and must not be treated as missing).

Report determinism rules (normative):
- `generated_at` must follow `cli_manifests/codex/RULES.json.timestamps.reports`:
  - deterministic in CI via `SOURCE_DATE_EPOCH`.
- Delta arrays must be sorted per `cli_manifests/codex/RULES.json.sorting.report.*`.

Filter semantics (normative, must match `cli_manifests/codex/RULES.json.report.filter_semantics`):
- `any`: include upstream surfaces that appear on at least one included target.
- `exact_target`: include upstream surfaces that appear on that target (only allowed for targets present in union inputs).
- `all`: only allowed when union `complete=true`.

### 2) `xtask codex-version-metadata`

Purpose:
- materialize `cli_manifests/codex/versions/<version>.json` deterministically from union/report state.

Invocation:
- `cargo run -p xtask -- codex-version-metadata --root cli_manifests/codex --version <semver> --status reported`

CLI:
- `--root <DIR>` (default: `cli_manifests/codex`)
- `--rules <FILE>` (default: `<root>/RULES.json`)
- `--version <SEMVER>` (required)
- `--status <snapshotted|reported|validated|supported>` (required)

Rules:
- `updated_at` must be deterministic in CI via `SOURCE_DATE_EPOCH`.
- When setting `status=reported`, required artifacts are the same as for report generation:
  - union exists and validates
  - `coverage.any.json` exists and validates
- This triad must not update pointer files (promotion happens elsewhere).

### 3) `xtask codex-retain` (mechanical retention pruning)

Purpose:
- delete out-of-window snapshots/reports deterministically (mechanical; no LLM decisions).

Invocation:
- dry-run by default: `cargo run -p xtask -- codex-retain --root cli_manifests/codex`
- apply deletions: `cargo run -p xtask -- codex-retain --root cli_manifests/codex --apply`

CLI:
- `--root <DIR>` (default: `cli_manifests/codex`)
- `--rules <FILE>` (default: `<root>/RULES.json`)
- `--apply` (optional; when absent, dry-run)

Keep-set rules (normative):
- Always keep:
  - versions referenced by `min_supported.txt` and `latest_validated.txt`
  - any per-target pointer values that are semver (exclude `none`)
- Keep the last `RULES.json.storage.retention.keep_last_validated` versions with `status ∈ {"validated","supported"}` by semver ordering.

Deletion rules (normative):
- Only delete under:
  - `cli_manifests/codex/snapshots/<version>/`
  - `cli_manifests/codex/reports/<version>/`
- Must not delete:
  - pointer files
  - `wrapper_coverage.json`
  - `versions/<version>.json`
  - `raw_help/**` (CI artifact store)

Output requirements:
- Print a deterministic keep list and delete list (sorted by version ascending).

## Testing Contract (normative)

All tests for report generation, metadata materialization, and retention must live under:
- `crates/xtask/tests/`

## Acceptance Criteria
- Reports validate against `cli_manifests/codex/SCHEMA.json` (`CoverageReportV1`).
- `versions/<version>.json` validates against `cli_manifests/codex/VERSION_METADATA_SCHEMA.json`.
- Generating `coverage.all.json` fails if union `complete=false`.
- Retention pruning deterministically deletes only out-of-window snapshots/reports and prints an explicit keep/delete list.

## Out of Scope
- CI workflow updates (handled in C4).
- Automated PR creation/branch management beyond existing best-effort behavior.
