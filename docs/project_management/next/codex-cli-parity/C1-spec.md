# C1-spec – Version policy + CI workflows (real-binary validation)

Source: `docs/adr/0001-codex-cli-parity-maintenance.md`

## Decisions (no ambiguity)
- GitHub Actions is the CI/workflow runner for this repo.
- Workflow files (exact paths):
  - Release Watch: `.github/workflows/codex-cli-release-watch.yml`
  - Update Snapshot: `.github/workflows/codex-cli-update-snapshot.yml`
  - CI validation (Linux gate): `.github/workflows/ci.yml`
- Upstream release source: GitHub releases for `openai/codex`.
- Supported platform for automated downloads in C1: Linux `x86_64` musl only (asset name must match `codex-x86_64-unknown-linux-musl`).
  - macOS support is explicitly deferred; Windows is treated as WSL/Linux per ADR.
- Checksum/traceability lockfile (exact path + schema):
  - `cli_manifests/codex/artifacts.lock.json`
  - JSON schema (v1):
    - `version` (int): must be `1`
    - `upstream_repo` (string): must be `openai/codex`
    - `artifacts` (array): stable-sorted by `codex_version`, then `os`, then `arch`
      - `codex_version` (string): semver (example: `0.77.0`)
      - `os` (string): must be `linux`
      - `arch` (string): must be `x86_64`
      - `variant` (string): must be `musl`
      - `asset_name` (string): must be `codex-x86_64-unknown-linux-musl`
      - `download_url` (string)
      - `sha256` (string)
      - `size_bytes` (int)

## Task Breakdown (no ambiguity)
- `C1-code` (non-test changes):
  - Create `.github/workflows/ci.yml`, `.github/workflows/codex-cli-release-watch.yml`, `.github/workflows/codex-cli-update-snapshot.yml`.
  - Create `cli_manifests/codex/artifacts.lock.json` (schema v1) and wire Update Snapshot workflow to update it.
- `C1-test` (tests only):
  - Expected no-op unless C1 introduces new testable Rust logic.
- `C1-integ`:
  - Merge `C1-code` + `C1-test`, reconcile to this spec, and run the ADR “validated” commands (with isolated home) plus `make preflight`.

## Scope
- Enforce the ADR’s version support policy pointers:
  - `cli_manifests/codex/min_supported.txt` is the minimum supported Codex CLI version (single semver line).
  - `cli_manifests/codex/latest_validated.txt` is the newest Codex CLI version validated by our Linux gate (single semver line).
  - These two files are the only authoritative “policy pointers”; `cli_manifests/codex/current.json` is generated for the `latest_validated.txt` version.
- Define “validated” exactly as ADR 0001:
  - Linux gating must include:
    - `cargo test -p codex`
    - `cargo test -p codex --examples`
    - `cargo test -p codex --test cli_e2e` using a supplied real `codex` binary path and a fully isolated `CODEX_HOME`.
  - Optional, non-gating (must remain opt-in via env vars):
    - Live/credentialed probes for `exec`/`resume`/`diff`/`apply`
    - macOS smoke coverage (incremental after Linux baseline)
    - Windows treated as WSL/Linux; native Windows CI can be deferred
- Add CI/workflow automation that **does not** introduce auto-download/auto-update behavior in the crate runtime:
  - Nightly “Release Watch” workflow: read-only upstream check, issue/alert creation, no downloads at runtime.
  - Maintainer-triggered “Update Snapshot” workflow (`workflow_dispatch`):
    - Inputs (exact):
      - `version` (string, required): exact Codex CLI semver to validate (example: `0.77.0`)
      - `update_min_supported` (boolean, default `false`): when true, also update `cli_manifests/codex/min_supported.txt` to `version`
    - Downloads the specified Linux release artifact in CI (asset name `codex-x86_64-unknown-linux-musl`).
    - Records checksums (`sha256` + `size_bytes`) in `cli_manifests/codex/artifacts.lock.json` and commits the update in the PR.
    - Runs the snapshot generator (C0) to update:
      - `cli_manifests/codex/current.json`
      - `cli_manifests/codex/raw_help/<version>/**` (enabled in workflow)
    - Updates `cli_manifests/codex/latest_validated.txt` to `version` in the same PR.
    - Opens a PR with snapshot diffs and runs real-binary validations as PR checks (see “Definition of validated”).
- Document the end-to-end “release watch → snapshot diff → update” process from an operator perspective (high-level; detailed runbook belongs in C3).

### Release Watch workflow details (no ambiguity)
- Schedule: nightly.
- Candidate selection:
  - Fetch latest GitHub releases for `openai/codex`.
  - Filter out prereleases.
  - Sort by semver (descending).
  - Candidate = the *second* newest stable release (stable-minus-one). If only one stable release is available, candidate = newest stable.
- Compare candidate vs the contents of `cli_manifests/codex/latest_validated.txt`.
- If different, open or update a GitHub issue with title:
  - `Codex CLI release watch: candidate <candidate-version>`
- Issue body must include:
  - `latest_validated` (current pointer value)
  - `latest_stable` (newest stable release)
  - `candidate` (stable-minus-one)
  - release URLs for `latest_stable` and `candidate`
  - a short checklist linking to the Update Snapshot workflow

### CI validation workflow details (no ambiguity)
- `ci.yml` must include a Linux job that runs the ADR “validated” checks against the local binary at `./codex-x86_64-unknown-linux-musl` (repo-pinned) by setting:
  - `CODEX_E2E_BINARY=./codex-x86_64-unknown-linux-musl`
- The job must run:
  - `cargo test -p codex`
  - `cargo test -p codex --examples`
  - `CODEX_E2E_BINARY=./codex-x86_64-unknown-linux-musl cargo test -p codex --test cli_e2e -- --nocapture`

## Acceptance Criteria
- `cli_manifests/codex/min_supported.txt` and `cli_manifests/codex/latest_validated.txt` exist and are treated as the only authoritative policy pointers in docs and workflows.
- CI has a Linux gate that runs the ADR “validated” test set against at least the pinned “latest validated” binary.
- Optional CI support exists (or is explicitly deferred with rationale in docs/spec) for:
  - validating the “minimum supported” binary, and
  - macOS smoke checks.
- Two workflows exist at the exact paths specified in “Decisions (no ambiguity)”:
  - `.github/workflows/codex-cli-release-watch.yml`
  - `.github/workflows/codex-cli-update-snapshot.yml`
- `cli_manifests/codex/artifacts.lock.json` exists and is updated by Update Snapshot PRs.

## Out of Scope
- Implementing new wrapper surfaces for newly discovered commands/flags (that work follows snapshot diffs and is handled in separate triads).
- Any crate-runtime behavior that downloads, installs, or updates `codex` binaries.
- Full native Windows CI parity (WSL/Linux is sufficient for initial policy).
