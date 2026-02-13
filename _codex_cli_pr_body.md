# Codex CLI Parity PR Body Template (for `automation/codex-cli-<version>` PRs)

@codex

## Goal

Bring the Rust wrapper (`crates/codex`) into parity with upstream `codex` **rust-v0.100.0** by using the generated parity artifacts in `cli_manifests/codex/`.

This PR already contains:
- pinned upstream release assets (`cli_manifests/codex/artifacts.lock.json`)
- upstream CLI snapshots (`cli_manifests/codex/snapshots/0.100.0/**`)
- deterministic coverage reports (work queue) (`cli_manifests/codex/reports/0.100.0/**`)
- version metadata (`cli_manifests/codex/versions/0.100.0.json`)

Your job is to use those outputs to implement/waive wrapper support until the report no longer contains uncovered surfaces for the required target (and, when union is complete, for all expected targets).

## Where To Look (Source Of Truth)

- Upstream union snapshot: `cli_manifests/codex/snapshots/0.100.0/union.json`
- Coverage work queue: `cli_manifests/codex/reports/0.100.0/coverage.any.json`
- Per-target work queue(s): `cli_manifests/codex/reports/0.100.0/coverage.<target_triple>.json`
- Wrapper coverage manifest (generated): `cli_manifests/codex/wrapper_coverage.json`
- Wrapper coverage source-of-truth (edit this, not the JSON): `crates/codex/src/wrapper_coverage_manifest.rs`
- Wrapper coverage scenario catalog (normative): `docs/specs/codex-wrapper-coverage-scenarios-v1.md`
- Wrapper coverage generator contract (normative): `docs/specs/codex-wrapper-coverage-generator-contract.md`
- Validator contract: `cli_manifests/codex/VALIDATOR_SPEC.md`
- Rules + policy: `cli_manifests/codex/RULES.json`
- Agent runbook: `cli_manifests/codex/CI_AGENT_RUNBOOK.md`

Baseline (previously supported):
- Latest validated version pointer: `cli_manifests/codex/latest_validated.txt`
- Baseline union snapshot: `cli_manifests/codex/snapshots/<latest_validated>/union.json`
- Baseline report: `cli_manifests/codex/reports/<latest_validated>/coverage.any.json`

Pointer policy:
- Do not change `cli_manifests/codex/min_supported.txt` unless maintainers explicitly request a policy bump.

## What To Do (Operational Steps)

1) **Triage the delta for 0.100.0**
- Open `cli_manifests/codex/reports/0.100.0/coverage.any.json`
- Work the lists in this order:
  - `deltas.missing_commands`
  - `deltas.missing_flags`
  - `deltas.missing_args`
  - `deltas.unsupported` (if present)
  - `deltas.passthrough_candidates` (if present; “nice-to-have explicit promotions”)

2) **Classify each missing/unsupported surface**
For each missing unit (command/flag/arg), choose exactly one:
- **Implement support** in `crates/codex` and mark it `explicit` in `crates/codex/src/wrapper_coverage_manifest.rs`, or
- If only safely doable via CLI forwarding, mark it `passthrough`, or
- If we intentionally will not support it, mark it `intentionally_unsupported` **with a non-empty `note`** (required by validator).

Guardrails:
- Do **not** hand-edit `cli_manifests/codex/wrapper_coverage.json` (it is generated).
- Do **not** modify snapshots/reports by hand; re-run generators instead.

3) **Compare 0.100.0 to the current supported baseline**
- Read baseline version: `BASELINE="$(cat cli_manifests/codex/latest_validated.txt)"`
- Determine what’s new/removed at the CLI surface layer by diffing the two union snapshots:
  - New surfaces = present in `snapshots/0.100.0/union.json` but not present in `snapshots/$BASELINE/union.json`
  - Removed surfaces = present in baseline union but not present in 0.100.0 union
  - Treat “new surfaces” as high priority to assess for wrapper support.
  - Treat “removed surfaces” as potential wrapper deprecations (or leave as wrapper-only if still needed).

4) **Regenerate + validate after changes**
Run these from repo root:
- `cargo run -p xtask -- codex-wrapper-coverage --out cli_manifests/codex/wrapper_coverage.json`
- `cargo run -p xtask -- codex-report --version 0.100.0 --root cli_manifests/codex`
- `cargo run -p xtask -- codex-version-metadata --version 0.100.0 --status reported --root cli_manifests/codex`
- `cargo run -p xtask -- codex-validate --root cli_manifests/codex`

Then run wrapper tests (Linux required):
- `cargo test -p codex`
- `cargo test -p codex --examples`
- `CODEX_E2E_BINARY=./codex-x86_64-unknown-linux-musl cargo test -p codex --test cli_e2e -- --nocapture`
- Optional (requires valid auth under `CODEX_E2E_HOME`): `CODEX_E2E_LIVE=1 CODEX_E2E_BINARY=./codex-x86_64-unknown-linux-musl cargo test -p codex --test cli_e2e -- --nocapture`

## Done Criteria

- `cargo run -p xtask -- codex-validate --root cli_manifests/codex` passes.
- For the required target (`x86_64-unknown-linux-musl`), `cli_manifests/codex/reports/0.100.0/coverage.x86_64-unknown-linux-musl.json` has:
  - no missing/unknown/unsupported surfaces after regeneration, OR all remaining gaps are explicitly `intentionally_unsupported` with rationale notes.
- If `snapshots/0.100.0/union.json.complete == true`, meet the same criterion for all expected targets.

## Work Queue Summary (autogenerated)

- Version: `0.100.0`
- Baseline: `0.97.0`
- Required target: `x86_64-unknown-linux-musl`
- Primary report: `cli_manifests/codex/reports/0.100.0/coverage.x86_64-unknown-linux-musl.json`

### Counts (required target)

- missing_commands: 0 (non-help: 0, help-only: 0)
- missing_flags: 4 (root: 0, non-root: 4)
- missing_args: 0
- unsupported: 0
- intentionally_unsupported (audit-only): 2
- excluded_commands/flags/args (audit-only): 0/1/1

### Notes on inheritance / reporting

- **IU subtree inheritance (ADR 0004):** if the wrapper marks a command path as `intentionally_unsupported`, all descendant commands/flags/args are treated as IU and appear under `deltas.intentionally_unsupported` (not `missing_*`). Override by adding an exact wrapper coverage entry.
- **Global flags model:** some flags are treated as global (root) and are reported at `path=[]` to avoid per-command noise; fix root coverage for global flags. If you see the same flag reported missing under many subcommands, confirm whether it is truly global for that version by inspecting `snapshots/0.100.0/union.json`.

### New/Removed Commands vs Baseline

- Only in baseline (`0.97.0`):
- Only in new (`0.100.0`):

### First Missing Commands (non-help)


### First Missing Commands (help-only)


### First Missing Flags (root)


### First Missing Flags (non-root)

- [app-server] --listen
- [exec] --ephemeral
- [exec resume] --ephemeral
- [exec review] --ephemeral

### First Missing Args


Next: open `cli_manifests/codex/reports/0.100.0/coverage.x86_64-unknown-linux-musl.json` and work missing_commands → missing_flags → missing_args.

