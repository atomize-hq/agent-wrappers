# ADR 0004: Wrapper Coverage - Intentionally Unsupported (IU) Subtree Inheritance

Date: 2026-01-31
Status: Proposed

## Context

ADR 0002 and ADR 0003 define a deterministic parity system:
- Upstream `codex` surfaces are captured in `cli_manifests/codex/snapshots/<version>/union.json`.
- Wrapper-declared support is captured in `cli_manifests/codex/wrapper_coverage.json`, generated from `crates/codex/src/wrapper_coverage_manifest.rs`.
- `xtask codex-report` produces a deterministic work queue under `cli_manifests/codex/reports/<version>/coverage.*.json`.

The wrapper coverage model supports a per-unit `intentionally_unsupported` coverage level for:
- commands,
- flags, and
- positional args.

The v1 generator contract requires:
- `intentionally_unsupported` units must include a non-empty `note` rationale (validator enforced),
- and the report must remain deterministic and reviewable.

The parity system explicitly excludes TUI-only help-surface units using `cli_manifests/codex/RULES.json.parity_exclusions` (these are reported under `excluded_*`, not `missing_*`).

## Problem

Some upstream CLI surface areas are intentionally not wrapped for product/policy reasons (for example: experimental/setup-time utilities such as `codex cloud ...` and `codex completion ...`).

Today, the parity pipeline requires explicit unit-by-unit enumeration to classify an entire subtree as intentionally unsupported:
- marking a parent command as `intentionally_unsupported` does not automatically classify its descendant commands/flags/args,
- so the reports remain noisy unless we add large, brittle lists of IU entries for every descendant unit.

This increases maintenance cost and makes it harder for the parity pipeline to produce actionable deltas.

## Decision

Introduce deterministic "IU subtree inheritance" as a report-time classification rule.

If the wrapper explicitly marks a command path as `intentionally_unsupported`, then upstream units under that command subtree are classified as `intentionally_unsupported` by inheritance, unless overridden by explicit wrapper declarations.

This decision:
- does not change how upstream snapshots are generated,
- and does not change wrapper coverage generation output shape (`wrapper_coverage.json` stays the same).

It changes only how `xtask codex-report` (and corresponding validation rules) interpret wrapper coverage when computing deltas.

## Detailed Semantics

### Definitions

- A command path is a list of tokens, `path: []` for root, `path: ["cloud","exec"]` for subcommands, etc.
- An upstream "unit" is one of:
  - command: identified by `path`,
  - flag: identified by `(path, key)`,
  - arg: identified by `(path, name)`.
- A wrapper coverage "explicit declaration" is any wrapper coverage entry that exactly matches the unit identity (including level `explicit|passthrough|unsupported|unknown|intentionally_unsupported`).

### IU Subtree Root

An "IU subtree root" is a wrapper coverage command entry with:
- `level = intentionally_unsupported`, and
- a non-empty `note` rationale.

Notes:
- IU subtree roots are command entries only (flags/args are never IU roots for inheritance).
- IU subtree roots may be nested; in that case the nearest (longest-prefix) root is used.

### Inheritance Rule

For a given upstream unit U with command path `P`:

1. Find the set of IU subtree roots whose `path` is a prefix of `P` (including equality).
2. If there are no matching IU subtree roots, IU inheritance does not apply.
3. If there is at least one matching IU subtree root:
   - Choose the nearest root by longest prefix (max path length).
   - The chosen root's `note` is the inherited IU rationale for U.

Applicability (scope + platform filter):
- IU subtree inheritance is evaluated in the same report context as all other wrapper resolution:
  - apply the report’s platform filter (`any` / `all` / `exact_target`) first,
  - evaluate only IU roots that are applicable under that filter and wrapper scope resolution rules.
- If scope differences would cause a unit to be IU on some relevant targets but not others in the same report run, that is treated as invalid/inconsistent (the report must fail rather than emit nondeterministic mixed classification).

### Override Rule (explicit wrapper declarations win)

If the wrapper explicitly declares coverage for U (exact unit identity), then IU inheritance does not apply to U.

This allows:
- treating a namespace command as IU while still supporting a specific descendant,
- and migrating a previously unwrapped subtree into the wrapper gradually without removing the parent IU immediately.

### Reporting Rule (do not suppress)

Inherited IU units remain visible and audit-friendly in reports:
- They MUST NOT appear under `missing_commands`, `missing_flags`, or `missing_args`.
- They MUST appear under `deltas.intentionally_unsupported` with:
  - the unit identity (command path, and flag key / arg name when applicable),
  - `wrapper_level = intentionally_unsupported`, and
  - `note` equal to the IU root’s rationale string.

This preserves auditability and makes upstream growth under an intentionally unwrapped subtree visible, while keeping the missing work queue actionable.

### Example (override + inheritance)

Given:
- Wrapper coverage declares command `["completion"]` as `intentionally_unsupported` with `note: "shell completion generation is out of scope for the wrapper"`.
- Wrapper coverage declares command `["completion","bash"]` as `explicit` (hypothetical future support).
- Upstream snapshot contains:
  - commands: `["completion"]`, `["completion","bash"]`, `["completion","zsh"]`
  - flags under `["completion","zsh"]`: `--help`, `--output`

Then:
- `["completion"]` is IU (explicit).
- `["completion","zsh"]` is IU (inherited from `["completion"]`).
- `["completion","bash"]` is **not** IU (explicit override wins).
- `["completion","zsh"] --output` is IU (inherited from `["completion"]`).

Reporting:
- `["completion","zsh"]` and its `--output` flag MUST NOT appear under `missing_*`.
- They MUST appear under `deltas.intentionally_unsupported` with the inherited note string.

### Precedence Order (deterministic)

When classifying an upstream unit:

1. Parity exclusions (TUI policy): if the unit matches `RULES.json.parity_exclusions`, it is reported only under `excluded_*` deltas.
2. Explicit wrapper declaration: if the wrapper coverage explicitly declares the unit, use that level and note.
3. IU subtree inheritance: if the unit is under an IU subtree root and not explicitly declared, classify as inherited IU.
4. Otherwise: classify as missing/unknown/unsupported per existing report rules.

### Note attachment (deterministic)

- If a unit is IU due to an explicit wrapper declaration, use that unit’s own `note` (required and non-empty).
- If a unit is IU due to inheritance, use the chosen IU root’s `note` verbatim (required and non-empty).

### Sorting (deterministic)

Any report list that contains IU entries MUST be stable-sorted so diffs are reviewable.

For `deltas.intentionally_unsupported`, sort by:
1. Unit kind order: command entries first, then flag entries, then arg entries  
   (determined by the entry shape: command has neither `key` nor `name`; flag has `key`; arg has `name`)
2. `path` (lexicographic token compare, then length; i.e. `["a"] < ["a","b"]`)
3. For flags: `key` (lexicographic)
4. For args: `name` (lexicographic)

### Schema / contract impact (normative)

To keep inherited IU entries for flags/args audit-friendly, the coverage report contract MUST allow `deltas.intentionally_unsupported` to include command, flag, and arg identities (not path-only).

This ADR does not introduce new report fields; it widens what may appear in `deltas.intentionally_unsupported` so it can represent IU flags/args as well as commands.

## Consequences

### Benefits

- Dramatically reduces the amount of boilerplate required to intentionally waive an entire command family.
- Keeps parity reports actionable (missing lists focus on real wrapper work).
- Preserves visibility into upstream changes under waived subtrees (reported as IU, not hidden).
- Enables incremental adoption: a subtree can remain IU at the top while specific descendants are promoted to explicit support.

### Tradeoffs

- Reports may include large `intentionally_unsupported` lists for waived subtrees. This is acceptable because:
  - it is deterministic,
  - it is audit-friendly,
  - and it avoids polluting `missing_*`.

### Non-goals

- This ADR does not add inheritance of flags/args from intermediate command paths in the upstream snapshot model.
- This ADR does not change the `root_only` positional-args model.
- This ADR does not add wildcard/pattern-based IU declarations.
- This ADR does not change how scope resolution works in wrapper coverage; IU inheritance is a classification rule layered on top of the existing scope resolution rules.

## Alternatives Considered

1. Require explicit IU enumeration for every unit
   - Rejected: too verbose and brittle for subtree-scale waivers.

2. Suppress inherited IU units entirely from reports
   - Rejected: hides upstream growth and weakens auditability.

3. Add a separate JSON "IU list" outside Rust code
   - Rejected for v1: increases the risk of "turn off parity by config"; keeping IU authored in Rust keeps it closer to wrapper ownership and tests.

## Implementation Notes (non-normative)

Expected changes (to be implemented in a follow-up PR):
- Update `xtask codex-report` to apply IU subtree inheritance and emit derived IU entries under `deltas.intentionally_unsupported`.
- Update `xtask codex-validate` to enforce:
  - `missing_*` never contains `wrapper_level = intentionally_unsupported`, and
  - `deltas.intentionally_unsupported` entries always have a non-empty `note`.
- Update `cli_manifests/codex/RULES.json` and `cli_manifests/codex/VALIDATOR_SPEC.md` to describe this rule without ambiguity.
- Update `cli_manifests/codex/CI_AGENT_RUNBOOK.md` and `cli_manifests/codex/OPS_PLAYBOOK.md` to reflect the workflow: mark the parent command IU to waive a subtree.
