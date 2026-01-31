# C4 - Integration + docs (Spec)

## Purpose
Integrate all ADR 0003 scenario coverage slices into a coherent, validated, committed state:
- merge scenario coverage from C1/C2/C3,
- refresh the committed `cli_manifests/codex/wrapper_coverage.json`,
- run report + validate gates against existing snapshots, and
- reconcile documentation if required.

## Normative references (must drive implementation)
- `docs/adr/0003-wrapper-coverage-auto-generation.md`
- `docs/specs/codex-wrapper-coverage-generator-contract.md`
- `docs/specs/codex-wrapper-coverage-scenarios-v1.md` (Scenarios 0-12)
- `cli_manifests/codex/RULES.json` (sorting + parity exclusions)
- `cli_manifests/codex/SCHEMA.json`
- `cli_manifests/codex/VALIDATOR_SPEC.md`

## Scope

### 1) Validate the integrated state
Integration assumes C1/C2/C3 are already merged into `feat/codex-wrapper-coverage-auto-generation` via their per-phase integration tasks.

Integration MUST:
- reconcile any drift so behavior matches the specs `C1-spec.md`, `C2-spec.md`, `C3-spec.md` exactly,
- keep v1 invariants (no scope fields; note restrictions),
- keep determinism rules (`SOURCE_DATE_EPOCH` required; no wall clock fallback).

### 2) Refresh committed wrapper coverage artifact
File:
- `cli_manifests/codex/wrapper_coverage.json`

Integration MUST regenerate and commit:
- `SOURCE_DATE_EPOCH=0 cargo run -p xtask -- codex-wrapper-coverage --out cli_manifests/codex/wrapper_coverage.json`

The committed artifact MUST be:
- non-empty,
- deterministic for the given tree + `SOURCE_DATE_EPOCH`,
- schema-valid (`WrapperCoverageV1`),
- stable-sorted per `RULES.json.sorting`,
- free of any `scope` fields,
- compliant with v1 note policy.

### 3) Run report + validate gates
Files:
- `cli_manifests/codex/reports/<version>/coverage.*.json` (generated)

Integration MUST run (and commit any report diffs required by the new wrapper coverage):
- `VERSION="$(tr -d '\\n' < cli_manifests/codex/latest_validated.txt)"`
- `SOURCE_DATE_EPOCH=0 cargo run -p xtask -- codex-report --version "$VERSION" --root cli_manifests/codex`
- `SOURCE_DATE_EPOCH=0 cargo run -p xtask -- codex-validate --root cli_manifests/codex`

Parity exclusions correctness requirement:
- identities listed in `RULES.json.parity_exclusions` MUST appear only under `excluded_*` report deltas, never under `missing_*`.

### 4) Documentation reconciliation (if required)
Files (allowed to change only if needed for correctness):
- `docs/adr/0003-wrapper-coverage-auto-generation.md`
- `docs/specs/codex-wrapper-coverage-generator-contract.md`
- `docs/specs/codex-wrapper-coverage-scenarios-v1.md`

Integration MUST update docs only to eliminate contradictions introduced by implementation.

## Acceptance Criteria
- `cargo fmt` passes.
- `cargo clippy --workspace --all-targets -- -D warnings` passes.
- `cargo test -p xtask` passes.
- `make preflight` passes.
- `SOURCE_DATE_EPOCH=0 cargo run -p xtask -- codex-validate --root cli_manifests/codex` passes.
- Committed `cli_manifests/codex/wrapper_coverage.json` is non-empty and validator-clean.
