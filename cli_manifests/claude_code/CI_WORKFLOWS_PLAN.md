# Claude Code CLI Parity: CI Triggers + Binary Acquisition Plan (v1)

This document describes the intended CI automation for **Claude Code CLI** parity:
- when workflows trigger,
- how upstream Claude Code binaries are acquired (from the Claude Code distribution bucket),
- how artifacts are generated (pins → snapshots → union → reports → triad scaffold),
- and how PR branches are managed for safe automation.

Normative contracts for artifact shapes and semantics live in:
- `cli_manifests/claude_code/SCHEMA.json`
- `cli_manifests/claude_code/RULES.json`
- `cli_manifests/claude_code/VERSION_METADATA_SCHEMA.json`
- `cli_manifests/claude_code/VALIDATOR_SPEC.md`

## Goals

- Track the upstream `stable` pointer conservatively.
- Download only pinned artifacts; verify size + sha256 from upstream `manifest.json`.
- Generate per-target snapshots and a union snapshot.
- Generate wrapper coverage + coverage reports + version metadata.
- Auto-generate a triad scaffold from the coverage delta to create a “next work pack”.
- Open/update the maintenance PR branch family `automation/<agent_id>-maintenance-<target_version>` so automation can safely push commits.

## Triggers (when CI runs)

### 1) Release Watch (scheduled + manual)

Workflow: `.github/workflows/agent-maintenance-release-watch.yml`

Trigger:
- schedule (nightly), plus `workflow_dispatch`

Responsibilities:
- Run `cargo run -p xtask -- maintenance-watch --emit-json _ci_tmp/maintenance-watch.json`.
- Build the shared `stale_agents[]` queue from registry truth rather than keeping a per-agent watcher.
- Dispatch the shared packet opener `.github/workflows/agent-maintenance-open-pr.yml`, which opens the packet PR and then calls the reusable acquisition lane against that branch.
- The deleted per-agent watcher workflows are not live entrypoints.

### 2) Acquisition (reusable lane, called by the packet opener)

Workflow: `.github/workflows/parity-acquire.yml` — one agent-agnostic lane shared by every agent.

Trigger:
- `workflow_call` from `.github/workflows/agent-maintenance-open-pr.yml`, and `workflow_dispatch`
  for manual replay, with inputs:
  - `agent_id`
  - `target_version`
  - `ref` (branch to check out and commit to; default `staging`)
  - `commit` (commit the acquired artifacts onto `ref`)

There are no per-agent inputs and no per-agent branching in the workflow. The release source, each
target's runner, the download URL, the archive shape, the snapshot command and the engine commands
are all read from the `acquisition` block in `cli_manifests/claude_code/RULES.json` via
`xtask manifest-acquisition-plan`, whose emitted `include` array is used directly as the job
matrix.

Responsibilities:
- Download upstream `manifest.json` for `version`.
- Acquire per-target binaries for each expected target from `RULES.json.union.expected_targets`.
- Verify integrity (sha256 + size).
- Update `cli_manifests/claude_code/artifacts.lock.json`.
- Generate per-target snapshots (schema v1) + raw help capture (CI artifacts only).
- Generate union snapshot (schema v2) on Linux.
- Generate wrapper coverage + reports + version metadata; validate the parity root.
- Run `prepare-agent-maintenance --write` and open PR branch `automation/claude_code-maintenance-<target_version>`.
- Use generated `docs/agents/lifecycle/claude_code-maintenance/governance/pr-summary.md` for the PR body; `docs/agents/lifecycle/claude_code-maintenance/HANDOFF.md` remains canonical and workflow YAML stays transport-only.

### 3) Promote (manual gate)

Promotion-only pointer updates remain a maintainer action outside the automated maintenance packet. They are not part of the `automation/<agent_id>-maintenance-<target_version>` branch family and should be handled in a separate PR.

## Target Matrix (v1)

Defined by `cli_manifests/claude_code/RULES.json`:
- required target: `linux-x64`
- best-effort targets: `darwin-arm64`, `win32-x64`

## Pins (artifacts.lock.json)

File: `cli_manifests/claude_code/artifacts.lock.json`

Source of truth:
- upstream channel: `stable`
- upstream bucket root: `upstream.bucket_root`

Each pin records:
- `claude_code_version`
- `target_triple`
- `download_url`
- `sha256`
- `size_bytes`
- `asset_name`

## Outputs (committed to the PR)

- `cli_manifests/claude_code/artifacts.lock.json`
- `cli_manifests/claude_code/snapshots/<version>/<target_triple>.json`
- `cli_manifests/claude_code/snapshots/<version>/union.json`
- `cli_manifests/claude_code/wrapper_coverage.json`
- `cli_manifests/claude_code/reports/<version>/**`
- `cli_manifests/claude_code/versions/<version>.json`
- `.archived/project_management/next/claude-code-cli-parity-<version>/**`
