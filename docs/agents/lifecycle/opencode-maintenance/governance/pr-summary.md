<!-- generated-by: xtask agent-maintenance renderer; source-of-truth: governance/maintenance-request.toml -->

# PR summary

Automated maintenance packet for `opencode` target `1.18.28`.

- canonical execution contract: `docs/agents/lifecycle/opencode-maintenance/HANDOFF.md`
- request artifact: `docs/agents/lifecycle/opencode-maintenance/governance/maintenance-request.toml`
- branch: `automation/opencode-maintenance-1.18.28`
- opened from: `.github/workflows/agent-maintenance-open-pr.yml`
- prompt sha256: `57f8aa98caabf303e08a3c399afe60a08c693a0346791ea5071443e09e462504`

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


## Next step

Follow `docs/agents/lifecycle/opencode-maintenance/HANDOFF.md` exactly. This PR summary is derivative from the same execution-packet renderer.

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
