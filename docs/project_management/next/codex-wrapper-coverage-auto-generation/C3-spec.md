# C3 - Scenario Catalog v1 (10-12) + parity exclusions (Spec)

## Purpose
Complete the remaining Scenario Catalog v1 command coverage (Scenarios 10-12) and implement generation-time parity exclusion enforcement, with tests that lock both down.

This phase MUST keep all determinism and v1 policy constraints established in C0.

## Normative references (must drive implementation)
- `docs/adr/0003-wrapper-coverage-auto-generation.md`
- `docs/specs/codex-wrapper-coverage-generator-contract.md`
- `docs/specs/codex-wrapper-coverage-scenarios-v1.md` (Scenarios 0-12; implement 10-12 in C3)
- `cli_manifests/codex/RULES.json` (especially `parity_exclusions`)
- `cli_manifests/codex/SCHEMA.json` (`WrapperCoverageV1`, report `excluded_*` deltas)
- `cli_manifests/codex/VALIDATOR_SPEC.md` (parity exclusions checks)

## Scope

### 1) Implement Scenario Catalog v1 Scenarios 10-12
File: `crates/codex/src/wrapper_coverage_manifest.rs`

Extend `codex::wrapper_coverage_manifest::wrapper_coverage_manifest()` to include, in addition to prior coverage:
- Scenario 10: `["sandbox","macos"]`, `["sandbox","linux"]`, `["sandbox","windows"]` with `--log-denials` only under `["sandbox","macos"]` and `COMMAND` arg
- Scenario 11: `["execpolicy","check"]` with required flags and `COMMAND` arg
- Scenario 12: `["mcp-server"]` and `["app-server"]` (server-mode)

Exactness requirements (normative; tests MUST enforce):
- For every command path listed above, emit exactly one command entry with `level: explicit`.
- For each command path, emitted flags/args MUST equal the union of flags/args listed by the catalog for that path.
- MUST omit any flag/arg not listed for that path by the catalog.

v1 restrictions (must hold for all emitted units):
- No scope fields anywhere (`scope` MUST be omitted).
- Note policy:
  - `note: "capability-guarded"` only for capability-guarded units listed in the catalog.
  - `intentionally_unsupported` requires a non-empty rationale note (enforcement must remain).
  - Otherwise omit `note`.

### 2) Enforce parity exclusions at generation time (TUI policy)
Files:
- `cli_manifests/codex/RULES.json` (read-only input)
- `crates/xtask/src/codex_wrapper_coverage.rs`
- `crates/xtask/src/codex_report.rs` (already classifies excluded deltas; tests must lock this down)

Requirements:
- `xtask codex-wrapper-coverage` MUST reject any identity listed in `RULES.json.parity_exclusions.units[]`.
  - If the wrapper-derived manifest contains an excluded identity, `xtask codex-wrapper-coverage` MUST fail with a deterministic error.
- Reports MUST classify excluded identities under:
  - `deltas.excluded_commands`
  - `deltas.excluded_flags`
  - `deltas.excluded_args`
  ...and MUST NOT include excluded identities under any `missing_*` deltas (validator-enforced).

## Acceptance Criteria

### C3 (code + integration observable outcomes)
- `SOURCE_DATE_EPOCH=0 cargo run -p xtask -- codex-wrapper-coverage --out /tmp/wrapper_coverage.json` succeeds and includes Scenario 10-12 paths in addition to prior paths.
- `xtask codex-wrapper-coverage` fails deterministically if the wrapper-derived manifest includes an identity listed in `RULES.json.parity_exclusions.units[]`.

### C3 (tests)
- New/updated tests under `crates/xtask/tests/` lock down:
  - Scenario 10-12 completeness and exactness (paths, flags, args, and notes).
  - parity exclusions enforcement in generation.
  - report semantics for exclusions: excluded identities appear only under `excluded_*` deltas, never under `missing_*`.

## Out of Scope (deferred)
- Refreshing the committed `cli_manifests/codex/wrapper_coverage.json` artifact (deferred to C4).

