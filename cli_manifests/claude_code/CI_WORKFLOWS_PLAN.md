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
- Open/update a dedicated PR branch per upstream version so automation can safely push commits.

## Triggers (when CI runs)

### 1) Release Watch (scheduled + manual)

Workflow: `.github/workflows/claude-code-release-watch.yml`

Trigger:
- schedule (nightly), plus `workflow_dispatch`

Responsibilities:
- Read upstream `stable` pointer from the distribution bucket.
- Compare to `cli_manifests/claude_code/latest_validated.txt`.
- If the stable pointer is strictly newer, dispatch Update Snapshot for that version.

### 2) Update Snapshot / Parity PR (workflow_dispatch, dispatched by Release Watch)

Workflow: `.github/workflows/claude-code-update-snapshot.yml`

Trigger:
- `workflow_dispatch` with input:
  - `version` (bare semver, example: `2.1.29`)

Responsibilities:
- Download upstream `manifest.json` for `version`.
- Acquire per-target binaries for each expected target from `RULES.json.union.expected_targets`.
- Verify integrity (sha256 + size).
- Update `cli_manifests/claude_code/artifacts.lock.json`.
- Generate per-target snapshots (schema v1) + raw help capture (CI artifacts only).
- Generate union snapshot (schema v2) on Linux.
- Generate wrapper coverage + reports + version metadata; validate the parity root.
- Generate a triad scaffold pack in `docs/project_management/next/`.
- Open PR branch `automation/claude-code-<version>`.

### 3) Promote (manual gate)

Workflow: `.github/workflows/claude-code-promote.yml`

Trigger:
- `workflow_dispatch` with input:
  - `version`

Responsibilities:
- Update:
  - `cli_manifests/claude_code/latest_validated.txt`
  - `cli_manifests/claude_code/pointers/latest_validated/<target>.txt`
  - `cli_manifests/claude_code/current.json`
- Validate and open PR branch `automation/claude-code-promote-<version>`.

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
- `docs/project_management/next/claude-code-cli-parity-<version>/**`
