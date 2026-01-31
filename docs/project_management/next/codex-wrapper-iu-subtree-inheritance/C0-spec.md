# C0 Spec — Report + Validate IU Subtree Inheritance (ADR 0004)

## Purpose
Implement ADR 0004’s deterministic IU subtree inheritance as a report-time classification rule in `xtask codex-report`, and enforce the corresponding invariants in `xtask codex-validate`.

## Scope (normative)

### Report behavior (`xtask codex-report`)
Implement the ADR 0004 semantics from `docs/adr/0004-wrapper-coverage-iu-subtree-inheritance.md`.

Expected production code touch points (minimum):
- `crates/xtask/src/codex_report.rs` (IU inheritance + IU delta emission + RULES sorting parsing/validation)
- `crates/xtask/src/codex_validate.rs` (IU report invariants)

This triad MUST NOT modify any contract/spec docs (they are the source of truth for this work):
- `docs/adr/0004-wrapper-coverage-iu-subtree-inheritance.md`
- `cli_manifests/codex/SCHEMA.json`
- `cli_manifests/codex/RULES.json`
- `cli_manifests/codex/VALIDATOR_SPEC.md`

Minimum required behaviors:
- **Precedence**: parity exclusions > explicit wrapper declaration > inherited IU > existing missing/unsupported/unknown logic.
- **Inheritance**: if an IU subtree root command path is a prefix of an upstream unit’s command path, classify the unit as IU by inheritance unless the wrapper explicitly declares that exact unit identity.
- **Nearest-root selection**: longest-prefix IU root wins.
- **Note attachment**:
  - explicit IU uses its own note
  - inherited IU uses the chosen root’s note verbatim
- **Report placement**:
  - IU (explicit or inherited) MUST NOT appear under any `missing_*` list.
  - IU MUST appear under `deltas.intentionally_unsupported` with `wrapper_level="intentionally_unsupported"` and a non-empty `note`.
- **Shape**: `deltas.intentionally_unsupported` MUST support command, flag, and arg entry shapes per `cli_manifests/codex/SCHEMA.json`.
- **Sorting**: `deltas.intentionally_unsupported` must be stable-sorted by kind, then path, then key/name per ADR 0004.
- **RULES wiring**: parsing/validation in `crates/xtask/src/codex_report.rs` must validate `sorting.report.{passthrough_candidates,unsupported,intentionally_unsupported}` (do not silently ignore).

Required RULES sorting values (hard requirements):
- `sorting.report.passthrough_candidates` MUST be `by_path`.
- `sorting.report.unsupported` MUST be `by_path`.
- `sorting.report.intentionally_unsupported` MUST be `by_kind_then_path_then_key_or_name`.

Determinism requirements (hard requirements):
- When generating reports in CI/automation, `SOURCE_DATE_EPOCH` MUST be set; the report generator MUST honor it for `generated_at`.
- `deltas.intentionally_unsupported` ordering MUST be fully deterministic and match ADR 0004 sorting rules.

### Validator behavior (`xtask codex-validate`)
Enforce the normative requirements from `cli_manifests/codex/VALIDATOR_SPEC.md`:
- missing lists must never contain IU (`wrapper_level == "intentionally_unsupported"`).
- IU report entries must have non-empty notes.
- IU delta list sorting must be stable/deterministic per the spec.

Required validator violation codes (hard requirements; copy verbatim from the spec):
- `REPORT_MISSING_INCLUDES_INTENTIONALLY_UNSUPPORTED`
- `REPORT_IU_NOTE_MISSING`
- `REPORT_IU_NOT_SORTED`

## Acceptance Criteria
- Report outputs are deterministic and match ADR 0004 (including inherited IU flags/args).
- Schema validation passes for reports produced by the updated report generator.
- `xtask codex-validate` fails with the specified codes for violations and passes for correct outputs.

## Tests (required; normative)

Add a new integration-style xtask test that runs real `xtask` subcommands against a synthetic `cli_manifests/codex` temp directory.

Required new test file:
- `crates/xtask/tests/c5_spec_iu_subtree_inheritance.rs`

The test MUST:
1. Materialize a minimal valid `cli_manifests/codex` directory in a temp folder by copying repo `SCHEMA.json`, `RULES.json`, and `VERSION_METADATA_SCHEMA.json`.
2. Write a minimal union snapshot containing an IU subtree with descendants (including a flag) and an explicit override.
3. Write a minimal `wrapper_coverage.json` that declares:
   - an IU subtree root command with a non-empty note, and
   - an explicit descendant command that overrides inheritance.
4. Run `xtask codex-report --root <temp>/cli_manifests/codex --version <V>` with `SOURCE_DATE_EPOCH` set.
5. Assert that:
   - descendant command/flag entries are absent from `deltas.missing_*`,
   - they are present in `deltas.intentionally_unsupported` with `wrapper_level="intentionally_unsupported"` and the inherited note,
   - the IU delta list is stable-sorted per ADR 0004.

Also add a validator-focused test that:
- creates a report fixture that violates the IU reporting rules by including a `deltas.missing_flags[]` entry with `wrapper_level="intentionally_unsupported"`,
- runs `xtask codex-validate --root <temp>/cli_manifests/codex`,
- and asserts `REPORT_MISSING_INCLUDES_INTENTIONALLY_UNSUPPORTED` is emitted.

Required new test file:
- `crates/xtask/tests/c6_spec_report_iu_validator.rs`

Test constants (hard requirements for determinism):
- Use `const VERSION: &str = "0.61.0";` and `const TS: &str = "1970-01-01T00:00:00Z";` (match existing xtask test conventions in this repo).

## Out of Scope
- Changing CI workflows.
- Changing wrapper coverage generator output shape or adding new wrapper coverage fields.
- Adding “scope” semantics or wildcard IU declarations.
- Adding new parity exclusions (TUI policy is unchanged).
