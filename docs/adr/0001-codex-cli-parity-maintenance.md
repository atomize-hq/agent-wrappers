# ADR 0001: Codex CLI Parity Maintenance & Release-Trailing Process

Date: 2025-12-24  
Status: Accepted

## Context

This repository provides a long-running Rust binding/wrapper around the OpenAI Codex CLI (`codex`).
The Codex CLI has a fast release cadence and its surface area evolves across:

- Commands/subcommands (e.g., `exec`, `resume`, `apply`, `diff`, `sandbox`, `mcp-server`, `app-server`, `features`, etc.)
- Flags and config overrides (global and per-command)
- JSONL event schemas (`--json`) and server notification schemas (MCP/app-server)
- Optional/experimental features (e.g., cloud utilities, experimental MCP management commands)

An in-depth audit (“`Audit of the Codex Wrapper and Coverage Gaps.docx.md`”) found the wrapper is in good shape for “major stable” CLI coverage and that most remaining gaps are intentional (e.g., interactive TUI mode, shell completion, some experimental surfaces). The biggest long-term risk is drift: new releases can change flags and JSON schemas, and we need a reliable process to detect and respond to changes quickly.

We also observed real-world drift across Codex CLI versions (e.g., JSONL payload shape changes in older binaries), reinforcing the need for:

- Automated change detection
- Strong CI coverage against real binaries
- Compatibility strategies for schema/flag drift

Current version reality (as of this ADR):

- Minimum supported Codex CLI: `0.61.0`
- Latest upstream Codex CLI release: `0.77.0`
- Platform priority: Linux first, then macOS, then Windows (primarily via WSL; native Windows support is best-effort)

This gap gives us room to validate the new “release watch → snapshot diff → update” process on real, recent versions.

## Decision

### 1) Adopt a “CLI Snapshot → Diff → Update” workflow (local first, CI next)

We will implement an automated mechanism that, given a `codex` binary path, generates a structured “snapshot” of the CLI surface and behavior-relevant metadata. At minimum the snapshot should include:

- `codex --version` output (raw + parsed semver when possible)
- `codex --help` and relevant subcommand `--help` outputs (recursive)
- `codex features list` output (JSON when supported; otherwise text)
- Notes for known “help does not show” cases (e.g., `sandbox` platform variants)

We will store one “current supported snapshot” in-repo and use git diff as the primary change signal.
When a new Codex CLI release is targeted, we regenerate the snapshot for the new binary and review the diff to drive code updates, test updates, and documentation updates.

#### Snapshot storage layout (repo)

We will store Codex CLI snapshot artifacts under:

- `cli_manifests/codex/min_supported.txt` — minimum supported Codex CLI version (single semver line)
- `cli_manifests/codex/latest_validated.txt` — latest Codex CLI version that passed our validation matrix (single semver line)
- `cli_manifests/codex/current.json` — the structured snapshot for `latest_validated.txt` (generated)
- `cli_manifests/codex/README.md` — schema notes, generation instructions, and conventions
- (optional) `cli_manifests/codex/raw_help/<version>/**` — raw `--help` captures (generated; used for debugging parser drift)
- (optional) `cli_manifests/codex/supplement/**` — small, explicit “help gaps”/known-omissions supplement files (hand-maintained)

Only `min_supported.txt` and `latest_validated.txt` are authoritative “policy pointers”.
The `current.json` snapshot and any raw help captures are treated as generated artifacts, reviewed via diff.

#### Snapshot schema (v1)

The snapshot is intentionally “diff-first”: stable ordering, small normalized strings, and separate optional raw captures.
Versioned schema (`snapshot_schema_version`) exists so we can evolve snapshot structure without breaking the maintenance loop.

`cli_manifests/codex/current.json` fields (v1; required unless marked optional):

- `snapshot_schema_version` (int): schema version for this JSON structure (start at `1`).
- `tool` (string): `codex-cli`.
- `collected_at` (RFC3339 string): when the snapshot was generated.
- `binary` (object):
  - `sha256` (string): checksum of the binary used for snapshot generation.
  - `size_bytes` (int)
  - `platform` (object): `os`, `arch` (strings; e.g., `linux`, `x86_64`)
  - `version_output` (string): raw `codex --version` output
  - `semantic_version` (string, optional): parsed semver when possible
  - `channel` (string, optional): `stable|beta|nightly|unknown` when derivable
  - `commit` (string, optional): parsed commit hash when available
- `commands` (array; stable-sorted):
  - `path` (array of strings): command/subcommand path tokens (e.g., `["exec","resume"]`)
  - `about` (string, optional): one-line description extracted from help output
  - `usage` (string, optional)
  - `stability` (string, optional): `stable|experimental|beta|deprecated|unknown` when derivable from help text
  - `platforms` (array of strings, optional): if the command is platform-specific (e.g., `["linux","macos"]`)
  - `args` (array of objects, optional): positional args as discoverable from help
  - `flags` (array of objects, optional): discovered options (short/long/value arity, repeatable; may include optional `stability`/`platforms` when derivable)
- `features` (object, optional):
  - `supports_json` (bool, optional): whether `codex features list --json` is accepted
  - `raw_text` (string, optional): raw text output of `codex features list`
  - `raw_json` (object, optional): parsed JSON output of `codex features list --json`
- `known_omissions` (array of strings, optional): any “help supplement” items applied (for review visibility)

We explicitly do **not** treat the snapshot as a perfect semantic model of Codex behavior. It is:
- a structured index for “what commands/flags exist”,
- a stable change signal for maintainers,
- a bridge to targeted wrapper work + tests.

#### Snapshot generator requirements

The snapshot generator must:

- Be exhaustive and recursive: enumerate commands/subcommands from `codex --help`, then run `--help` for every discovered command path until leaf commands are reached.
- Capture both:
  - a parsed, structured inventory (for diffs and planning), and
  - the raw help output (for debugging parser drift).
- Support a small, explicit “supplement” mechanism for known omissions in help text (e.g., `sandbox` platform variants not shown in `--help`) so the resulting snapshot is actually exhaustive.
- Produce deterministic output (stable ordering, canonicalized whitespace where appropriate) so diffs are meaningful.

#### Snapshot diff review requirements

When reviewing a snapshot update PR, treat diffs as a checklist, not just “what’s new”:

- **Additions**: new commands/flags/config toggles → decide wrap vs intentionally unwrapped; add/update E2E coverage where possible.
- **Removals/renames**: anything that disappears from help output is a high-signal potential breaking change → confirm via real-binary smoke tests and update wrapper surface/tests/docs as needed.
- **Deprecations/experimental markers**: if help text adds “deprecated/experimental/beta” labels, capture them in the snapshot (`stability`) and reassess promotion criteria and default exposure.

### 2) Automate “release watch” and “update” workflows (no auto-provision in the crate)

We will not auto-download/auto-update Codex binaries from the core crate at runtime. Instead:

- A nightly **Release Watch** GitHub workflow will check upstream releases, apply our selection heuristic (e.g., “latest stable minus 1”, skipping pre-releases), and alert maintainers when the candidate changes.
- A maintainer-triggered **Update Snapshot** workflow will download a specific version, regenerate snapshots, open a PR with diffs, and run real-binary CI validations.

This keeps supply-chain risk and nondeterminism out of the crate’s default behavior while making maintenance fast and repeatable.

#### Release Watch workflow (nightly, read-only)

The nightly workflow should:

- Query upstream Codex releases/tags.
- Exclude pre-releases/betas/nightlies per policy.
- Compute the “candidate” version (e.g., stable-minus-one).
- Compare candidate vs a repo-tracked “latest validated” pointer (text file or field in the snapshot).
- If different, open/update an issue with:
  - latest stable
  - computed candidate
  - release URLs
  - a checklist for running the update workflow

#### Update Snapshot workflow (workflow_dispatch)

The maintainer-triggered workflow should accept inputs such as:

- `version` (exact version to validate)
- `platforms` (prioritize Linux, then macOS; Windows is primarily via WSL and does not need native Windows CI)
- optional “update min supported” toggle

And it should:

- Download the specified release artifact(s).
- Record checksums (at least `sha256`) into a lockfile-like artifact for traceability.
- Run the snapshot generator to produce:
  - CLI help inventory (recursive)
  - features list output
  - version metadata
- Open a PR updating snapshot files and linking to any detected drift.
- Run real-binary smoke tests as PR checks.

### 3) Use “additional signals” beyond help output (warn/alert, not source-of-truth)

Help output is necessary but not always sufficient. We will augment change detection with:

- **Release notes mining**: extract likely new commands/flags from GitHub release notes (e.g., backticked commands, `--flag` tokens) and include them in the maintainer alert as “signals to verify”.
- **Docs/reference cross-check** (optional): compare against official docs (or curated references) to surface discrepancies as prompts for human review.

These signals should drive investigation and planning, but the structured snapshot + real-binary tests remain the primary source of truth.

### 3.1) Explicitly scoped to Codex CLI (non-goal: multi-CLI manifests)

The audit discusses extending a manifest/diff approach to other agent CLIs (e.g., Claude/Gemini). This ADR is intentionally scoped to **Codex CLI parity maintenance** for this repo. If we later add additional CLIs, we should create a separate ADR rather than broadening this one.

### 4) Define and enforce a version support policy

We will explicitly track:

- **Minimum supported Codex CLI version** (the oldest we commit to keeping compatible)
- **Latest validated Codex CLI version** (the newest we test and support first-class)

#### Definition of “validated”

A Codex CLI version is “validated” when the following pass on Linux:

- `cargo test -p codex` (library + unit tests)
- `cargo test -p codex --examples` (examples compile and run as applicable)
- `cargo test -p codex --test cli_e2e` with a supplied real binary path, using a fully isolated `CODEX_HOME`

Optional (non-gating, opt-in):

- Live/credentialed probes for `exec`/`resume`/`diff`/`apply` (gated behind explicit env vars)
- macOS smoke coverage (added incrementally after Linux baseline is reliable)
- Windows coverage is primarily via WSL (treat as Linux); native Windows CI is optional and can be deferred

#### CI enforcement model

CI will run real-binary E2E smoke tests against at least:

- The pinned “latest validated” binary
- Optionally the “minimum supported” binary (or a representative older binary) to prevent regressions

Live/credentialed tests remain opt-in and gated by environment variables.

### 5) Treat JSONL schemas as versioned and build compatibility layers

We will assume the `--json` event schema can drift between versions (especially across older builds).
Our policy is:

- Prefer typed event parsing, but tolerate schema drift with normalization and “unknown field” capture.
- Do not fail the entire stream on the first parse/normalize error; surface errors to the caller while continuing to read remaining events when possible.
- Maintain a fixtures-based sample corpus for JSONL and server notifications, and refresh it when the live CLI changes.

### 6) Keep “intentionally unwrapped” surfaces explicit and tracked

Some surfaces are not required for programmatic embedding and will remain unwrapped by default:

- Interactive TUI mode (`codex` with no args)
- Shell completion generation (`codex completion …`)
- `codex cloud exec` (experimental/setup-time utility unless it becomes core to embedding)
- Experimental MCP CLI management commands (`codex mcp list/get/add/remove/login/logout`) unless they become stable and necessary

These should still be tracked in the CLI snapshot/manifest so changes are visible and decisions are explicit.

#### Promotion criteria (unwrapped → wrapped)

We will consider promoting an “intentionally unwrapped” surface into the wrapper when:

- It is stable (not marked experimental/beta) for at least 2 stable releases, and
- It is needed for headless embedding (not primarily interactive UX), and
- It can be exercised by non-interactive tests (or can be safely gated behind opt-in live probes), and
- Its failure modes can be surfaced deterministically (exit codes / JSON errors), and
- It does not require us to accept new supply-chain/network behavior in the core crate.

Anything promoted must be represented in:
- the CLI snapshot diff review, and
- a minimal E2E smoke test (real binary) or a documented reason it remains live-only.

### 7) Make real-binary confirmations repeatable and safe by default

Real-binary smoke tests and examples should be:

- Easy to run (“one command”)
- Isolated from user state by default (`CODEX_HOME` isolation)
- Able to reuse provisioned auth by seeding only credential files into the isolated home

## Consequences

### Positive

- New CLI releases become actionable quickly: we can answer “what changed?” via a structured diff.
- Reduced reliance on manual audit and trial-and-error to find drift.
- Clear expectations for users and maintainers on supported versions.
- Stronger guarantees that changes to flags and JSON schemas don’t silently break hosts.

### Tradeoffs / Costs

- Requires maintaining snapshot tooling and CI jobs.
- Some drift is behavioral (not just help text), so the snapshot must be paired with real-binary tests.
- Backward compatibility can increase parser complexity (normalization paths).

## Implementation Notes (current repo direction)

This ADR commits us to building on existing pieces already present in the repo:

- Static inventory: `CLI_MATRIX.md` (currently pegged to older CLI versions)
- Capability probing + cache policies: `crates/codex/src/lib.rs` and `_archived/workstreams/F_versioning_features/BRIEF.md`
- Real-binary E2E harness: `crates/codex/tests/cli_e2e.rs`
- Examples + fixtures: `crates/codex/examples/*` and `crates/codex/examples/fixtures/*`
- Auth isolation + seeding helpers: `CodexHomeLayout::seed_auth_from` and related docs/examples

## Next Steps (for the upcoming planning session)

1) Implement the snapshot generator tool (prefer `xtask` or a small `scripts/` tool) and define the on-disk snapshot format.
2) Decide where snapshots live (e.g., `cli_manifests/`), and which fields are required vs optional.
3) Add the two GitHub workflows:
   - nightly “Release Watch” (alerts only)
   - maintainer-triggered “Update Snapshot” (downloads, generates diffs, runs CI)
4) Add CI jobs that:
   - run `cli_e2e` against the pinned “latest validated” binary
   - optionally run against the minimum supported binary
   - publish snapshot diffs as PR artifacts when a snapshot update is proposed
5) Decide criteria for moving an “experimental/unwrapped” command into the supported wrapper surface.
6) Establish cadence and ownership: who updates snapshots when Codex releases ship and how quickly we trail.
7) Trial-run the process against recent real versions (currently min `0.61.0`, upstream `0.77.0`) to validate the workflow end-to-end.

## Planning Session Notes (local recon)

Local binaries available at the time of writing:

- Repo-local (gitignored) binary: `./codex-x86_64-unknown-linux-musl` → `codex-cli 0.61.0`
- System binary on `PATH`: `codex` → `codex-cli 0.77.0`

Help diff signal (0.61.0 → 0.77.0) includes at least:

- New top-level `codex review` command (and `codex exec review` subcommand)
