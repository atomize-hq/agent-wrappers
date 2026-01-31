# C1 - Scenario Catalog v1 (3-6) (Spec)

## Purpose
Extend ADR 0003 wrapper coverage generation by implementing Scenario Catalog v1 Scenarios 3-6 and locking them down with tests, without refreshing committed artifacts yet.

This phase MUST keep all determinism and v1 policy constraints established in C0.

## Normative references (must drive implementation)
- `docs/adr/0003-wrapper-coverage-auto-generation.md`
- `docs/specs/codex-wrapper-coverage-generator-contract.md`
- `docs/specs/codex-wrapper-coverage-scenarios-v1.md` (Scenarios 0-12; implement 3-6 in C1)
- `cli_manifests/codex/RULES.json` (sorting + parity exclusions context)
- `cli_manifests/codex/SCHEMA.json` (`WrapperCoverageV1`)
- `cli_manifests/codex/VALIDATOR_SPEC.md` (IU notes; parity exclusions checks)

## Scope

### 1) Implement Scenario Catalog v1 Scenarios 3-6
File: `crates/codex/src/wrapper_coverage_manifest.rs`

Extend `codex::wrapper_coverage_manifest::wrapper_coverage_manifest()` to include, in addition to C0 coverage:
- Scenario 3: `["resume"]` (flags + args as specified; includes `SESSION_ID` and `PROMPT`)
- Scenario 4: `["apply"]`, `["diff"]`
- Scenario 5: `["login"]`, `["login","status"]`, `["logout"]` with capability-guarded `--mcp` note policy
- Scenario 6: `["features","list"]` with `--json`

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

### 2) Preserve determinism and offline rules
Files:
- `crates/codex/src/wrapper_coverage_manifest.rs`
- `crates/xtask/src/codex_wrapper_coverage.rs`

C1 MUST NOT relax any of the hard constraints from C0:
- `xtask codex-wrapper-coverage` MUST require `SOURCE_DATE_EPOCH` and derive `generated_at` from it (no wall-clock fallback).
- No subprocess execution, no network access, no filesystem discovery reads, no randomness.

## Acceptance Criteria

### C1 (code + integration observable outcomes)
- `SOURCE_DATE_EPOCH=0 cargo run -p xtask -- codex-wrapper-coverage --out /tmp/wrapper_coverage.json` succeeds and includes the Scenario 3-6 paths in addition to C0 paths.
- `/tmp/wrapper_coverage.json` remains deterministic and v1 policy compliant (no scope fields; note policy).

### C1 (tests)
- New/updated tests under `crates/xtask/tests/` lock down:
  - Scenario 3-6 completeness and exactness (paths, flags, args, and notes).
  - v1 invariants still hold (no scope; note restrictions).

## Out of Scope (deferred)
- Implementing Scenarios 7-12.
- Generation-time enforcement of `RULES.json.parity_exclusions` (deferred to C3).
- Refreshing the committed `cli_manifests/codex/wrapper_coverage.json` artifact (deferred to C4).

