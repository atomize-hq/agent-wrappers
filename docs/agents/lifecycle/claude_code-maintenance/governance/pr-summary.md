<!-- generated-by: xtask agent-maintenance renderer; source-of-truth: governance/maintenance-request.toml -->

# PR summary

Automated maintenance packet for `claude_code` target `2.1.236`.

- canonical execution contract: `docs/agents/lifecycle/claude_code-maintenance/HANDOFF.md`
- request artifact: `docs/agents/lifecycle/claude_code-maintenance/governance/maintenance-request.toml`
- branch: `automation/claude_code-maintenance-2.1.236`
- opened from: `.github/workflows/agent-maintenance-open-pr.yml`
- prompt sha256: `6d308aea034c6b63313e600fead74aee0794dae591646edbdecf6ffd6d0fee3b`

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


## Next step

Follow `docs/agents/lifecycle/claude_code-maintenance/HANDOFF.md` exactly. This PR summary is derivative from the same execution-packet renderer.

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
