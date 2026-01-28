# CODEX_CLI_PARITY_COVERAGE_MAPPING

Source ADR: `docs/adr/0002-codex-cli-parity-coverage-mapping.md`

This planning pack defines the triad work required to implement the ADR 0002 “coverage mapping” system:
- upstream snapshots (per-target + union),
- deterministic wrapper coverage manifest generation,
- deterministic coverage reports and version metadata,
- deterministic validators for `cli_manifests/codex/SCHEMA.json` + `cli_manifests/codex/RULES.json` + `cli_manifests/codex/VALIDATOR_SPEC.md`,
- CI wiring to run the full pipeline and produce a clean work queue.

Start with:
- `docs/project_management/next/codex-cli-parity-coverage-mapping/plan.md`
- `docs/project_management/next/codex-cli-parity-coverage-mapping/tasks.json`

