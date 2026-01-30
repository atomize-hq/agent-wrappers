# C0 - Non-empty deterministic core (Spec)

## Purpose
Deliver a **non-empty**, deterministic wrapper coverage generator foundation for ADR 0003:
- start producing meaningful `cli_manifests/codex/wrapper_coverage.json`,
- enforce determinism (`SOURCE_DATE_EPOCH`),
- establish v1 restrictions (no `scope`, restricted `note`),
- seed the manifest with Scenario Catalog v1 coverage for Scenarios 0-2.

This phase is intentionally a "core unblock": it must eliminate the empty-coverage blocker and provide a clear extension path to full Scenario Catalog coverage in C1.

## Normative references (must drive implementation)
- `docs/adr/0003-wrapper-coverage-auto-generation.md`
- `docs/specs/codex-wrapper-coverage-generator-contract.md`
- `docs/specs/codex-wrapper-coverage-scenarios-v1.md` (Scenarios 0-2 only for C0)
- `cli_manifests/codex/RULES.json` (sorting + scope semantics; parity exclusions context)
- `cli_manifests/codex/SCHEMA.json` (`WrapperCoverageV1`)
- `cli_manifests/codex/VALIDATOR_SPEC.md` (IU notes; parity exclusions checks)

## Scope

### 1) Wrapper-derived coverage source is non-empty and deterministic
File: `crates/codex/src/wrapper_coverage_manifest.rs`

Implement `codex::wrapper_coverage_manifest::wrapper_coverage_manifest()` such that:
- `schema_version == 1`
- `generated_at == None` and `wrapper_version == None` (xtask sets these)
- `coverage` is **non-empty** and includes:
  - Scenario 0 root entry: `path=[]` (level: `explicit`) with the required root/global flags listed in `docs/specs/codex-wrapper-coverage-scenarios-v1.md` Scenario 0.
  - Scenario 1/2 entry: `path=["exec"]` (level: `explicit`) with the required flags/args listed in Scenario 1 + Scenario 2 (union rules apply).

v1 restrictions (must hold for all emitted units in C0):
- **No scope fields**: `coverage[].scope`, `coverage[].flags[].scope`, `coverage[].args[].scope` MUST be omitted (`None`).
- **Note policy**:
  - Capability-guarded units MUST have `note: "capability-guarded"` (exact string).
  - All other non-IU units MUST omit `note`.
  - C0 MUST include at least these capability-guarded notes (per the catalog):
    - root `--add-dir` (global; capability-guarded)
    - `["exec"]` `--output-schema` (capability-guarded)

Determinism hard rules (C0 must not violate):
- No subprocess execution (do not run a Codex binary).
- No network access.
- No filesystem reads for discovery.
- No wall-clock time and no randomness (no UUIDs, no temp-path generation as a "signal source").
- Scenario literals used to exercise option presence MUST be fixed constants committed in code.

Merge behavior for overlaps (required by contract):
- If multiple scenarios contribute the same identity, merge deterministically using the precedence rules from `docs/specs/codex-wrapper-coverage-generator-contract.md` ("Union merge rule for duplicate identities").

### 2) `xtask codex-wrapper-coverage` enforces determinism and v1 rules
File: `crates/xtask/src/codex_wrapper_coverage.rs`

Update the generator so it conforms to the generator contract:
- `SOURCE_DATE_EPOCH` is **required**:
  - If missing or invalid, `xtask codex-wrapper-coverage` MUST fail (no fallback to wall-clock).
  - `generated_at` MUST be derived from `SOURCE_DATE_EPOCH` as RFC3339 UTC.
- After normalization:
  - Fail if `manifest.coverage` is empty (prevents silent regression back to "coverage: []").
  - Fail if any `scope` field is present anywhere (v1 scope rule).
  - Fail if `note` violates the v1 note policy:
    - allow `note: "capability-guarded"` only for capability-guarded units
    - allow non-empty `note` only for `intentionally_unsupported` units
    - otherwise require `note` omitted
- Continue to:
  - stable-sort commands/flags/args using `RULES.json.sorting`
  - keep (or refactor) scope normalization code as needed, but in v1 the generator MUST reject any scope presence (do not silently strip and continue)
  - pretty-print JSON and write a trailing newline

## Acceptance Criteria

### C0 (code + integration observable outcomes)
- `SOURCE_DATE_EPOCH=0 cargo run -p xtask -- codex-wrapper-coverage --out /tmp/wrapper_coverage.json` succeeds.
- `/tmp/wrapper_coverage.json`:
  - validates as `WrapperCoverageV1` per `cli_manifests/codex/SCHEMA.json`
  - is non-empty (`coverage` has at least `path=[]` and `path=["exec"]`)
  - is byte-identical across two runs with the same `SOURCE_DATE_EPOCH`
  - ends with a single trailing newline
  - includes `generated_at` derived from `SOURCE_DATE_EPOCH`
  - includes `wrapper_version` matching `crates/codex` crate version
  - contains no `scope` fields anywhere
  - contains no `note` fields except `capability-guarded` where required

### C0 (tests)
- New/updated tests under `crates/xtask/tests/` cover:
  - `SOURCE_DATE_EPOCH` is required (missing/invalid fails)
  - generator output is deterministic and non-empty
  - v1 scope and note restrictions are enforced by generation-time validation (not only by schema)

## Out of Scope (deferred to C1)
Completing Scenario Catalog v1 coverage beyond Scenarios 0-2 is deferred to later phases (C1/C2/C3).

Additionally deferred:
- Generation-time enforcement of parity exclusions (implemented in C3).
- Updating the committed `cli_manifests/codex/wrapper_coverage.json` artifact (C4 integration owns the first committed refresh for ADR 0003).
