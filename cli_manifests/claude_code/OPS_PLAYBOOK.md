# Claude Code CLI Parity Ops Playbook

This is the maintainer runbook for keeping `crates/claude_code` in parity with upstream **Claude Code CLI** releases using the same parity framework used by the Codex wrapper.

Key references:
- Parity framework ADRs: `docs/adr/0001-codex-cli-parity-maintenance.md`, `docs/adr/0003-wrapper-coverage-auto-generation.md`, `docs/adr/0004-wrapper-coverage-iu-subtree-inheritance.md`
- Parity root (schemas/rules): `cli_manifests/claude_code/README.md`
- CI plan: `cli_manifests/claude_code/CI_WORKFLOWS_PLAN.md`
- CI agent runbook: `cli_manifests/claude_code/CI_AGENT_RUNBOOK.md`
- PR body template: `cli_manifests/claude_code/PR_BODY_TEMPLATE.md`
- Workflows:
  - `.github/workflows/claude-code-release-watch.yml`
  - `.github/workflows/claude-code-update-snapshot.yml`
  - `.github/workflows/claude-code-promote.yml`

## Core Policies

- **No runtime downloads.** The wrapper crate must not download/update Claude Code at runtime. Downloads and pins happen in CI workflows only.
- **Channel policy:** automation tracks the upstream `stable` pointer.
- **Targets (v1):**
  - required: `linux-x64`
  - best-effort: `darwin-arm64`, `win32-x64`
- **Authoritative pointers:** `min_supported.txt` and `latest_validated.txt` (plus per-target pointers under `pointers/`). During bootstrap these pointers may be `none`.
- **Promotion safety:** only promote versions that have passed validation on the required target and meet the `RULES.json` gating rules.

## Release Watch: Triage Checklist

When the nightly Release Watch workflow runs (or you run it manually):

1. Read the upstream `stable` pointer from the Claude Code distribution bucket.
2. Compare to `cli_manifests/claude_code/latest_validated.txt`.
3. If the candidate is strictly newer, run the Update Snapshot workflow for that version.

## Update Snapshot (workflow_dispatch)

Preferred path: run the GitHub Actions workflow:
- `.github/workflows/claude-code-update-snapshot.yml`

Required input:
- `version` (bare semver, example: `2.1.29`)

Responsibilities (high level):
- Download `manifest.json` and verify integrity (sha256 + size).
- Update `cli_manifests/claude_code/artifacts.lock.json`.
- Generate per-target help snapshots via `xtask claude-snapshot` (matrix).
- Generate a union snapshot via `xtask claude-union` (Linux).
- Generate wrapper coverage + reports + version metadata + validate:
  - `xtask claude-wrapper-coverage`
  - `xtask codex-report --root cli_manifests/claude_code`
  - `xtask codex-version-metadata --root cli_manifests/claude_code`
  - `xtask codex-validate --root cli_manifests/claude_code`
- Generate a triad scaffold under:
  - `docs/project_management/next/claude-code-cli-parity-<version>/`
- Open a PR branch `automation/claude-code-<version>`.

## Promotion (manual gate)

Promotion is a separate PR so it can be reviewed/approved independently.

Workflow:
- `.github/workflows/claude-code-promote.yml`

Responsibilities:
- Update `latest_validated.txt` and per-target pointers under `pointers/latest_validated/`.
- Update `current.json` to match the promoted union snapshot.
- Re-run `xtask codex-validate --root cli_manifests/claude_code`.
- Open a PR branch `automation/claude-code-promote-<version>`.

## Local Debugging Commands

- Validate committed artifacts:  
  `cargo run -p xtask -- codex-validate --root cli_manifests/claude_code`
- Regenerate wrapper coverage JSON:  
  `cargo run -p xtask -- claude-wrapper-coverage --out cli_manifests/claude_code/wrapper_coverage.json`
- Generate a per-target snapshot (no downloads; uses a local binary):  
  `cargo run -p xtask -- claude-snapshot --claude-binary <PATH> --out-file cli_manifests/claude_code/snapshots/<v>/<target>.json --capture-raw-help --raw-help-target <target> --supplement cli_manifests/claude_code/supplement/commands.json`
- Union snapshots:  
  `cargo run -p xtask -- claude-union --root cli_manifests/claude_code --version <v>`
