<!-- generated-by: xtask agent-maintenance renderer; source-of-truth: governance/maintenance-request.toml -->

# codex maintenance

This packet tracks automated upstream-release maintenance for `codex`.

## Request

- request artifact: `docs/agents/lifecycle/codex-maintenance/governance/maintenance-request.toml`
- trigger kind: `upstream_release_detected`
- basis ref: `cli_manifests/codex/latest_validated.txt`
- opened from: `.github/workflows/agent-maintenance-open-pr.yml`
- recorded at: `2026-07-22T00:14:53Z`
- request commit: `df0762ea109af1206b85c585a87030a8e3bbfe38`

## Trigger context

- detected_by: `.github/workflows/agent-maintenance-release-watch.yml`
- current_validated: `0.125.0`
- target_version: `0.144.6`
- latest_stable: `0.145.0`
- version_policy: `latest_stable_minus_one`
- source_kind: `github_releases`
- source_ref: `openai/codex`
- dispatch_kind: `packet_pr`
- dispatch_workflow: `agent-maintenance-open-pr.yml`
- branch_name: `automation/codex-maintenance-0.144.6`

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


## Canonical execution contract

Use `docs/agents/lifecycle/codex-maintenance/HANDOFF.md` as the exact contributor execution contract for this lane. The PR body summary under `docs/agents/lifecycle/codex-maintenance/governance/pr-summary.md` is derivative only.
