# C0-spec – Deterministic Validators (SCHEMA + RULES)

## Scope
- Implement a deterministic validator command: `cargo run -p xtask -- codex-validate`.
- The validator must enforce the normative contracts in:
  - `cli_manifests/codex/SCHEMA.json`
  - `cli_manifests/codex/RULES.json`
  - `cli_manifests/codex/VALIDATOR_SPEC.md`
  - `cli_manifests/codex/VERSION_METADATA_SCHEMA.json`
- The validator must be offline (no network) and deterministic (stable output ordering for failures).
- Required validations (minimum):
  - Pointer files exist and match `RULES.json.storage.pointers` format/invariants.
  - Every committed artifact listed in `RULES.json.ci_validation.validate_committed_artifacts` (where the `<version>`/`<target_triple>` placeholders expand to existing files) schema-validates against `SCHEMA.json` or `VERSION_METADATA_SCHEMA.json` as appropriate.
  - For wrapper coverage resolution rules: overlapping scopes are detected and treated as a hard error (manifest invalid) per `RULES.json.wrapper_coverage.validation`.

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

