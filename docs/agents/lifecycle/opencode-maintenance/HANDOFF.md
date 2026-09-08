<!-- generated-by: xtask agent-maintenance renderer; source-of-truth: governance/maintenance-request.toml -->

# Handoff

This file is the canonical contributor execution contract for `opencode` maintenance.

## Packet origin

- detected_by: `.github/workflows/agent-maintenance-release-watch.yml`
- current_validated: `1.4.11`
- target_version: `1.18.28`
- latest_stable: `1.18.29`
- version_policy: `latest_stable_minus_one`
- source_kind: `github_releases`
- source_ref: `anomalyco/opencode`
- dispatch_kind: `packet_pr`
- dispatch_workflow: `agent-maintenance-open-pr.yml`
- branch_name: `automation/opencode-maintenance-1.18.28`

## Support-surface audit

- required: `true`
- pre-run debt count: `15`
- expected post-run debt count: `15`
- discovered upstream surface rows: `0`
- preexisting unsupported rows: `15`
- required uplifts this run:
- none
- deferred preexisting gaps:
- `opencode run` `run` via `requires_new_architectural_seam` (TODOS.md#close-opencode-non-tui-maintenance-gaps)
- `opencode acp` `acp` via `requires_new_architectural_seam` (TODOS.md#close-opencode-non-tui-maintenance-gaps)
- `opencode attach` `attach` via `requires_new_architectural_seam` (TODOS.md#close-opencode-non-tui-maintenance-gaps)
- `opencode models` `models` via `requires_new_architectural_seam` (TODOS.md#close-opencode-non-tui-maintenance-gaps)
- `opencode providers` `providers` via `requires_new_architectural_seam` (TODOS.md#close-opencode-non-tui-maintenance-gaps)
- `opencode serve` `serve` via `requires_new_architectural_seam` (TODOS.md#close-opencode-non-tui-maintenance-gaps)
- `opencode web` `web` via `requires_new_architectural_seam` (TODOS.md#close-opencode-non-tui-maintenance-gaps)
- `opencode run` `--format` via `requires_new_architectural_seam` (TODOS.md#close-opencode-non-tui-maintenance-gaps)
- `opencode run` `--dir` via `requires_new_architectural_seam` (TODOS.md#close-opencode-non-tui-maintenance-gaps)
- `opencode run` `--attach` via `requires_new_architectural_seam` (TODOS.md#close-opencode-non-tui-maintenance-gaps)
- `opencode run` `--model` via `requires_new_architectural_seam` (TODOS.md#close-opencode-non-tui-maintenance-gaps)
- `opencode run` `--continue` via `requires_new_architectural_seam` (TODOS.md#close-opencode-non-tui-maintenance-gaps)
- `opencode run` `--session` via `requires_new_architectural_seam` (TODOS.md#close-opencode-non-tui-maintenance-gaps)
- `opencode run` `--fork` via `requires_new_architectural_seam` (TODOS.md#close-opencode-non-tui-maintenance-gaps)
- `opencode run` `--agent` via `requires_new_architectural_seam` (TODOS.md#close-opencode-non-tui-maintenance-gaps)


## Relay contract

- maintained agent packet: `opencode`
- local execution host: `local Codex CLI host via execute-agent-maintenance`
- executor surface: `execute-agent-maintenance`
- request artifact: `docs/agents/lifecycle/opencode-maintenance/governance/maintenance-request.toml`
- prompt template path: `docs/agents/lifecycle/opencode-maintenance/governance/execute-agent-maintenance-prompt.md`
- prompt sha256: `57f8aa98caabf303e08a3c399afe60a08c693a0346791ea5071443e09e462504`
- canonical handoff: `docs/agents/lifecycle/opencode-maintenance/HANDOFF.md`
- derivative pr summary: `docs/agents/lifecycle/opencode-maintenance/governance/pr-summary.md`
- exact closeout artifact: `docs/agents/lifecycle/opencode-maintenance/governance/maintenance-closeout.json`
- branch linkage: `automation/opencode-maintenance-1.18.28`
- manual closeout required: `true`

## Writable surfaces

- `docs/agents/lifecycle/opencode-maintenance/**`
- `crates/opencode/**`
- `crates/agent_api/**`
- `cli_manifests/opencode/artifacts.lock.json`
- `cli_manifests/opencode/snapshots/1.18.28/**`
- `cli_manifests/opencode/reports/1.18.28/**`
- `cli_manifests/opencode/versions/1.18.28.json`
- `cli_manifests/opencode/wrapper_coverage.json`
- `cli_manifests/support_matrix/current.json`
- `docs/specs/unified-agent-api/support-matrix.md`
- `crates/agent_api/src/runtime_support_data.rs`
- `docs/specs/unified-agent-api/non-tui-support-debt.md`

## Read-only inputs

- `docs/agents/lifecycle/opencode-maintenance/OPS_PLAYBOOK.md`
- `docs/agents/lifecycle/opencode-maintenance/CI_WORKFLOWS_PLAN.md`
- `docs/agents/lifecycle/opencode-maintenance/governance/execute-agent-maintenance-prompt.md`
- `.github/workflows/agent-maintenance-open-pr.yml`
- `docs/specs/unified-agent-api/non-tui-support-debt.md`

## Ordered repo commands

- `cargo fmt --all`
- `cargo run -p xtask -- codex-validate --root cli_manifests/opencode`
- `cargo run -p xtask -- support-matrix --check`
- `cargo run -p xtask -- capability-matrix --check`
- `cargo run -p xtask -- capability-matrix-audit`
- `make preflight`

## Exact green gates

- `cargo fmt --all`
- `cargo run -p xtask -- codex-validate --root cli_manifests/opencode`
- `cargo run -p xtask -- support-matrix --check`
- `cargo run -p xtask -- capability-matrix --check`
- `cargo run -p xtask -- capability-matrix-audit`
- `make preflight`

## Recovery

- recreate packet command: `cargo run -p xtask -- refresh-agent --request docs/agents/lifecycle/opencode-maintenance/governance/maintenance-request.toml --write`
- reopen pr body path: `docs/agents/lifecycle/opencode-maintenance/governance/pr-summary.md`
- reopen pr branch: `automation/opencode-maintenance-1.18.28`
- notes:
- If PR creation fails after packet generation, rerun packet regeneration from the frozen request and reopen the PR from the generated pr-summary path.
- If the local execution-host preflight (local Codex CLI host via execute-agent-maintenance) fails, fix the Codex binary/auth state and rerun `execute-agent-maintenance --dry-run` before write mode.

## Dry-run to write relay

Use the `run_id` printed by the dry-run output, replacing `RUN_ID_FROM_DRY_RUN` before invoking write mode.

```sh
cargo run -p xtask -- execute-agent-maintenance --dry-run --request docs/agents/lifecycle/opencode-maintenance/governance/maintenance-request.toml
cargo run -p xtask -- execute-agent-maintenance --write --request docs/agents/lifecycle/opencode-maintenance/governance/maintenance-request.toml --run-id RUN_ID_FROM_DRY_RUN
```

## Exact closeout command

```sh
cargo run -p xtask -- close-agent-maintenance --request docs/agents/lifecycle/opencode-maintenance/governance/maintenance-request.toml --closeout docs/agents/lifecycle/opencode-maintenance/governance/maintenance-closeout.json
```

## Exact maintained-agent prompt

```md
# Packet PR Maintenance Prompt (`1.18.28`)

This template renders the exact maintained-agent prompt for `opencode` packet execution.
`docs/agents/lifecycle/opencode-maintenance/HANDOFF.md` remains canonical and `governance/pr-summary.md` is derivative.

@codex

## Goal

Execute the automated maintenance packet for `opencode` target `1.18.28`.

## Frozen request contract

- Read `docs/agents/lifecycle/opencode-maintenance/governance/maintenance-request.toml` before changing code or docs.
- Read the packet-owned `support_surface_audit` block before deciding whether the run can succeed.
- Treat `docs/agents/lifecycle/opencode-maintenance/HANDOFF.md` as canonical for writable surfaces, read-only inputs, ordered commands, green gates, and recovery.
- Treat `.github/workflows/agent-maintenance-open-pr.yml` as the opening workflow source.
- Do not write outside the execution contract frozen in the request packet.

## Manifest inputs

- `cli_manifests/opencode/README.md`
- `cli_manifests/opencode/VALIDATOR_SPEC.md`
- `cli_manifests/opencode/RULES.json`
- `cli_manifests/opencode/SCHEMA.json`
- `cli_manifests/opencode/current.json`
- `cli_manifests/opencode/latest_validated.txt`
- `cli_manifests/opencode/wrapper_coverage.json`

## Required workflow

1. Compare the current validated baseline from `cli_manifests/opencode/latest_validated.txt` against the target `1.18.28` artifacts.
2. Use `support_surface_audit` to classify newly discovered non-TUI surface, preexisting non-TUI debt, required uplifts, and allowed deferrals.
3. Land bounded wrapper/backend/manifest/publication updates for every row in `required_uplifts_this_run`.
4. Refresh or create version-scoped manifest artifacts under `cli_manifests/opencode/snapshots/1.18.28/`, `cli_manifests/opencode/reports/1.18.28/`, and `cli_manifests/opencode/versions/1.18.28.json` as required by the packet.
5. Leave closeout manual; record it only with `close-agent-maintenance` after the declared green gates pass.

## Done criteria

- Changes stay within the writable surfaces frozen in `docs/agents/lifecycle/opencode-maintenance/governance/maintenance-request.toml`.
- No newly discovered non-TUI surface remains unresolved unless the packet records one allowed deferral.
- `cargo run -p xtask -- codex-validate --root cli_manifests/opencode` passes.
- The remaining ordered commands and green gates from `docs/agents/lifecycle/opencode-maintenance/HANDOFF.md` pass or are captured in maintainer follow-up notes.

```
