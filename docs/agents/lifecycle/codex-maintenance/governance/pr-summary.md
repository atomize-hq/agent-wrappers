<!-- generated-by: xtask agent-maintenance renderer; source-of-truth: governance/maintenance-request.toml -->

# PR summary

Automated maintenance packet for `codex` target `0.153.3`.

- canonical execution contract: `docs/agents/lifecycle/codex-maintenance/HANDOFF.md`
- request artifact: `docs/agents/lifecycle/codex-maintenance/governance/maintenance-request.toml`
- branch: `automation/codex-maintenance-0.153.3`
- opened from: `.github/workflows/agent-maintenance-open-pr.yml`
- prompt sha256: `fc70c83e1f54419ee39fa2a9424a12af8187a7eadaa838a4ef5d9b44ed92162e`

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


## Next step

Follow `docs/agents/lifecycle/codex-maintenance/HANDOFF.md` exactly. This PR summary is derivative from the same execution-packet renderer.

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
