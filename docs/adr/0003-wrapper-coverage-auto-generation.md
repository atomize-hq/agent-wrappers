# ADR 0003: Deterministic Auto-Generation of Wrapper Coverage (No Handwritten Mapping)

Date: 2026-01-29  
Status: Accepted

## Context

ADR 0002 ("Snapshot -> Coverage -> Work Queue") established a parity workflow:

1. CI runs upstream `codex` binaries to generate per-target snapshots and a merged union snapshot (`cli_manifests/codex/snapshots/<version>/union.json`).
2. CI generates a wrapper coverage inventory (`cli_manifests/codex/wrapper_coverage.json`) describing what `crates/codex` supports at the command/flag/arg level.
3. CI compares upstream snapshot(s) to wrapper coverage to produce deterministic coverage reports (`cli_manifests/codex/reports/<version>/coverage.*.json`) that become an actionable work queue.

The upstream side is working end-to-end: CI downloads upstream binaries, generates snapshots, merges a union, generates reports, and validates artifact invariants.

However, wrapper coverage is currently not meaningful:

- `xtask codex-wrapper-coverage` generates `cli_manifests/codex/wrapper_coverage.json`.
- The generator's only input is `codex::wrapper_coverage_manifest::wrapper_coverage_manifest()` from `crates/codex/src/wrapper_coverage_manifest.rs`.
- That function currently returns `coverage: Vec::new()`, so the generated JSON contains `"coverage": []`.
- Under `cli_manifests/codex/RULES.json`, missing wrapper entries are treated as `unknown`, which causes reports for new upstream versions to show nearly everything as missing/unknown even when the wrapper already supports many surfaces.

Clarification: `cli_manifests/codex/current.json` is generated from the upstream `codex` binary (it must match `snapshots/<latest_validated>/union.json`), not from the wrapper. Wrapper support must be reflected via wrapper coverage artifacts, not via `current.json`.

This contradicts the operational goal: CI MUST highlight *delta work* for a new upstream release (new/changed surfaces), not rediscover the entire CLI as unsupported.

## Problem

We need a deterministic mechanism to generate accurate wrapper coverage automatically from the wrapper implementation signals, without requiring humans to maintain a handwritten command/flag/arg inventory (in JSON or ad hoc mapping tables).

## Decision

Implement **deterministic auto-generation of wrapper coverage** such that:

- `cli_manifests/codex/wrapper_coverage.json` is produced mechanically from `crates/codex` implementation signals.
- The output is deterministic (stable ordering; timestamps controlled via `SOURCE_DATE_EPOCH`; no nondeterministic discovery).
- The generator is offline (no network access; no runtime upstream binary downloads).
- The output validates against existing contracts:
  - shape validation per `cli_manifests/codex/SCHEMA.json` (`WrapperCoverageV1`)
  - scope semantics and resolution per `cli_manifests/codex/RULES.json.wrapper_coverage`
  - invariants per `cli_manifests/codex/VALIDATOR_SPEC.md` (including rationale note requirements for `intentionally_unsupported`)

The generator MUST NOT rely on manual hand-curation of the upstream command/flag/arg inventory as a long-term source of truth.

Parity scope note:
- The wrapper coverage generator targets headless embedding CLI surfaces only.
- Interactive TUI mode and TUI-only units are excluded from parity deltas via `cli_manifests/codex/RULES.json.parity_exclusions`.
- The generator MUST NOT emit excluded identities.

### Decision (v1 generator approach)

Adopt an **instrumentation-first hybrid** approach:

- The wrapper itself (in `crates/codex`) derives a coverage manifest from deterministic coverage scenarios that exercise the wrapper's own command-building logic **without** spawning subprocesses.
- `xtask codex-wrapper-coverage` remains a mechanical normalizer/emitter that:
  - validates/sorts,
  - expands/normalizes scope,
  - sets `generated_at` and `wrapper_version`.

This avoids brittle Rust static analysis while keeping the output offline and deterministic.

### Generator contract (normative)

The generator is fully specified by:

- `docs/specs/codex-wrapper-coverage-generator-contract.md`
- `docs/specs/codex-wrapper-coverage-scenarios-v1.md`

If there is any conflict between prose in this ADR and the contract documents above, the contract documents take precedence for implementation.

## What Is Already Specified (No Contract Changes Required)

The following are already sufficient to specify artifact shapes and comparison semantics:

- `cli_manifests/codex/SCHEMA.json` (snapshots, wrapper coverage, reports)
- `cli_manifests/codex/RULES.json` (scope resolution, report semantics, supported policy)
- `cli_manifests/codex/VERSION_METADATA_SCHEMA.json` (version metadata shape)
- `docs/adr/0002-codex-cli-parity-coverage-mapping.md` (system intent and constraints)

This ADR does not require changes to these contracts by default.

## What Is Not Yet Specified (Requires a Generator Contract)

This ADR's open specification items are resolved by the contract documents:

- `docs/specs/codex-wrapper-coverage-generator-contract.md`
- `docs/specs/codex-wrapper-coverage-scenarios-v1.md`

## Consequences

### Benefits

- Coverage reports become actionable deltas for new upstream versions rather than everything missing.
- CI reliably produces a work queue to add support, deprecate/adjust old surfaces, or explicitly waive surfaces with policy rationale.

### Tradeoffs / Risks

- Auto-derivation is non-trivial; static analysis is brittle, probes are incomplete, and hybrids add complexity.
- False positives/negatives mislead the work queue; tests MUST lock behavior down.

## Follow-ups

- Implement the generator and accompanying tests under `crates/xtask/tests/` per the contract.
- Update ADR 0002 to link directly to the generator contract documents (no contract changes required).
