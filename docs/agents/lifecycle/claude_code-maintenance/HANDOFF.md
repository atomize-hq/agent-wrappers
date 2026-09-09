<!-- generated-by: xtask agent-maintenance renderer; source-of-truth: governance/maintenance-request.toml -->

# Handoff

This file is the canonical contributor execution contract for `claude_code` maintenance.

## Packet origin

- detected_by: `.github/workflows/agent-maintenance-release-watch.yml`
- current_validated: `2.1.29`
- target_version: `2.1.236`
- latest_stable: `2.1.236`
- version_policy: `upstream_stable_pointer`
- source_kind: `npm_dist_tag`
- source_ref: `@anthropic-ai/claude-code#stable`
- dispatch_kind: `packet_pr`
- dispatch_workflow: `agent-maintenance-open-pr.yml`
- branch_name: `automation/claude_code-maintenance-2.1.236`

## Support-surface audit

- required: `true`
- pre-run debt count: `2`
- expected post-run debt count: `2`
- discovered upstream surface rows: `0`
- preexisting unsupported rows: `2`
- required uplifts this run:
- none
- deferred preexisting gaps:
- `claude install` `install` via `requires_new_architectural_seam` (TODOS.md#close-claude-code-install-maintenance-gap)
- `claude install` `--force` via `requires_new_architectural_seam` (TODOS.md#close-claude-code-install-maintenance-gap)


## Relay contract

- maintained agent packet: `claude_code`
- local execution host: `local Codex CLI host via execute-agent-maintenance`
- executor surface: `execute-agent-maintenance`
- request artifact: `docs/agents/lifecycle/claude_code-maintenance/governance/maintenance-request.toml`
- prompt template path: `docs/agents/lifecycle/claude_code-maintenance/governance/execute-agent-maintenance-prompt.md`
- prompt sha256: `6d308aea034c6b63313e600fead74aee0794dae591646edbdecf6ffd6d0fee3b`
- canonical handoff: `docs/agents/lifecycle/claude_code-maintenance/HANDOFF.md`
- derivative pr summary: `docs/agents/lifecycle/claude_code-maintenance/governance/pr-summary.md`
- exact closeout artifact: `docs/agents/lifecycle/claude_code-maintenance/governance/maintenance-closeout.json`
- branch linkage: `automation/claude_code-maintenance-2.1.236`
- manual closeout required: `true`

## Writable surfaces

- `docs/agents/lifecycle/claude_code-maintenance/**`
- `crates/claude_code/**`
- `crates/agent_api/**`
- `cli_manifests/claude_code/artifacts.lock.json`
- `cli_manifests/claude_code/snapshots/2.1.236/**`
- `cli_manifests/claude_code/reports/2.1.236/**`
- `cli_manifests/claude_code/versions/2.1.236.json`
- `cli_manifests/claude_code/wrapper_coverage.json`
- `cli_manifests/support_matrix/current.json`
- `docs/specs/unified-agent-api/support-matrix.md`
- `crates/agent_api/src/runtime_support_data.rs`
- `docs/specs/unified-agent-api/non-tui-support-debt.md`

## Read-only inputs

- `docs/agents/lifecycle/claude_code-maintenance/OPS_PLAYBOOK.md`
- `docs/agents/lifecycle/claude_code-maintenance/CI_WORKFLOWS_PLAN.md`
- `docs/agents/lifecycle/claude_code-maintenance/governance/execute-agent-maintenance-prompt.md`
- `.github/workflows/agent-maintenance-open-pr.yml`
- `docs/specs/unified-agent-api/non-tui-support-debt.md`

## Ordered repo commands

- `cargo fmt --all`
- `cargo run -p xtask -- codex-validate --root cli_manifests/claude_code`
- `cargo run -p xtask -- support-matrix --check`
- `cargo run -p xtask -- capability-matrix --check`
- `cargo run -p xtask -- capability-matrix-audit`
- `make preflight`

## Exact green gates

- `cargo fmt --all`
- `cargo run -p xtask -- codex-validate --root cli_manifests/claude_code`
- `cargo run -p xtask -- support-matrix --check`
- `cargo run -p xtask -- capability-matrix --check`
- `cargo run -p xtask -- capability-matrix-audit`
- `make preflight`

## Recovery

- recreate packet command: `cargo run -p xtask -- refresh-agent --request docs/agents/lifecycle/claude_code-maintenance/governance/maintenance-request.toml --write`
- reopen pr body path: `docs/agents/lifecycle/claude_code-maintenance/governance/pr-summary.md`
- reopen pr branch: `automation/claude_code-maintenance-2.1.236`
- notes:
- If PR creation fails after packet generation, rerun packet regeneration from the frozen request and reopen the PR from the generated pr-summary path.
- If the local execution-host preflight (local Codex CLI host via execute-agent-maintenance) fails, fix the Codex binary/auth state and rerun `execute-agent-maintenance --dry-run` before write mode.

## Dry-run to write relay

Use the `run_id` printed by the dry-run output, replacing `RUN_ID_FROM_DRY_RUN` before invoking write mode.

```sh
cargo run -p xtask -- execute-agent-maintenance --dry-run --request docs/agents/lifecycle/claude_code-maintenance/governance/maintenance-request.toml
cargo run -p xtask -- execute-agent-maintenance --write --request docs/agents/lifecycle/claude_code-maintenance/governance/maintenance-request.toml --run-id RUN_ID_FROM_DRY_RUN
```

## Exact closeout command

```sh
cargo run -p xtask -- close-agent-maintenance --request docs/agents/lifecycle/claude_code-maintenance/governance/maintenance-request.toml --closeout docs/agents/lifecycle/claude_code-maintenance/governance/maintenance-closeout.json
```

## Exact maintained-agent prompt

```md
# Packet PR Maintenance Prompt (`2.1.236`)

This template renders the exact maintained-agent prompt for `claude_code` packet execution.
`docs/agents/lifecycle/claude_code-maintenance/HANDOFF.md` remains canonical and `governance/pr-summary.md` is derivative.

@codex

## Goal

Execute the automated maintenance packet for `claude_code` target `2.1.236`.

## Frozen request contract

- Read `docs/agents/lifecycle/claude_code-maintenance/governance/maintenance-request.toml` before changing code or docs.
- Read the packet-owned `support_surface_audit` block before deciding whether the run can succeed.
- Treat `docs/agents/lifecycle/claude_code-maintenance/HANDOFF.md` as canonical for writable surfaces, read-only inputs, ordered commands, green gates, and recovery.
- Treat `.github/workflows/agent-maintenance-open-pr.yml` as the opening workflow source.
- Do not write outside the execution contract frozen in the request packet.

## Manifest inputs

- `cli_manifests/claude_code/README.md`
- `cli_manifests/claude_code/VALIDATOR_SPEC.md`
- `cli_manifests/claude_code/RULES.json`
- `cli_manifests/claude_code/SCHEMA.json`
- `cli_manifests/claude_code/current.json`
- `cli_manifests/claude_code/latest_validated.txt`
- `cli_manifests/claude_code/wrapper_coverage.json`

## Required workflow

1. Compare the current validated baseline from `cli_manifests/claude_code/latest_validated.txt` against the target `2.1.236` artifacts.
2. Use `support_surface_audit` to classify newly discovered non-TUI surface, preexisting non-TUI debt, required uplifts, and allowed deferrals.
3. Land bounded wrapper/backend/manifest/publication updates for every row in `required_uplifts_this_run`.
4. Refresh or create version-scoped manifest artifacts under `cli_manifests/claude_code/snapshots/2.1.236/`, `cli_manifests/claude_code/reports/2.1.236/`, and `cli_manifests/claude_code/versions/2.1.236.json` as required by the packet.
5. Leave closeout manual; record it only with `close-agent-maintenance` after the declared green gates pass.

## Done criteria

- Changes stay within the writable surfaces frozen in `docs/agents/lifecycle/claude_code-maintenance/governance/maintenance-request.toml`.
- No newly discovered non-TUI surface remains unresolved unless the packet records one allowed deferral.
- `cargo run -p xtask -- codex-validate --root cli_manifests/claude_code` passes.
- The remaining ordered commands and green gates from `docs/agents/lifecycle/claude_code-maintenance/HANDOFF.md` pass or are captured in maintainer follow-up notes.

```
