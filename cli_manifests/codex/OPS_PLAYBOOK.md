# Codex CLI Parity Ops Playbook

This document is the maintainer runbook for keeping this repo’s Rust wrapper in parity with upstream Codex CLI releases using the “snapshot → diff → update” workflow.

Key policy references:
- Source-of-truth policy/architecture: `docs/adr/0001-codex-cli-parity-maintenance.md`
- Snapshot artifacts home: `cli_manifests/codex/README.md`

## Core Policies (read before operating)

- **No runtime downloads.** The crate must not download or update Codex binaries at runtime. Downloads happen only in CI/workflows (per ADR).
- **Promotion safety.** We only promote snapshots/pointers for upstream releases that are **non-prerelease** and **non-draft**, unless a human explicitly decides otherwise and records that decision in the ops log / PR notes.
- **Authoritative pointers.** Only these files are authoritative:
  - `cli_manifests/codex/min_supported.txt`
  - `cli_manifests/codex/latest_validated.txt`
  `cli_manifests/codex/current.json` is generated and should correspond to `latest_validated.txt`.
- **Help/real-binary reality wins.** Snapshot diffs and real-binary validations are the primary source of truth; “additional signals” (release notes mining, doc cross-checks) are alerts only.

## Release Watch: Triage Checklist

When the nightly Release Watch workflow alerts (issue/comment):

1. Confirm the alert is for a **stable** upstream release (not draft/prerelease).
2. Record:
   - upstream latest stable version
   - computed candidate version (per workflow policy)
   - release URLs (and any notable release-note callouts)
3. Decide whether to run the Update Snapshot workflow now:
   - If the candidate is new and we’re within normal maintenance cadence, proceed.
   - If the release is risky/large, proceed but expect follow-on triads (don’t “quick fix” in the ops PR).

## Run Update Snapshot (workflow_dispatch)

Preferred path: run the GitHub Actions workflow:
- `.github/workflows/codex-cli-update-snapshot.yml`

Required input:
- `version`: the exact upstream version to validate (example: `0.77.0`)

Optional inputs (only if you intend to change policy):
- “update min supported” toggle (if present): update `min_supported.txt` as part of the run

Notes:
- The workflow is responsible for downloading/extracting the upstream release artifact(s) and updating `cli_manifests/codex/artifacts.lock.json`.
- The workflow should regenerate `cli_manifests/codex/current.json` (and optionally `cli_manifests/codex/raw_help/<version>/**`) using `xtask`.

## Review Snapshot Diff (treat as a checklist)

In the snapshot PR, review diffs in `cli_manifests/codex/current.json` (and any `raw_help/**`) as a checklist:

- **Additions:** new commands/flags/config toggles
  - Decide: wrap now vs intentionally unwrapped (see below)
  - If wrap: plan follow-on triads (code/test/integ) for the new surface(s)
- **Removals/renames:** anything disappearing from help is high-signal
  - Confirm against the real binary and release notes
  - Plan compatibility shims or breaking-change handling as needed
- **Stability markers:** new “experimental/beta/deprecated” labels
  - Capture in snapshot (`stability`) and reassess whether the wrapper should expose it
- **Schema drift signals:** JSON/notification schema hints in help output or release notes
  - Use as prompts for review, but treat fixtures + real-binary outputs as the arbiter

### Intentionally Unwrapped Surfaces (explicit policy)

These surfaces are intentionally *not* wrapped unless/until they meet promotion criteria:

- Interactive TUI mode (`codex` with no args)
- Shell completion generation (`codex completion …`)
- `codex cloud exec` (experimental/setup-time utility unless it becomes core to embedding)
- Experimental MCP management commands (`codex mcp list/get/add/remove/login/logout`) unless they become stable and necessary

### Promotion Criteria (unwrapped → wrapped)

Promote an intentionally unwrapped surface to “wrapped” only when it meets all of the following:

1. **Stable enough:** marked stable (not experimental/beta) for at least **2 stable releases**.
2. **Embedding need:** needed for headless embedding (not primarily interactive UX).
3. **Testable:** exercisable via non-interactive tests (or explicitly gated live probes).
4. **Deterministic failures:** clear exit codes and/or machine-readable JSON errors.
5. **No new supply-chain behavior:** does not require new network/download behavior in the core crate.

## Update Pointers Safely (`min_supported.txt` / `latest_validated.txt`)

Use these rules when updating pointers:

- Only update `latest_validated.txt` after validations succeed (see next section) for the exact version you’re promoting.
- Only update `min_supported.txt` intentionally (policy change). Prefer leaving it stable until we have explicit intent + coverage.
- Ensure `cli_manifests/codex/current.json` corresponds to `latest_validated.txt` (same semantic version, generated from the validated binary).

## “Validated” Commands (Linux, isolated home)

Run the “validated” commands with an isolated home directory by setting **both** `CODEX_E2E_HOME` and `CODEX_HOME` to the same temporary directory.

Example (from repo root):

```bash
CODEX_E2E_HOME="$(mktemp -d)"
export CODEX_E2E_HOME
export CODEX_HOME="$CODEX_E2E_HOME"
export CODEX_E2E_BINARY="./codex-x86_64-unknown-linux-musl"

cargo test -p codex
cargo test -p codex --examples
cargo test -p codex --test cli_e2e -- --nocapture
```

Notes:
- Live/credentialed probes (if any) must remain opt-in and explicitly gated (do not enable by default in CI).
- If you are validating a different binary path, update `CODEX_E2E_BINARY` accordingly.

## Additional Signals (warn/alert only)

Use these signals to prompt human review; they are not the source of truth:

- **Release notes mining:** scan upstream release notes for backticked commands and `--flag` tokens; treat as “verify in snapshot/help/binary”.
- **Optional docs/reference cross-check:** compare to official docs as prompts for discrepancies; confirm via snapshot + real-binary behavior.

## Trial Run: 0.61.0 → 0.77.0 (Linux)

This is a one-time (or occasional) checklist to validate the operational loop on Linux using the known gap described in ADR 0001.

### Prereqs

- A locally available **0.61.0** Linux musl binary placed at:
  - `./codex-x86_64-unknown-linux-musl`
  This is a gitignored workspace artifact. CI obtains it by downloading/extracting the upstream asset `codex-x86_64-unknown-linux-musl.tar.gz`.
- A locally available **0.77.0** binary, one of:
  - `codex` on `PATH`, or
  - obtained via the Update Snapshot workflow, or
  - stored at `./.codex-bins/0.77.0/codex-x86_64-unknown-linux-musl` and symlinked into `./codex-x86_64-unknown-linux-musl` when running validations.

### Checklist

1. Snapshot 0.61.0 (baseline):
   - `cargo run -p xtask -- codex-snapshot --codex-binary ./codex-x86_64-unknown-linux-musl --out-dir cli_manifests/codex --capture-raw-help --supplement cli_manifests/codex/supplement/commands.json`
2. Snapshot 0.77.0 (candidate):
   - Point the generator at the 0.77.0 binary path (or update the symlink) and rerun the same command.
3. Review diffs as a checklist:
   - additions / removals / renames / stability markers
   - any “help omissions” that should be tracked via `cli_manifests/codex/supplement/commands.json`
4. Run Linux validations with isolated home:
   - Set `CODEX_E2E_HOME` and `CODEX_HOME` to the same `mktemp -d` directory
   - Set `CODEX_E2E_BINARY=./codex-x86_64-unknown-linux-musl` (pointing at the candidate binary)
   - Run: `cargo test -p codex`, `cargo test -p codex --examples`, `cargo test -p codex --test cli_e2e -- --nocapture`
5. Promote pointers (only after validation):
   - Update `cli_manifests/codex/latest_validated.txt` to `0.77.0`
   - Ensure `cli_manifests/codex/current.json` corresponds to `0.77.0`
   - Leave `min_supported.txt` at `0.61.0` unless you are explicitly changing policy

