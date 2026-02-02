# C1 Spec — Adopt IU Subtree Roots for Intentionally Unwrapped Surfaces

## Purpose
After ADR 0004 is implemented, represent intentionally unwrapped command families as IU subtree roots in wrapper coverage source-of-truth so reports stop producing noisy `missing_*` deltas for those subtrees.

## Scope (normative)

### Wrapper coverage source-of-truth update
Update `crates/codex/src/wrapper_coverage_manifest.rs` to add IU subtree roots (command entries) with non-empty, stable notes for these intentionally unwrapped families:
- `["completion"]`

Exact note strings (stable; copy verbatim):
- `["completion"]`: `Shell completion generation is out of scope for the wrapper.`

Notes:
- These are command entries (IU roots). Do not enumerate descendant flags/args for the purpose of waiving the subtree; ADR 0004 inheritance handles descendants.
- Do not add any explicit descendant overrides in C1 (overrides are out of scope for this triad).

This triad MUST NOT modify any contract/spec docs (they are the source of truth for this work):
- `docs/adr/0004-wrapper-coverage-iu-subtree-inheritance.md`
- `cli_manifests/codex/SCHEMA.json`
- `cli_manifests/codex/RULES.json`
- `cli_manifests/codex/VALIDATOR_SPEC.md`

### Generated artifacts (integration step)
Integration MUST regenerate and commit artifacts so the repository remains self-consistent.

Required commands (copy/paste; run from repo root):
1. Set deterministic timestamp for generated artifacts:
   - `export SOURCE_DATE_EPOCH="$(git log -1 --format=%ct)"`
2. Regenerate wrapper coverage:
   - `cargo run -p xtask -- codex-wrapper-coverage --out cli_manifests/codex/wrapper_coverage.json`
3. Regenerate coverage reports for every committed report version directory:
   - Copy/paste loop:
     - `for dir in cli_manifests/codex/reports/*; do V="$(basename "$dir")"; cargo run -p xtask -- codex-report --version "$V" --root cli_manifests/codex; done`
4. Validate:
   - `cargo run -p xtask -- codex-validate --root cli_manifests/codex`

Required verification (deterministic; run all checks against `coverage.any.json` for the latest validated version):
- Define `V` as: `V="$(cat cli_manifests/codex/latest_validated.txt)"`
- Require file exists: `test -f "cli_manifests/codex/reports/${V}/coverage.any.json"`
- Confirm no `missing_commands` entries exist under the IU roots:
  - `jq -e '.deltas.missing_commands[]? | select(.path[0] == "completion")' "cli_manifests/codex/reports/${V}/coverage.any.json"` MUST produce no output (exit non-zero).
- Confirm no `missing_flags` entries exist under the IU roots:
  - `jq -e '.deltas.missing_flags[]? | select(.path[0] == "completion")' "cli_manifests/codex/reports/${V}/coverage.any.json"` MUST produce no output (exit non-zero).
- Confirm no `missing_args` entries exist under the IU roots:
  - `jq -e '.deltas.missing_args[]? | select(.path[0] == "completion")' "cli_manifests/codex/reports/${V}/coverage.any.json"` MUST produce no output (exit non-zero).
- Confirm IU audit visibility exists:
  - `jq -e '.deltas.intentionally_unsupported[]? | select(.path[0] == "completion")' "cli_manifests/codex/reports/${V}/coverage.any.json"` MUST produce one or more entries (exit zero).

## Tests (required; normative)

Add a new integration-style xtask test that verifies the presence of IU subtree roots in generated wrapper coverage and validates the report impact.

Required new test file:
- `crates/xtask/tests/c7_spec_iu_roots_adoption.rs`

The test MUST:
1. Materialize a minimal valid `cli_manifests/codex` directory in a temp folder by copying repo `SCHEMA.json`, `RULES.json`, and `VERSION_METADATA_SCHEMA.json`.
2. Provide a minimal union snapshot that contains a descendant surface under the IU root (`completion`) so inherited IU classification is observable.
3. Run `xtask codex-wrapper-coverage` against a build that includes the C1 IU roots in `crates/codex/src/wrapper_coverage_manifest.rs` (this is a real-code-path generator run; no hand-edited JSON).
4. Run `xtask codex-report` and assert that descendants under those roots do not appear in `missing_*` and do appear under `deltas.intentionally_unsupported`.

Test constants (hard requirements for determinism):
- Use `const VERSION: &str = "0.61.0";` and `const TS: &str = "1970-01-01T00:00:00Z";` (match existing xtask test conventions in this repo).

## Acceptance Criteria
- IU subtree roots exist in wrapper coverage source-of-truth with the exact note strings above.
- Regenerated reports demonstrate reduced `missing_*` noise for those families and audit visibility under `deltas.intentionally_unsupported`.
- `xtask codex-validate` passes on the regenerated artifacts.

## Out of Scope
- Changing which surfaces are “intentionally unwrapped” (policy is owned by ops docs).
- Adding IU roots for other command families not listed above.
- Converting legacy inventories (`CLI_MATRIX.md`, `capability_manifest.json`) into a separate IU list.
