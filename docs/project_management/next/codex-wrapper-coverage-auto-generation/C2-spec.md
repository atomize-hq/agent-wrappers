# C2 - Scenario Catalog v1 (7-9) (Spec)

## Purpose
Extend ADR 0003 wrapper coverage generation by implementing Scenario Catalog v1 Scenarios 7-9 and locking them down with tests, without refreshing committed artifacts yet.

This phase MUST keep all determinism and v1 policy constraints established in C0.

## Normative references (must drive implementation)
- `docs/adr/0003-wrapper-coverage-auto-generation.md`
- `docs/specs/codex-wrapper-coverage-generator-contract.md`
- `docs/specs/codex-wrapper-coverage-scenarios-v1.md` (Scenarios 0-12; implement 7-9 in C2)
- `cli_manifests/codex/RULES.json` (sorting + parity exclusions context)
- `cli_manifests/codex/SCHEMA.json` (`WrapperCoverageV1`)
- `cli_manifests/codex/VALIDATOR_SPEC.md` (IU notes; parity exclusions checks)

## Scope

### 1) Implement Scenario Catalog v1 Scenarios 7-9
File: `crates/codex/src/wrapper_coverage_manifest.rs`

Extend `codex::wrapper_coverage_manifest::wrapper_coverage_manifest()` to include, in addition to prior coverage:
- Scenario 7: `["app-server","generate-ts"]`, `["app-server","generate-json-schema"]` with `--out`, plus `--prettier` only under `["app-server","generate-ts"]`
- Scenario 8: `["responses-api-proxy"]` with required flags
- Scenario 9: `["stdio-to-uds"]` with `SOCKET_PATH` arg

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

## Acceptance Criteria

### C2 (code + integration observable outcomes)
- `SOURCE_DATE_EPOCH=0 cargo run -p xtask -- codex-wrapper-coverage --out /tmp/wrapper_coverage.json` succeeds and includes the Scenario 7-9 paths in addition to prior paths.
- `/tmp/wrapper_coverage.json` remains deterministic and v1 policy compliant (no scope fields; note policy).

### C2 (tests)
- New/updated tests under `crates/xtask/tests/` lock down:
  - Scenario 7-9 completeness and exactness (paths, flags, args, and notes).
  - v1 invariants still hold (no scope; note restrictions).

## Out of Scope (deferred)
- Implementing Scenarios 10-12.
- Generation-time enforcement of `RULES.json.parity_exclusions` (deferred to C3).
- Refreshing the committed `cli_manifests/codex/wrapper_coverage.json` artifact (deferred to C4).

