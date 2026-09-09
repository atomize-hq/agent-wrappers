<!-- generated-by: xtask agent-maintenance renderer; source-of-truth: governance/maintenance-request.toml -->

# Handoff

This file is the canonical contributor execution contract for `codex` maintenance.

## Packet origin

- detected_by: `.github/workflows/agent-maintenance-release-watch.yml`
- current_validated: `0.125.0`
- target_version: `0.153.3`
- latest_stable: `0.153.4`
- version_policy: `latest_stable_minus_one`
- source_kind: `github_releases`
- source_ref: `openai/codex`
- dispatch_kind: `packet_pr`
- dispatch_workflow: `agent-maintenance-open-pr.yml`
- branch_name: `automation/codex-maintenance-0.153.3`

## Support-surface audit

- required: `true`
- pre-run debt count: `2`
- expected post-run debt count: `2`
- discovered upstream surface rows: `0`
- preexisting unsupported rows: `2`
- required uplifts this run:
- none
- deferred preexisting gaps:
- `codex completion` `completion` via `requires_new_architectural_seam` (TODOS.md#close-codex-completion-maintenance-gap)
- `codex completion` `SHELL` via `requires_new_architectural_seam` (TODOS.md#close-codex-completion-maintenance-gap)


## Relay contract

- maintained agent packet: `codex`
- local execution host: `local Codex CLI host via execute-agent-maintenance`
- executor surface: `execute-agent-maintenance`
- request artifact: `docs/agents/lifecycle/codex-maintenance/governance/maintenance-request.toml`
- prompt template path: `docs/agents/lifecycle/codex-maintenance/governance/execute-agent-maintenance-prompt.md`
- prompt sha256: `fc70c83e1f54419ee39fa2a9424a12af8187a7eadaa838a4ef5d9b44ed92162e`
- canonical handoff: `docs/agents/lifecycle/codex-maintenance/HANDOFF.md`
- derivative pr summary: `docs/agents/lifecycle/codex-maintenance/governance/pr-summary.md`
- exact closeout artifact: `docs/agents/lifecycle/codex-maintenance/governance/maintenance-closeout.json`
- branch linkage: `automation/codex-maintenance-0.153.3`
- manual closeout required: `true`

## Writable surfaces

- `docs/agents/lifecycle/codex-maintenance/**`
- `crates/codex/**`
- `crates/agent_api/**`
- `cli_manifests/codex/artifacts.lock.json`
- `cli_manifests/codex/snapshots/0.153.3/**`
- `cli_manifests/codex/reports/0.153.3/**`
- `cli_manifests/codex/versions/0.153.3.json`
- `cli_manifests/codex/wrapper_coverage.json`
- `cli_manifests/support_matrix/current.json`
- `docs/specs/unified-agent-api/support-matrix.md`
- `crates/agent_api/src/runtime_support_data.rs`
- `docs/specs/unified-agent-api/non-tui-support-debt.md`
- `docs/specs/codex-wrapper-coverage-scenarios-v1.md`

## Read-only inputs

- `docs/agents/lifecycle/codex-maintenance/OPS_PLAYBOOK.md`
- `docs/agents/lifecycle/codex-maintenance/CI_WORKFLOWS_PLAN.md`
- `docs/agents/lifecycle/codex-maintenance/governance/execute-agent-maintenance-prompt.md`
- `.github/workflows/agent-maintenance-open-pr.yml`
- `docs/specs/unified-agent-api/non-tui-support-debt.md`

## Ordered repo commands

- `cargo fmt --all`
- `cargo run -p xtask -- codex-validate --root cli_manifests/codex`
- `cargo run -p xtask -- support-matrix --check`
- `cargo run -p xtask -- capability-matrix --check`
- `cargo run -p xtask -- capability-matrix-audit`
- `make preflight`

## Exact green gates

- `cargo fmt --all`
- `cargo run -p xtask -- codex-validate --root cli_manifests/codex`
- `cargo run -p xtask -- support-matrix --check`
- `cargo run -p xtask -- capability-matrix --check`
- `cargo run -p xtask -- capability-matrix-audit`
- `make preflight`

## Recovery

- recreate packet command: `cargo run -p xtask -- refresh-agent --request docs/agents/lifecycle/codex-maintenance/governance/maintenance-request.toml --write`
- reopen pr body path: `docs/agents/lifecycle/codex-maintenance/governance/pr-summary.md`
- reopen pr branch: `automation/codex-maintenance-0.153.3`
- notes:
- If PR creation fails after packet generation, rerun packet regeneration from the frozen request and reopen the PR from the generated pr-summary path.
- If the local execution-host preflight (local Codex CLI host via execute-agent-maintenance) fails, fix the Codex binary/auth state and rerun `execute-agent-maintenance --dry-run` before write mode.

## Dry-run to write relay

Use the `run_id` printed by the dry-run output, replacing `RUN_ID_FROM_DRY_RUN` before invoking write mode.

```sh
cargo run -p xtask -- execute-agent-maintenance --dry-run --request docs/agents/lifecycle/codex-maintenance/governance/maintenance-request.toml
cargo run -p xtask -- execute-agent-maintenance --write --request docs/agents/lifecycle/codex-maintenance/governance/maintenance-request.toml --run-id RUN_ID_FROM_DRY_RUN
```

## Exact closeout command

```sh
cargo run -p xtask -- close-agent-maintenance --request docs/agents/lifecycle/codex-maintenance/governance/maintenance-request.toml --closeout docs/agents/lifecycle/codex-maintenance/governance/maintenance-closeout.json
```

## Exact maintained-agent prompt

```md
# Packet PR Maintenance Prompt (`0.153.3`)

This template renders the exact maintained-agent prompt for `codex` packet execution.
`docs/agents/lifecycle/codex-maintenance/HANDOFF.md` remains canonical and `governance/pr-summary.md` is derivative.

@codex

## Goal

Execute the automated maintenance packet for `codex` target `0.153.3`.

## Frozen request contract

- Read `docs/agents/lifecycle/codex-maintenance/governance/maintenance-request.toml` before changing code or docs.
- Read the packet-owned `support_surface_audit` block before deciding whether the run can succeed.
- Treat `docs/agents/lifecycle/codex-maintenance/HANDOFF.md` as canonical for writable surfaces, read-only inputs, ordered commands, green gates, and recovery.
- Treat `.github/workflows/agent-maintenance-open-pr.yml` as the opening workflow source.
- Do not write outside the execution contract frozen in the request packet.

## Manifest inputs

- `cli_manifests/codex/README.md`
- `cli_manifests/codex/VALIDATOR_SPEC.md`
- `cli_manifests/codex/RULES.json`
- `cli_manifests/codex/SCHEMA.json`
- `cli_manifests/codex/current.json`
- `cli_manifests/codex/latest_validated.txt`
- `cli_manifests/codex/wrapper_coverage.json`

## Required workflow

1. Compare the current validated baseline from `cli_manifests/codex/latest_validated.txt` against the target `0.153.3` artifacts.
2. Use `support_surface_audit` to classify newly discovered non-TUI surface, preexisting non-TUI debt, required uplifts, and allowed deferrals.
3. Land bounded wrapper/backend/manifest/publication updates for every row in `required_uplifts_this_run`.
4. Refresh or create version-scoped manifest artifacts under `cli_manifests/codex/snapshots/0.153.3/`, `cli_manifests/codex/reports/0.153.3/`, and `cli_manifests/codex/versions/0.153.3.json` as required by the packet.
5. Leave closeout manual; record it only with `close-agent-maintenance` after the declared green gates pass.

## Done criteria

- Changes stay within the writable surfaces frozen in `docs/agents/lifecycle/codex-maintenance/governance/maintenance-request.toml`.
- No newly discovered non-TUI surface remains unresolved unless the packet records one allowed deferral.
- `cargo run -p xtask -- codex-validate --root cli_manifests/codex` passes.
- The remaining ordered commands and green gates from `docs/agents/lifecycle/codex-maintenance/HANDOFF.md` pass or are captured in maintainer follow-up notes.

```
