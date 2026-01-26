# C3-spec – Release-trailing ops playbook + promotion rules

Source: `docs/adr/0001-codex-cli-parity-maintenance.md`

## Decisions (no ambiguity)
- Ops playbook file (exact path): `cli_manifests/codex/OPS_PLAYBOOK.md`
- `cli_manifests/codex/README.md` must link to the ops playbook under a section titled `## Ops Playbook`.
- Trial-run checklist must be included as a dedicated section inside `cli_manifests/codex/OPS_PLAYBOOK.md` titled `## Trial Run: 0.61.0 → 0.77.0 (Linux)`.

## Task Breakdown (no ambiguity)
- `C3-code` (non-test changes):
  - Create `cli_manifests/codex/OPS_PLAYBOOK.md` with the required sections/checklists.
  - Update `cli_manifests/codex/README.md` to link to the ops playbook under `## Ops Playbook`.
- `C3-test` (tests only):
  - Expected no-op unless C3 introduces new testable Rust logic.
- `C3-integ`:
  - Merge `C3-code` + `C3-test`, reconcile to this spec, and run `cargo fmt`, `cargo clippy ...`, `cargo test -p codex` (if applicable), and `make preflight`.

## Scope
- Make “Codex CLI parity maintenance” operationally repeatable and low-risk by documenting:
  - how maintainers respond to Release Watch alerts,
  - how to run the Update Snapshot workflow and review snapshot diffs,
  - how to decide wrap vs intentionally-unwrapped for new surfaces,
  - how to update `latest_validated.txt` (and optionally `min_supported.txt`) safely.
- Keep “intentionally unwrapped” surfaces explicit and tracked (per ADR):
  - Interactive TUI mode (`codex` with no args)
  - Shell completion generation (`codex completion …`)
  - `codex cloud exec` (experimental/setup-time utility unless it becomes core to embedding)
  - Experimental MCP management commands (`codex mcp list/get/add/remove/login/logout`) unless they become stable and necessary
- Encode ADR promotion criteria (unwrapped → wrapped) in maintainer-facing docs:
  - stable (not experimental/beta) for at least 2 stable releases
  - needed for headless embedding (not primarily interactive UX)
  - exercisable via non-interactive tests (or explicitly gated live probes)
  - deterministic failure modes (exit codes / JSON errors)
  - does not require new supply-chain/network behavior in the core crate
- Document the “additional signals” policy (warn/alert only; not source of truth):
  - release notes mining (commands/flags tokens)
  - optional docs/reference cross-check prompts
- Provide a trial-run checklist to validate the process on Linux for the known gap noted in ADR 0001 (min `0.61.0` → upstream `0.77.0` at time of writing).

## Acceptance Criteria
- `cli_manifests/codex/OPS_PLAYBOOK.md` exists and provides copy/paste-able steps for:
  - responding to a Release Watch alert,
  - running Update Snapshot with an exact `version`,
  - reviewing `cli_manifests/codex/current.json` diffs as a checklist (additions/removals/renames/deprecations),
  - executing the ADR “validated” commands on Linux with an isolated home (set both `CODEX_E2E_HOME` and `CODEX_HOME` to the same temporary directory).
- Documentation explicitly lists:
  - unwrapped surfaces and promotion criteria,
  - the policy that downloads occur only in CI/workflows (not crate runtime),
  - the authoritative role of `min_supported.txt` and `latest_validated.txt`.
- `cli_manifests/codex/OPS_PLAYBOOK.md` includes `## Trial Run: 0.61.0 → 0.77.0 (Linux)` that references:
  - repo-pinned binary `./codex-x86_64-unknown-linux-musl` (0.61.0)
  - system `codex` on PATH (0.77.0 at time of ADR) or the Update Snapshot workflow for obtaining 0.77.0

## Out of Scope
- Implementing new wrapper features discovered during snapshot diffs (those are follow-on triads).
- Making Release Watch or Update Snapshot fully autonomous (human review remains required).
- Expanding parity maintenance to other CLIs.
