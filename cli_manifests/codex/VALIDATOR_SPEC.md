# Codex Parity Validator Spec (`xtask codex-validate`)

This document specifies the deterministic validator used to enforce `cli_manifests/codex/RULES.json` invariants in CI.

## Goals

- Deterministically enforce semantic invariants that JSON Schema cannot express (cross-file consistency, pointer invariants, rationale requirements).
- Produce stable, actionable error output (sorted, path-addressable) suitable for CI logs.
- Support an optional, purely mechanical `fix` mode for safe automations (no LLM decisions, no network).

## Non-Goals

- No upstream downloads.
- No “best effort” guessing: violations fail fast (unless explicitly suppressed via flags).
- No interactive prompts.

## Command Interface

Binary: `xtask`  
Subcommand: `codex-validate`

### Usage

`cargo run -p xtask -- codex-validate [OPTIONS]`

### Options

- `--codex-dir <PATH>` (default: `cli_manifests/codex`)
  - Root directory containing `SCHEMA.json`, `RULES.json`, pointer files, snapshots, reports, and version metadata.
- `--mode <check|fix>` (default: `check`)
  - `check`: validate only; exit non-zero on any violation.
  - `fix`: apply **purely mechanical** fixes (see “Fix Mode”), then re-run `check`.
- `--rules <PATH>` (default: `<codex-dir>/RULES.json`)
- `--schema <PATH>` (default: `<codex-dir>/SCHEMA.json`)
- `--version-metadata-schema <PATH>` (default: `<codex-dir>/VERSION_METADATA_SCHEMA.json`)
- `--checks <LIST>` (default: `all`)
  - Comma-separated list to run a subset (e.g., `pointers,iu_notes`).
  - Supported values:
    - `pointers`
    - `iu_notes`
    - `version_metadata_validation_sets`
    - `current_json_identity`
    - `schemas` (optional convenience: validate JSON shape against schema files)
    - `all`
- `--json` (default: off)
  - Emit a machine-readable JSON report to stdout in addition to human text.

## Inputs (authoritative)

The validator reads:
- `RULES.json` as the semantic contract (expected targets, required target, pointer invariants, etc.)
- `SCHEMA.json` and `VERSION_METADATA_SCHEMA.json` as shape contracts
- The workspace files under `--codex-dir` (snapshots, reports, pointers, metadata)

No other sources are consulted.

## Output

### Human text output (default)

- One line summary:
  - `OK: codex-validate` on success
  - `FAIL: <n> violations` on failure
- Then one violation per line, stable-sorted:
  - `<CODE> <SEVERITY> <FILEPATH> <JSON_POINTER?> <MESSAGE>`

Sorting:
1. `CODE` (lexicographic)
2. `FILEPATH` (lexicographic)
3. `JSON_POINTER` (lexicographic; empty last)
4. `MESSAGE` (lexicographic)

### JSON output (`--json`)

Top-level object:
- `ok` (bool)
- `violations` (array of objects), each:
  - `code` (string)
  - `severity` (`error` only in v1)
  - `path` (string; file path)
  - `json_pointer` (string, optional)
  - `message` (string)
  - `details` (object, optional)

## Exit Codes

- `0`: success (no violations)
- `2`: violations found
- `3`: validator error (I/O, parse error, invalid RULES/schema files)

## Determinism/Safety

- No network access.
- No nondeterministic ordering.
- `fix` performs only mechanical actions defined below.

## Checks (normative)

### `iu_notes`: `intentionally_unsupported` requires rationale

File: `wrapper_coverage.json`

For each unit with `level == "intentionally_unsupported"`:
- Require `note` exists and `trim().len() > 0`.

Applies to:
- command entries (`coverage[]`)
- nested flags (`coverage[].flags[]`)
- nested args (`coverage[].args[]`)

Violation:
- `IU_NOTE_MISSING`
  - Include unit type, `path`, and `key`/`name` where applicable.

### `pointers`: pointer materialization, format, and consistency

Pointer files are defined by `RULES.json.storage.pointers.*`.

#### Materialization

For every `target_triple` in `RULES.json.union.expected_targets` require existence of:
- `pointers/latest_supported/<target_triple>.txt`
- `pointers/latest_validated/<target_triple>.txt`

Violation:
- `POINTER_MISSING_FILE`

#### Format

Each pointer file must:
- be UTF-8 text
- contain exactly one line and end with a trailing newline
- value is either `none` or a stable semver matching `RULES.json.versioning.pointers.stable_semver_pattern`

Violations:
- `POINTER_INVALID_FORMAT`
- `POINTER_INVALID_VALUE`

#### Linux compatibility invariants

- `latest_validated.txt` must exist, must be a stable semver (not `none`), and must equal:
  - `pointers/latest_validated/<required_target>.txt`
- `current.json` must exist and be byte-for-byte identical to:
  - `snapshots/<latest_validated>/union.json`

Violations:
- `POINTER_LATEST_VALIDATED_MISMATCH`
- `CURRENT_JSON_NOT_EQUAL_UNION`

#### Pointer → metadata/snapshot consistency

If a pointer value is `<semver>`:
- Require `versions/<semver>.json` exists.
- Require `snapshots/<semver>/union.json` exists.
- Require `snapshots/<semver>/<target_triple>.json` exists (for that pointer’s target).

If `latest_supported/<target_triple>.txt` is `<semver>`:
- Require `versions/<semver>.json.coverage.supported_targets` contains `<target_triple>`.

If `latest_validated/<target_triple>.txt` is `<semver>`:
- Require `versions/<semver>.json.coverage.supported_targets` contains `<target_triple>`.
- Require `versions/<semver>.json.validation.passed_targets` contains `<target_triple>`.

Violations:
- `POINTER_REFERENCES_MISSING_VERSION_METADATA`
- `POINTER_REFERENCES_MISSING_SNAPSHOT`
- `POINTER_INCONSISTENT_WITH_VERSION_METADATA`

### `version_metadata_validation_sets`: validation sets are explicit and disjoint

File: `versions/<version>.json`

If `validation` is present:
- `passed_targets`, `failed_targets`, and `skipped_targets` must be pairwise disjoint.
- All targets in those sets must be members of `RULES.json.union.expected_targets`.
- The required target must appear in **exactly one** of the three sets.

Violations:
- `VALIDATION_TARGET_SETS_OVERLAP`
- `VALIDATION_TARGET_NOT_EXPECTED`
- `VALIDATION_REQUIRED_TARGET_NOT_EXPLICIT`

### `current_json_identity`: `current.json` corresponds to latest validated

Using `latest_validated.txt`:
- Parse union snapshot `snapshots/<latest_validated>/union.json`
- Ensure `current.json` is byte-for-byte identical to that union snapshot
- Ensure `current.json.inputs[]` includes the required target (Linux-first v1)

Violations:
- `CURRENT_JSON_NOT_EQUAL_UNION` (same as pointer check)
- `CURRENT_JSON_MISSING_REQUIRED_TARGET`

### `schemas` (optional convenience)

Validate JSON files against:
- `SCHEMA.json` for snapshots/reports/wrapper coverage
- `VERSION_METADATA_SCHEMA.json` for version metadata

Violations:
- `SCHEMA_INVALID_JSON`
- `SCHEMA_VALIDATION_FAILED`

## Fix Mode (mechanical)

`--mode fix` may apply only these mechanical changes:

1. **Create missing pointer files** for every expected target (if absent) with content:
   - `none\n`
2. **Normalize pointer file formatting**:
   - ensure exactly one line + trailing newline; rewrite if necessary
3. **Normalize `current.json`** to match `snapshots/<latest_validated>/union.json` byte-for-byte:
   - overwrite `current.json` with the union snapshot content

`fix` must not:
- delete arbitrary files (retention pruning is a separate mechanical command)
- modify snapshots, reports, or version metadata (except `current.json` per above)
- invent values other than `none`

