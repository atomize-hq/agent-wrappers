# Codex Wrapper Coverage Generator Contract (v1)

Status: **Normative** (paired with ADR 0003)  
Scope: `cli_manifests/codex/wrapper_coverage.json` generation for CI parity reports

## Normative language

This document uses RFC 2119-style requirement keywords (`MUST`, `MUST NOT`).
Any change to these requirements requires updating this spec (and, if applicable, ADR 0003).

## Purpose

Define a **zero-ambiguity**, deterministic contract for generating `cli_manifests/codex/wrapper_coverage.json` from `crates/codex` implementation signals so CI produces meaningful deltas against upstream snapshots.

This contract specifies:
- exact inputs and outputs,
- determinism requirements,
- identity mapping rules (command path / flag key / arg name),
- classification rules (`explicit|passthrough|unsupported|intentionally_unsupported|unknown`),
- scope rules (`platforms` / `target_triples`),
- required "scenario catalog" coverage (what wrapper APIs MUST contribute to the manifest),
- validation and acceptance criteria.

## Non-goals

- Do not discover upstream surfaces by running a Codex binary.
- Do not maintain a handwritten upstream inventory in JSON.
- Do not attempt to model deeper runtime parsing semantics beyond help-surface parity.
- Do not attempt to prove that every recorded surface works on every upstream version.
- Do not claim support for interactive TUI mode surfaces excluded by `cli_manifests/codex/RULES.json.parity_exclusions`.

## Normative references

- Shape contract: `cli_manifests/codex/SCHEMA.json` (`WrapperCoverageV1`)
- Semantic rules: `cli_manifests/codex/RULES.json` (`wrapper_coverage.*`, globals model)
- Cross-file invariants: `cli_manifests/codex/VALIDATOR_SPEC.md`
- System intent: `docs/adr/0002-codex-cli-parity-coverage-mapping.md`
- Auto-generation intent: `docs/adr/0003-wrapper-coverage-auto-generation.md`
- Scenario catalog (v1): `docs/specs/codex-wrapper-coverage-scenarios-v1.md`

## Definitions

### Unit identities

All identities are compared against upstream union snapshots using `cli_manifests/codex/RULES.json.identity`:

- **Command** identity: `path: string[]` (root is `[]`)
- **Flag** identity: `key: string` (canonical `--long` or `-s` per union)
- **Arg** identity: `name: string` (help-derived positional arg name)

### Coverage levels (meaning in v1)

These are wrapper-side claims about support:

- `explicit`: wrapper provides a first-class API surface (typed builder field, request field, or dedicated method) that deterministically causes this unit to be used.
- `passthrough`: wrapper reaches the unit only through a generic forwarding mechanism with weak semantics (e.g., "raw config overrides"), or the wrapper intentionally avoids validating/typing the surface.
- `unsupported`: wrapper intentionally does not support the unit (no capability), but this is not a policy decision; reports MUST treat this as work-queue input.
- `intentionally_unsupported`: wrapper intentionally will not support the unit; it MUST include a non-empty `note` (validator-enforced).
- `unknown`: wrapper has not yet assessed the unit; treated as missing for work-queue purposes.

In v1, reports treat `explicit` and `passthrough` as supported and omit them from missing lists; `unsupported`, `intentionally_unsupported`, `unknown`, and not present in wrapper coverage are reported as deltas.

## Inputs (authoritative)

### Required inputs

1. Rust implementation signal source:
   - `codex::wrapper_coverage_manifest::wrapper_coverage_manifest()` (`crates/codex/src/wrapper_coverage_manifest.rs`)
2. Rule file:
   - `cli_manifests/codex/RULES.json` (used by `xtask codex-wrapper-coverage` for target ordering and scope normalization)

### Required environment for deterministic artifacts

To keep committed artifacts deterministic, `xtask codex-wrapper-coverage` MUST require:
- `SOURCE_DATE_EPOCH` set to an integer Unix timestamp (seconds).

If `SOURCE_DATE_EPOCH` is missing or invalid, `xtask codex-wrapper-coverage` MUST fail.

### Prohibited inputs (hard rules)

- No network access.
- No filesystem reads for discovery (other than the rules file read by `xtask`).
- No subprocess execution (do not run a Codex binary).
- No wall-clock time usage inside the wrapper-derived manifest (timestamps are `xtask` responsibility).
- No randomness (UUIDs, temp paths) as signal sources.

## Output (authoritative)

### File

- `cli_manifests/codex/wrapper_coverage.json`

### Schema

Must validate as `WrapperCoverageV1` per `cli_manifests/codex/SCHEMA.json`.

### Required metadata fields

The generated file MUST include:
- `generated_at`: an RFC3339 UTC timestamp derived from `SOURCE_DATE_EPOCH` (seconds) interpreted as a Unix timestamp.
- `wrapper_version`: the `crates/codex` crate version string, taken from the compiled `codex` crate (`env!("CARGO_PKG_VERSION")`) and written by `xtask`.

### Sorting and normalization

`xtask codex-wrapper-coverage` is responsible for:
- stable ordering of commands/flags/args per `cli_manifests/codex/RULES.json.sorting.*`,
- scope normalization:
  - `scope.platforms` expands to expected targets using `RULES.json.union.platform_mapping`,
  - normalized form uses `scope.target_triples` in `RULES.json.union.expected_targets` order,
  - empty/invalid scopes are rejected.

The wrapper-derived manifest MUST be deterministic as a *set* of units (identities + levels + notes). Ordering in the wrapper-derived manifest MUST NOT be relied upon; `xtask` sorting is authoritative.

### Root globals model compatibility

`cli_manifests/codex/RULES.json.globals.root_path` defines `path=[]` as the canonical location for global flags and global positional args.

Contract requirement:
- The generator MUST include a `coverage[]` entry for `path=[]` when the wrapper supports any global flags (as defined in the scenario catalog).
- The generator MUST NOT duplicate global flags across every subcommand entry as a workaround.

The upstream union snapshot generator (`xtask codex-union`) MUST apply `globals.effective_flags_model.union_normalization.dedupe_per_command_flags_against_root=true` so global flags appear only at `path=[]` in union snapshots.

The report generator (`xtask codex-report`) MUST apply `globals.effective_flags_model.reporting` semantics so coverage deltas for global flags are reported only under `path=[]`.

## Derivation algorithm (normative)

### Summary

The wrapper coverage manifest is generated by a deterministic, offline derivation of wrapper-supported surfaces, using an **instrumentation-first hybrid** model:

- The wrapper defines a finite set of **coverage scenarios** (see scenario catalog).
- Each scenario records:
  - supported command paths,
  - supported flags (key + coverage level),
  - supported positional args (name + coverage level),
  - stable notes (only when required by this contract),
  - no scope fields (v1 forbids scope; see below).
- All scenario outputs are union-merged by identity into a single `WrapperCoverageManifestV1`.

The generator MUST NOT emit any command paths, flag keys, or arg names that are not enumerated in the scenario catalog for v1.

### No upstream dependency

The generator MUST NOT consult upstream snapshots to decide what to output. It only emits what the wrapper claims/supports.

If a wrapper-emitted key/name/path does not exist in upstream snapshots, reports MUST classify it as `wrapper_only_*`. `wrapper_only_*` deltas MUST NOT be treated as upstream parity gaps. If the same identity is present in the upstream union snapshot for the version under review, it is compared and reported as part of parity.

### Capability-guarded behavior (normative)

The wrapper contains runtime capability probes (e.g., to decide whether to emit `--add-dir` or `--output-schema` for a specific Codex binary).

For wrapper coverage generation:

- The generator MUST NOT run capability probes.
- The generator MUST record capability-guarded surfaces as supported (`explicit` or `passthrough` per scenario catalog).
- The generator MUST set `note` to the exact string `capability-guarded` on capability-guarded units.

### Note policy (v1, normative)

In v1, `note` fields are restricted to prevent diff churn:

- `intentionally_unsupported` units MUST include a non-empty `note` (validator requirement).
- `passthrough` units MUST include a non-empty rationale `note` (validator requirement).
- Capability-guarded units MUST include `note: "capability-guarded"`.
- All other units MUST omit `note`.

### Identity mapping rules (normative)

#### Command path (`coverage[].path`)

- `path=[]` denotes the root command.
- Subcommand paths are tokens as invoked (e.g., `["features","list"]`).
- The generator MUST record exactly the tokens the wrapper uses when spawning the command.

#### Flag key (`coverage[].flags[].key`)

- Must be the canonical upstream identity string form (e.g., `--model`, `-m`).
- Keys are recorded exactly as the wrapper would emit them in argv.
- The generator MUST NOT emit both a long and short key for the same underlying flag. If the wrapper supports both spellings, it MUST record the long form key.

#### Arg name (`coverage[].args[].name`)

- Arg names MUST match the scenario catalog exactly.
- For command paths that appear in upstream union snapshots, the scenario catalog MUST use the upstream help-derived arg names (e.g., `PROMPT`, `SESSION_ID`, `COMMAND`).
- The generator MUST NOT guess arg names from runtime values.

### Classification rules (normative)

#### Commands

- A command path is `explicit` if the wrapper provides a dedicated API to run that path (method or request type).
- A command path is `passthrough` only if the wrapper exposes a generic "run arbitrary codex command" API; v1 does not require such an API.
- Commands MUST NOT be emitted as `unknown` by default. If a wrapper API exists, it MUST be classified.

#### Flags

- A flag is `explicit` if:
  - the wrapper exposes a specific knob corresponding to that flag (builder/request field, method parameter), OR
  - the wrapper intentionally forces the flag as a default (still explicit).
- A flag is `passthrough` if it is only reachable via a generic forwarding channel (e.g., raw overrides) and not modeled as a dedicated knob.
- A flag is `intentionally_unsupported` only with a non-empty rationale `note`.

#### Positional args

- A positional arg is `explicit` if the wrapper has a first-class API field that maps to that arg and an explicit name binding exists.
- If the wrapper has no explicit binding, omit the arg from wrapper coverage (do not emit `unknown` stubs).

### Scope rules (normative)

For v1, the generator MUST NOT emit any `scope` field:
- `coverage[].scope` MUST be omitted.
- `coverage[].flags[].scope` MUST be omitted.
- `coverage[].args[].scope` MUST be omitted.

Any future introduction of scope requires updating this contract and the scenario catalog to enumerate every scoped entry explicitly.

## Determinism requirements (normative)

The wrapper-derived manifest MUST be deterministic given the source tree:

- It MUST NOT depend on:
  - environment variables (except compile-time `CARGO_PKG_VERSION` for `xtask`, not for wrapper manifest),
  - current time,
  - randomness,
  - filesystem state,
  - runtime capability probes.
- Any scenario values used to exercise fields MUST be fixed literals (strings/paths) committed in code.
- Notes MUST be stable strings (no timestamps, no version numbers, no platform-dependent phrasing).

`xtask codex-wrapper-coverage` MUST be the only component that sets:
- `generated_at`
- `wrapper_version`

### File formatting

`cli_manifests/codex/wrapper_coverage.json` MUST be:
- pretty-printed JSON,
- terminated by a single trailing newline (`\\n`).

## Error handling (normative)

### Wrapper-side (manifest derivation)

`codex::wrapper_coverage_manifest::wrapper_coverage_manifest()` MUST NOT panic under normal operation.

If internal invariants are violated during derivation (e.g., duplicate identity entries with conflicting levels), the wrapper MUST resolve deterministically using the merge rules below.

### xtask-side (file emission)

`xtask codex-wrapper-coverage` MUST fail the run if:
- `schema_version != 1`,
- any scope contains unknown platforms or non-expected targets,
- the manifest is invalid per normalization rules,
- the manifest coverage is empty (to prevent silent regressions).

### Union merge rule for duplicate identities

When multiple scenarios contribute the same unit identity:
- choose the strongest `level` by precedence:
  1. `explicit`
  2. `passthrough`
  3. `intentionally_unsupported`
  4. `unsupported`
  5. `unknown`
- choose `note`:
  - if the winning entry has a note, keep it,
  - else choose the lexicographically-smallest non-empty note among contributing entries,
  - else omit `note`.

This produces monotonic support-does-not-regress behavior when scenarios overlap.

## Validation requirements (normative)

Generated artifacts MUST satisfy:

- JSON Schema validation (`WrapperCoverageV1`) via `cli_manifests/codex/SCHEMA.json`.
- Validator invariants via `xtask codex-validate` and `cli_manifests/codex/VALIDATOR_SPEC.md`, including:
  - `intentionally_unsupported` requires non-empty `note`,
  - scope overlap constraints (single best match).

## Acceptance criteria (normative)

This generator is considered correct when:

1. `cli_manifests/codex/wrapper_coverage.json` is non-empty.
2. The output is deterministic:
   - byte-identical across two runs given the same source tree and `SOURCE_DATE_EPOCH`.
3. The output passes:
   - `xtask codex-validate` (all checks).
4. Coverage reports become actionable deltas:
   - For a new upstream version, the report no longer lists everything missing due solely to empty wrapper coverage.
5. Scenario catalog completeness:
   - Every scenario required by `docs/specs/codex-wrapper-coverage-scenarios-v1.md` is represented in the generated coverage.
