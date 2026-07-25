# Codex CLI Parity: CI Triggers + Binary Acquisition Plan (v1)

This document describes the intended CI automation for Codex CLI parity:
- when workflows trigger,
- how upstream `codex` binaries are acquired (from GitHub Releases),
- how artifacts are generated (snapshots → union → reports → pointers/metadata),
- and how PR branches are managed for safe automation.

Normative contracts for artifact shapes and semantics live in:
- `cli_manifests/codex/SCHEMA.json`
- `cli_manifests/codex/RULES.json`
- `cli_manifests/codex/VERSION_METADATA_SCHEMA.json`
- `cli_manifests/codex/VALIDATOR_SPEC.md`

## Goals

- Pull upstream binaries directly from GitHub Releases assets for stable releases.
- Avoid prerelease/draft tags and avoid alpha-style tags (e.g. `rust-vX.Y.Z-alpha.N`).
- Generate multi-platform snapshots (Linux/macOS/Windows) and a union snapshot.
- Open/update the maintenance PR branch family `automation/<agent_id>-maintenance-<target_version>` so automation can safely push commits.
- Keep downloads deterministic and integrity-checked (size + sha256 pinned in-repo).
- Support orgs that disable write permissions for `GITHUB_TOKEN` by using a dedicated automation token (or a deterministic artifact fallback).

## Triggers (when CI runs)

### 1) Release Watch (scheduled + manual)

Workflow: `.github/workflows/agent-maintenance-release-watch.yml`

Trigger:
- schedule (nightly), plus `workflow_dispatch`

Responsibilities:
- Run `cargo run -p xtask -- maintenance-watch --emit-json _ci_tmp/maintenance-watch.json`.
- Build the shared `stale_agents[]` queue from registry truth rather than reimplementing release selection in YAML.
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
are all read from the `acquisition` block in `cli_manifests/codex/RULES.json` via
`xtask manifest-acquisition-plan`, whose emitted `include` array is used directly as the job
matrix.

Responsibilities:
- Acquire upstream binaries for each expected target (see “Targets” and “Assets” below).
- Update the in-repo lockfile `cli_manifests/codex/artifacts.lock.json` with:
  - `download_url` (direct GitHub release URL)
  - `sha256`
  - `size_bytes`
- Generate per-target snapshots (schema v1) and raw help captures.
- Upload raw help captures as GitHub Actions artifacts (not committed).
- Merge per-target inputs into a union snapshot (schema v2).
- Generate coverage reports from the union snapshot + wrapper coverage.
- Update per-version metadata and per-target pointers (Linux-first v1 promotion rules).
- Run `prepare-agent-maintenance --write` and create or update branch `automation/codex-maintenance-<target_version>` (when permitted).
- Use generated `docs/agents/lifecycle/codex-maintenance/governance/pr-summary.md` for the PR body; `docs/agents/lifecycle/codex-maintenance/HANDOFF.md` remains canonical and workflow YAML stays transport-only.
- If PR creation is not permitted for workflows in the org/repo, fall back to uploading the regenerated files as a workflow artifact for a maintainer to apply in a PR.

### 3) PR CI (validation + enforcement)

Workflow: `.github/workflows/ci.yml` and/or dedicated parity workflows.

Responsibilities:
- Validate that all committed artifacts validate against schemas.
- Enforce semantic invariants per `VALIDATOR_SPEC.md` (deterministic `RULES.json` enforcement).
- Run wrapper test matrix (Linux is required target in v1).

## Targets (v1)

Minimal expected targets (from `cli_manifests/codex/RULES.json`):
- `x86_64-unknown-linux-musl` (required / promotion anchor)
- `aarch64-apple-darwin`
- `x86_64-pc-windows-msvc`

Platform mapping:
- `x86_64-unknown-linux-musl` → `linux`
- `aarch64-apple-darwin` → `macos`
- `x86_64-pc-windows-msvc` → `windows`

## Upstream Release Selection Rules

Upstream repo: `openai/codex`

We treat as “official stable releases” only those releases where:
- GitHub release flags: `draft=false` and `prerelease=false`
- `tag_name` matches exactly `rust-v<MAJOR.MINOR.PATCH>`
  - examples:
    - allowed: `rust-v0.92.0`
    - disallowed: `rust-v0.92.0-alpha.1` (suffix; not official stable)

## Asset Names and Download URLs

For a given upstream version `VERSION` and tag `TAG=rust-v${VERSION}`:

Direct download URL template (no API required for the actual download step):

`https://github.com/openai/codex/releases/download/${TAG}/${ASSET}`

Expected assets for v1 targets:
- Linux musl:
  - `ASSET=codex-x86_64-unknown-linux-musl.tar.gz`
- macOS arm64:
  - `ASSET=codex-aarch64-apple-darwin.tar.gz`
- Windows x86_64 MSVC:
  - `ASSET=codex-x86_64-pc-windows-msvc.exe`

Notes:
- Upstream also provides `.zst` variants (and DMG/ZIP in some cases). v1 uses tar.gz for *nix and `.exe` for Windows for minimal tooling dependencies.
- Some releases include Sigstore artifacts; v1 uses sha256 + size pinning (in-repo) for integrity.

## Integrity and Determinism

### `artifacts.lock.json` as the pin

`cli_manifests/codex/artifacts.lock.json` is the authoritative pin for upstream assets:
- which asset file is used for each `(version,target_triple)`,
- what URL it is downloaded from (direct URL),
- and its expected sha256 and size.

Consumer jobs must:
- download the asset from the recorded `download_url`,
- verify `size_bytes`,
- verify `sha256`,
- then extract/install to a deterministic path.

### Extraction/install conventions

- `.tar.gz` assets:
  - extract the first non-directory member
  - install to a deterministic filename in the workspace, e.g.:
    - `./codex-x86_64-unknown-linux-musl`
    - `./codex-aarch64-apple-darwin`
- Windows `.exe` assets:
  - download to `./codex-x86_64-pc-windows-msvc.exe`

### Raw help captures

- Raw help captures are uploaded as GitHub Actions artifacts:
  - `cli_manifests/codex/raw_help/<version>/<target_triple>/**`
- They are not committed.
- Union conflicts may include evidence references pointing into this artifact bundle.

## Job Graph (target end state)

1) **Acquire + snapshot (matrix over expected targets)**
- Each OS runner acquires its own codex asset.
- Runs `xtask` snapshot generation for that target.
- Produces:
  - `snapshots/<version>/<target_triple>.json`
  - raw help capture bundle for that target (artifact)

2) **Merge union + generate reports**
- Runs on Linux (or any runner with access to the per-target snapshots as artifacts).
- Produces:
  - `snapshots/<version>/union.json`
  - `reports/<version>/coverage.any.json`
  - `reports/<version>/coverage.all.json` (only if union `complete=true`)
  - `reports/<version>/coverage.<target_triple>.json` (per target)
  - `versions/<version>.json` (status + computed coverage/validation fields as available)
  - per-target pointers under `pointers/`
  - `current.json` updated to match `snapshots/<latest_validated>/union.json` when promoted

3) **Enforce + validate**
- Runs deterministic validators:
  - JSON schema validation
  - `xtask manifest-validate` (RULES enforcement)
- Runs wrapper validation tests (Linux required in v1).

## Branch + PR Strategy

- All automation pushes happen only to the maintenance branch family:
  - `automation/codex-maintenance-<target_version>`
- Automation never pushes directly to `main`, `master`, or `feat/*`.
- If the org disables workflow write permissions for `GITHUB_TOKEN`, the workflow must use a maintainer-provisioned secret (example: `CODEX_AUTOMATION_TOKEN`) with repo write permissions; otherwise it should upload a snapshot artifact bundle and require a manual PR.
- Agents may push commits back to the PR branch to close coverage gaps by:
  - implementing wrapper support, and/or
  - marking `intentionally_unsupported` with rationale (policy-driven).
