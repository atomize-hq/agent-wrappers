# Claude Code CLI Parity PR Body Template (for `automation/claude-code-<version>` PRs)

## Goal

Bring the Rust wrapper (`crates/claude_code`) into parity with upstream `claude` **{{VERSION}}** by using the generated parity artifacts in `cli_manifests/claude_code/`.

This PR already contains:
- pinned upstream release assets (`cli_manifests/claude_code/artifacts.lock.json`)
- upstream CLI snapshots (`cli_manifests/claude_code/snapshots/{{VERSION}}/**`)
- deterministic coverage reports (work queue) (`cli_manifests/claude_code/reports/{{VERSION}}/**`)
- version metadata (`cli_manifests/claude_code/versions/{{VERSION}}.json`)

Your job is to use those outputs to implement/waive wrapper support until the report no longer contains uncovered surfaces for the required target (and, when union is complete, for all expected targets).

## Where To Look (Source Of Truth)

- Upstream union snapshot: `cli_manifests/claude_code/snapshots/{{VERSION}}/union.json`
- Coverage work queue: `cli_manifests/claude_code/reports/{{VERSION}}/coverage.any.json`
- Per-target work queue(s): `cli_manifests/claude_code/reports/{{VERSION}}/coverage.<target_triple>.json`
- Wrapper coverage manifest (generated): `cli_manifests/claude_code/wrapper_coverage.json`
- Wrapper coverage source-of-truth (edit this, not the JSON): `crates/claude_code/src/wrapper_coverage_manifest.rs`
- Validator contract: `cli_manifests/claude_code/VALIDATOR_SPEC.md`
- Rules + policy: `cli_manifests/claude_code/RULES.json`
- Agent runbook: `cli_manifests/claude_code/CI_AGENT_RUNBOOK.md`

Baseline (previously supported):
- Latest validated version pointer: `cli_manifests/claude_code/latest_validated.txt`
- Baseline union snapshot: `cli_manifests/claude_code/snapshots/<latest_validated>/union.json`
- Baseline report: `cli_manifests/claude_code/reports/<latest_validated>/coverage.any.json`

Pointer policy:
- Do not change `cli_manifests/claude_code/min_supported.txt` unless maintainers explicitly request a policy bump.

## What To Do (Operational Steps)

1) **Triage the delta for {{VERSION}}**
- Open `cli_manifests/claude_code/reports/{{VERSION}}/coverage.any.json`
- Work the lists in this order:
  - `deltas.missing_commands`
  - `deltas.missing_flags`
  - `deltas.missing_args`
  - `deltas.unsupported` (if present)
  - `deltas.passthrough_candidates` (if present; “nice-to-have explicit promotions”)

2) **Classify each missing/unsupported surface**
For each missing unit (command/flag/arg), choose exactly one:
- **Implement support** in `crates/claude_code` and mark it `explicit` in `crates/claude_code/src/wrapper_coverage_manifest.rs`, or
- If only safely doable via CLI forwarding, mark it `passthrough`, or
- If we intentionally will not support it, mark it `intentionally_unsupported` **with a non-empty `note`** (required by validator).

Guardrails:
- Do **not** hand-edit `cli_manifests/claude_code/wrapper_coverage.json` (it is generated).
- Do **not** modify snapshots/reports by hand; re-run generators instead.

3) **Compare {{VERSION}} to the current supported baseline**
- Read baseline version: `BASELINE="$(cat cli_manifests/claude_code/latest_validated.txt)"`
- Determine what’s new/removed at the CLI surface layer by diffing the two union snapshots:
  - New surfaces = present in `snapshots/{{VERSION}}/union.json` but not present in `snapshots/$BASELINE/union.json`
  - Removed surfaces = present in baseline union but not present in {{VERSION}} union
  - Treat “new surfaces” as high priority to assess for wrapper support.
  - Treat “removed surfaces” as potential wrapper deprecations (or leave as wrapper-only if still needed).

4) **Regenerate + validate after changes**
Run these from repo root:
- `cargo run -p xtask -- claude-wrapper-coverage --out cli_manifests/claude_code/wrapper_coverage.json`
- `cargo run -p xtask -- codex-report --version {{VERSION}} --root cli_manifests/claude_code`
- `cargo run -p xtask -- codex-version-metadata --version {{VERSION}} --status reported --root cli_manifests/claude_code`
- `cargo run -p xtask -- codex-validate --root cli_manifests/claude_code`

Then run wrapper tests (Linux required):
- `cargo test -p claude_code`

## Done Criteria

- `cargo run -p xtask -- codex-validate --root cli_manifests/claude_code` passes.
- For the required target (`linux-x64`), `cli_manifests/claude_code/reports/{{VERSION}}/coverage.linux-x64.json` has:
  - no missing/unknown/unsupported surfaces after regeneration, OR all remaining gaps are explicitly `intentionally_unsupported` with rationale notes.
- If `snapshots/{{VERSION}}/union.json.complete == true`, meet the same criterion for all expected targets.
