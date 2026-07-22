<!-- generated-by: xtask agent-maintenance renderer; source-of-truth: governance/maintenance-request.toml -->

# PR summary

Automated maintenance packet for `codex` target `0.144.6`.

- canonical execution contract: `docs/agents/lifecycle/codex-maintenance/HANDOFF.md`
- request artifact: `docs/agents/lifecycle/codex-maintenance/governance/maintenance-request.toml`
- branch: `automation/codex-maintenance-0.144.6`
- opened from: `.github/workflows/agent-maintenance-open-pr.yml`
- prompt sha256: `9e7a6f4055d32dabd4235ec29f45b909066e992d47983275f5fd38f1ea5f0a1a`

## Support-surface audit

- required: `true`
- pre-run debt count: `2`
- expected post-run debt count: `2`
- discovered upstream surface rows: `68`
- preexisting unsupported rows: `2`
- required uplifts this run:
- `codex archive` `archive` via `new_upstream_surface`
- `codex delete` `delete` via `new_upstream_surface`
- `codex doctor` `doctor` via `new_upstream_surface`
- `codex remote-control` `remote-control` via `new_upstream_surface`
- `codex unarchive` `unarchive` via `new_upstream_surface`
- `codex app-server` `--stdio` via `new_upstream_surface`
- `codex app-server daemon bootstrap` `--remote-control` via `new_upstream_surface`
- `codex delete` `--force` via `new_upstream_surface`
- `codex doctor` `--all` via `new_upstream_surface`
- `codex doctor` `--ascii` via `new_upstream_surface`
- `codex doctor` `--json` via `new_upstream_surface`
- `codex doctor` `--no-color` via `new_upstream_surface`
- `codex doctor` `--summary` via `new_upstream_surface`
- `codex exec resume` `--output-schema` via `new_upstream_surface`
- `codex exec review` `--output-schema` via `new_upstream_surface`
- `codex exec-server` `--environment-id` via `new_upstream_surface`
- `codex exec-server` `--use-agent-identity-auth` via `new_upstream_surface`
- `codex mcp add` `--oauth-client-id` via `new_upstream_surface`
- `codex mcp add` `--oauth-resource` via `new_upstream_surface`
- `codex plugin add` `--json` via `new_upstream_surface`
- `codex plugin add` `--marketplace` via `new_upstream_surface`
- `codex plugin list` `--available` via `new_upstream_surface`
- `codex plugin list` `--json` via `new_upstream_surface`
- `codex plugin list` `--marketplace` via `new_upstream_surface`
- `codex plugin marketplace add` `--json` via `new_upstream_surface`
- `codex plugin marketplace list` `--json` via `new_upstream_surface`
- `codex plugin marketplace remove` `--json` via `new_upstream_surface`
- `codex plugin marketplace upgrade` `--json` via `new_upstream_surface`
- `codex plugin remove` `--json` via `new_upstream_surface`
- `codex plugin remove` `--marketplace` via `new_upstream_surface`
- `codex remote-control` `--json` via `new_upstream_surface`
- `codex remote-control pair` `--json` via `new_upstream_surface`
- `codex remote-control start` `--json` via `new_upstream_surface`
- `codex remote-control stop` `--json` via `new_upstream_surface`
- `codex sandbox` `--allow-unix-socket` via `new_upstream_surface`
- `codex sandbox` `--include-managed-config` via `new_upstream_surface`
- `codex sandbox` `--log-denials` via `new_upstream_surface`
- `codex sandbox` `--permission-profile` via `new_upstream_surface`
- `codex sandbox` `--sandbox-state-disable-network` via `new_upstream_surface`
- `codex sandbox` `--sandbox-state-json` via `new_upstream_surface`
- `codex sandbox` `--sandbox-state-readable-root` via `new_upstream_surface`
- `codex` `--dangerously-bypass-hook-trust` via `new_upstream_surface`
- `codex` `--strict-config` via `new_upstream_surface`
- `codex app-server daemon help` `COMMAND` via `new_upstream_surface`
- `codex archive` `SESSION` via `new_upstream_surface`
- `codex delete` `SESSION` via `new_upstream_surface`
- `codex plugin add` `PLUGIN[@MARKETPLACE]` via `new_upstream_surface`
- `codex plugin remove` `PLUGIN[@MARKETPLACE]` via `new_upstream_surface`
- `codex remote-control help` `COMMAND` via `new_upstream_surface`
- `codex sandbox` `COMMAND` via `new_upstream_surface`
- `codex unarchive` `SESSION` via `new_upstream_surface`
- `codex app-server daemon` `daemon` via `new_upstream_surface`
- `codex app-server daemon bootstrap` `bootstrap` via `new_upstream_surface`
- `codex app-server daemon disable-remote-control` `disable-remote-control` via `new_upstream_surface`
- `codex app-server daemon enable-remote-control` `enable-remote-control` via `new_upstream_surface`
- `codex app-server daemon help` `help` via `new_upstream_surface`
- `codex app-server daemon restart` `restart` via `new_upstream_surface`
- `codex app-server daemon start` `start` via `new_upstream_surface`
- `codex app-server daemon stop` `stop` via `new_upstream_surface`
- `codex app-server daemon version` `version` via `new_upstream_surface`
- `codex plugin add` `add` via `new_upstream_surface`
- `codex plugin list` `list` via `new_upstream_surface`
- `codex plugin marketplace list` `list` via `new_upstream_surface`
- `codex plugin remove` `remove` via `new_upstream_surface`
- `codex remote-control help` `help` via `new_upstream_surface`
- `codex remote-control pair` `pair` via `new_upstream_surface`
- `codex remote-control start` `start` via `new_upstream_surface`
- `codex remote-control stop` `stop` via `new_upstream_surface`
- deferred preexisting gaps:
- `codex completion` `completion` via `requires_new_architectural_seam` (TODOS.md#close-codex-completion-maintenance-gap)
- `codex completion` `SHELL` via `requires_new_architectural_seam` (TODOS.md#close-codex-completion-maintenance-gap)


## Next step

Follow `docs/agents/lifecycle/codex-maintenance/HANDOFF.md` exactly. This PR summary is derivative from the same execution-packet renderer.

## Exact maintained-agent prompt

```md
# Packet PR Maintenance Prompt (`0.144.6`)

This template renders the exact maintained-agent prompt for `codex` packet execution.
`docs/agents/lifecycle/codex-maintenance/HANDOFF.md` remains canonical and `governance/pr-summary.md` is derivative.

@codex

## Goal

Execute the automated maintenance packet for `codex` target `0.144.6`.

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

1. Compare the current validated baseline from `cli_manifests/codex/latest_validated.txt` against the target `0.144.6` artifacts.
2. Use `support_surface_audit` to classify newly discovered non-TUI surface, preexisting non-TUI debt, required uplifts, and allowed deferrals.
3. Land bounded wrapper/backend/manifest/publication updates for every row in `required_uplifts_this_run`.
4. Refresh or create version-scoped manifest artifacts under `cli_manifests/codex/snapshots/0.144.6/`, `cli_manifests/codex/reports/0.144.6/`, and `cli_manifests/codex/versions/0.144.6.json` as required by the packet.
5. Leave closeout manual; record it only with `close-agent-maintenance` after the declared green gates pass.

## Done criteria

- Changes stay within the writable surfaces frozen in `docs/agents/lifecycle/codex-maintenance/governance/maintenance-request.toml`.
- No newly discovered non-TUI surface remains unresolved unless the packet records one allowed deferral.
- `cargo run -p xtask -- codex-validate --root cli_manifests/codex` passes.
- The remaining ordered commands and green gates from `docs/agents/lifecycle/codex-maintenance/HANDOFF.md` pass or are captured in maintainer follow-up notes.

```
