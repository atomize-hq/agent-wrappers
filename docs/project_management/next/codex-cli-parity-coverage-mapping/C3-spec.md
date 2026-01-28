# C3-spec – Coverage Reports + Version Metadata + Retention Pruning

## Scope
- Implement deterministic coverage report generation comparing:
  - `cli_manifests/codex/snapshots/<version>/union.json`
  - `cli_manifests/codex/wrapper_coverage.json`
- Output reports must follow `RULES.json.report.file_naming`:
  - `cli_manifests/codex/reports/<version>/coverage.any.json`
  - `cli_manifests/codex/reports/<version>/coverage.<target_triple>.json` (one per expected target)
  - `cli_manifests/codex/reports/<version>/coverage.all.json` only if union `complete=true` (error otherwise)
- Implement per-version metadata materialization at:
  - `cli_manifests/codex/versions/<version>.json` (schema `VERSION_METADATA_SCHEMA.json`)
  - Status transitions and gate logic must follow `RULES.json.version_metadata`.
- Implement deterministic retention pruning as a mechanical tool (no LLM decisions), per `RULES.json.storage.retention`:
  - compute keep-set from pointers + last N validated versions,
  - delete out-of-window `snapshots/<version>/` and `reports/<version>/` directories only (never touch raw help artifacts).

## Acceptance Criteria
- Reports validate against `cli_manifests/codex/SCHEMA.json` (`CoverageReportV1`).
- `versions/<version>.json` validates against `cli_manifests/codex/VERSION_METADATA_SCHEMA.json`.
- Generating `coverage.all.json` fails if union `complete=false`.
- Retention pruning deterministically deletes only out-of-window snapshots/reports and prints an explicit keep/delete list.

## Out of Scope
- CI workflow updates (handled in C4).
- Automated PR creation/branch management beyond existing best-effort behavior.

