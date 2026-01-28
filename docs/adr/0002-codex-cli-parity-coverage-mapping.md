# ADR 0002: Codex CLI Parity Coverage Mapping (Snapshot → Coverage → Work Queue)

Date: 2026-01-26  
Status: Proposed

## Context

This repository wraps the upstream Codex CLI (`codex`) as a Rust library (`crates/codex`). The upstream CLI changes frequently across:
- commands/subcommands,
- flags/options (global + per-command),
- positional arguments/usage shapes,
- feature-gated surfaces (via `codex features list` and `--enable <FEATURE>`),
- platform-gated surfaces (e.g., `sandbox` variants),
- JSONL event schema and server notifications.

Note: feature stages in `codex features list` may include `deprecated` and `removed` in addition to `stable|beta|experimental`. We record the stage string to support proactive planning before surfaces disappear.

Feature enable policy (for exhaustive discovery):
- Use best-effort enabling of all features listed by `codex features list` except those with stage `removed`.
- If enabling a feature fails, record the failure and continue discovery with the subset of features that successfully enabled.

Scope note (positional parsing semantics):
- The parity/coverage mapping system is help-surface based (commands/flags/positional args as discoverable from `--help` and `Usage:` inference). It does not attempt to model deeper runtime parsing semantics (operand forwarding, `--` handling, prompt placeholder expansion) unless we add a dedicated, probeable “behavioral semantics” layer later.

ADR 0001 established the operational workflow to manage drift:
- generate a deterministic CLI snapshot from a specific `codex` binary,
- review diffs as a checklist,
- validate a version via a real-binary test matrix before promoting `latest_validated.txt`.

What is still missing is an automated, *granular* mapping from:
- “what a specific upstream `codex` binary exposes” (version + platform + enabled feature set; commands/subcommands, global + per-command flags, and positional args/usage shapes) to
- “what the wrapper explicitly supports (and how)”

…so maintainers can produce a clean, structured work queue when a new stable upstream release lands.

### Existing (legacy) inventories

The repo contains older, static inventories used during early planning:
- `capability_manifest.json` (static “supports matrix”, created in the init commit)
- `CLI_MATRIX.md` (static command/flag inventory)

These are useful as historical reference but are not suitable as long-term sources of truth:
- they are not generated deterministically from code or binaries,
- they are not granularly keyed for reliable automated diffs,
- they will drift unless manually maintained.

This ADR defines the replacement system: generated upstream snapshots + generated wrapper coverage + deterministic reports.

## Decision

We will implement a **coverage mapping system** that compares:

1. **Upstream CLI snapshots** (generated from real `codex` binaries, with all features enabled)  
2. A **wrapper coverage manifest** describing what `crates/codex` supports at the command/flag/arg level

…and produces a deterministic **coverage report** that can be converted into triad tasks.

This system is “diff-first” in the sense that its primary output is a **structured coverage delta**, not a prose checklist and not a raw `git diff`:
- Input: two machine-readable inventories (`upstream snapshot` + `wrapper_coverage.json`), keyed by command `path` + flag/arg identity.
- Output: deterministic reports that list added/removed/changed commands, flags, and positional args, plus their wrapper coverage level (`explicit|passthrough|unsupported|intentionally_unsupported|unknown`).

It will avoid network access at crate runtime. Any upstream release discovery/download remains CI/workflow-driven (per ADR 0001).

Critically, the wrapper coverage manifest is not meant to be hand-edited JSON. It must be produced by a deterministic generator (from code and/or wrapper probes) so it can be refreshed by CI/cron with human-in-the-loop gating only.

## Definitions

### Coverage Levels

Each upstream surface is classified as one of:
- `explicit`: first-class API exists in the wrapper (typed request/response or dedicated method).
- `passthrough`: wrapper can drive the surface only via generic argument/override forwarding (weak support).
- `unsupported`: wrapper cannot safely drive the surface today (missing implementation or incomplete semantics); treated as work-queue input.
- `intentionally_unsupported`: we deliberately choose not to support the surface (policy/safety/maintenance reasons); must include a rationale note and should not create perpetual churn in reports.
- `unknown`: not yet assessed (allowed during rollout; treated as work-queue input).

### Surface Units (granular)

The coverage system operates at:
- **Command path**: `["exec","resume"]`, `["sandbox","linux"]`, etc. Root command is `[]`.
- **Flags**: identity key is `long` when present, else `short` (from the upstream snapshot).
- **Positional args**: identity key is `name` (from upstream `Arguments:` and/or `Usage:` inference).

Notes:
- Some upstream help text omits args present in `Usage:`; snapshots infer these and mark them as inferred.
- Some upstream surfaces appear only when enabling feature flags; snapshots record which features were enabled and which commands only appeared when enabled.
- Global flags/options are represented on the root command entry (`path: []`). Comparisons and reports treat global flags at the root scope to avoid repeating the same “missing global flag” across every command.
  - The union snapshot may normalize away per-command duplicates of global flags (same canonical flag key), since the effective flag model is `root.flags ∪ command.flags`.
- Global/top-level positional args are also represented on the root entry (`path: []`). We do not synthesize “inherited” positional args for subcommands; reports treat global positional args at the root scope only.

## Artifacts

### Normative Spec Files (schema + rules)

To eliminate ambiguity, the canonical (machine-checkable) spec for this system lives in:
- `cli_manifests/codex/SCHEMA.json` (JSON Schema; source of truth for artifact shapes)
- `cli_manifests/codex/RULES.json` (merge + comparison rules; source of truth for identity keys, union semantics, and report expectations)

This ADR provides narrative context and rationale; `SCHEMA.json` + `RULES.json` define the exact contract.

CI validation:
- CI must hard-fail if any committed parity artifact (snapshots, wrapper coverage, reports) does not validate against `SCHEMA.json`.
- CI must also validate per-version status metadata files against `VERSION_METADATA_SCHEMA.json`.
CI triggering/binary acquisition:
- The v1 target state is: Release Watch selects stable `rust-v<MAJOR.MINOR.PATCH>` releases (non-draft, non-prerelease, no suffixes) and dispatches Update Snapshot.
- Update Snapshot downloads upstream assets from GitHub Releases (direct download URLs), pins them in `artifacts.lock.json`, and runs multi-platform snapshot generation before union merge/report generation.

Version and timestamp normalization (normative; see `RULES.json`):
- Upstream `binary.semantic_version` is parsed from the codex binary and must be SemVer-like; promotion pointers accept stable `MAJOR.MINOR.PATCH` only.
- To avoid churn, CI should set timestamps deterministically (or omit them where allowed) for committed artifacts.

### Upstream Snapshots

We will store versioned upstream snapshots and treat them as generated artifacts:
- Per-target inputs: `cli_manifests/codex/snapshots/<version>/<target-triple>.json` (schema v1)
- Union snapshot: `cli_manifests/codex/snapshots/<version>/union.json` (schema v2; multi-platform merged view)
- Optional raw help captures for debugging:
  - `cli_manifests/codex/raw_help/<version>/<target-triple>/**` (stored as GitHub Actions artifacts; not committed)

`cli_manifests/codex/current.json` is the convenience pointer for the latest validated union snapshot (schema v2). Per-target snapshots are the required merge inputs and are used for debugging platform-specific drift.

Retention (committed artifacts):
- Keep snapshots + reports for the last 3 validated versions (sliding window), plus the versions referenced by `min_supported.txt` and `latest_validated.txt`.
  - Retention pruning must be mechanical/deterministic (implemented by CI tooling; no LLM decisions).

Version status metadata:
- To avoid overloading the word “validated”, we track per-version workflow state in `cli_manifests/codex/versions/<version>.json`:
  - `snapshotted`: snapshots generated and schema-valid; no wrapper claims.
  - `reported`: coverage report generated; work queue available; no wrapper validation claim.
  - `validated`: passed the validation matrix (promotion-grade for `latest_validated.txt` / `current.json`).
  - `supported`: wrapper coverage meets policy requirements for this version (stronger than validated).

Supported (normative summary; see `RULES.json` for exact contract):
- Requires a complete union snapshot (`complete=true`) with a known `semantic_version`.
- For every upstream command/flag/arg surfaced on a target, wrapper coverage must resolve to one of:
  - `explicit`, `passthrough`, or `intentionally_unsupported`
- `unsupported`, `unknown`, and missing coverage entries are disallowed for `supported`.
- Any `intentionally_unsupported` entry must include a rationale note.

## Automation Loop (CI → PR → Agent → Re-run)

Target end state: the system can run on a schedule (or on demand), open a PR with new upstream snapshots/reports, and use an agent to close the coverage gap by either:
- adding wrapper support (moving surfaces to `explicit` or `passthrough`), or
- explicitly waiving them as `intentionally_unsupported` with rationale (policy-driven).

Workflow sketch:
1. CI generates per-target snapshots + union snapshot + coverage reports and opens/updates a dedicated PR branch for that upstream version.
2. An agent updates wrapper code and wrapper coverage until `is_supported(version)=true` per `RULES.json` (either by implementing support or waiving via `intentionally_unsupported`).
3. CI reruns snapshot/coverage/validation; when requirements are met, the agent/bot may push commits back to the same PR branch (never to mainline branches) and advance `versions/<version>.json.status` accordingly.

Branch hygiene:
- Automation commits must land only on dedicated automation branches/PRs for that version (not directly on `feat/*` or mainline).

## Per-Target Pointers (validated/supported)

Because Linux promotion can proceed even when macOS/Windows signals are missing or lagging, we track pointers per `target_triple`:
- `cli_manifests/codex/pointers/latest_supported/<target_triple>.txt`
- `cli_manifests/codex/pointers/latest_validated/<target_triple>.txt`

Compatibility:
- `cli_manifests/codex/latest_validated.txt` remains the canonical pointer for the required target (`x86_64-unknown-linux-musl`) and must match `pointers/latest_validated/x86_64-unknown-linux-musl.txt`.

Status gates:
- `validated` requires validation passing on the required target and coverage being supported on that target.
- `supported` remains the stricter “supported on all expected targets” designation.

Pointer file format + invariants (normative; see `RULES.json` for exact contract):
- Each pointer file is a single UTF-8 line plus trailing newline: either `<semver>` or `none`.
- Pointer files should always exist for every expected target; use `none` until a value is known.
- `latest_validated.txt` must match `pointers/latest_validated/x86_64-unknown-linux-musl.txt` and must not be `none`.
- `current.json.binary.semantic_version` must match `latest_validated.txt` (Linux-first v1).
- `current.json` must be byte-for-byte identical to `snapshots/<latest_validated>/union.json`.

Snapshots must include:
- a root command entry represented as `path: []` so global flags/args are comparable,
- platform metadata for where the snapshot was generated (at minimum `binary.target_triple`; `os` and `arch` are still recorded as well),
- a parseable `binary.semantic_version` (fail snapshot/union generation if missing/unknown),
- feature probe metadata (what features were enabled during discovery, the `stable|beta|experimental` stage for each feature, and what commands only appeared when enabled).

Supplements:
- Apply `supplement/commands.json` per-target (before union merge). Supplements may be platform-scoped; the union must reflect availability only on the targets the supplement applied to.

### Multi-Platform Discovery and Merge

Some upstream surfaces are platform-gated (or behave differently) across Linux/macOS/Windows (and sometimes by architecture). To make drift detection robust:

- Generate upstream snapshots in CI for each supported OS (Linux/macOS/Windows), using the same snapshot schema and “all features enabled” mode.
- Compare wrapper coverage against snapshots per-platform (so OS-specific gaps don’t get lost).
- Generate a merged “union” view as the canonical snapshot input for comparisons:
  - merged inventory is a union by command `path` + flag/arg identity,
  - each unit records an availability set (which `target_triple`s it appeared on),
  - the coverage report can be filtered to “any platform”, “all platforms”, or a specific platform.
  - when per-target help-derived fields diverge (e.g., flag value-taking semantics, usage text), the union records a `conflicts[]` entry that captures:
    - the targets involved and their observed values, and
    - optional evidence references to raw help captures (path + sha256) so maintainers can inspect the exact help text.

Concrete CI matrix (minimal; can be expanded later):
- Linux: `x86_64-unknown-linux-musl` (required; promotion anchor)
- macOS: `aarch64-apple-darwin`
- Windows: `x86_64-pc-windows-msvc`

Platform mapping (normative; see `RULES.json`):
- `x86_64-unknown-linux-musl` → `linux`
- `aarch64-apple-darwin` → `macos`
- `x86_64-pc-windows-msvc` → `windows`

Partial union policy:
- If the required Linux snapshot is missing, fail the run (no union).
- If macOS/Windows snapshots are missing, emit a union with `complete=false` and an explicit `missing_targets[]` list.
Promotion (v1):
- We allow Linux-first promotion even when `complete=false`, as long as the required target is present, passes validation, and is supported by wrapper coverage on that target.
- macOS/Windows pointers advance independently when their signals are available.

Promotion policy (ADR 0001) can still gate `latest_validated` on Linux-only integration validation, while discovery snapshots remain multi-platform to uncover gated surfaces early.

### Wrapper Capability Snapshot (building block, not a coverage manifest)

`crates/codex` already contains a runtime probe model (`CodexCapabilities`) and an example snapshot generator:
- `cargo run -p codex --example capability_snapshot -- <codex-binary> <out-path> refresh`

This snapshot is useful for wrapper runtime decisions and CI automation (e.g., “what version/features did we actually probe?”), but it is **not** a substitute for `wrapper_coverage.json`:
- it does not enumerate the full command/flag/positional-arg surface area,
- it focuses on “what this binary reports + what probes we ran” rather than “what the wrapper supports”.

We should reuse this capability snapshot as an input to automation (metadata + sanity checks), while keeping the wrapper coverage manifest as a deterministic, generator-produced inventory keyed to upstream `path`/flag/arg identities.

### Wrapper Coverage Manifest

We will maintain a deterministic wrapper coverage manifest, version-controlled:
- `cli_manifests/codex/wrapper_coverage.json`

Schema (v1, conceptual):
- `schema_version` (int)
- `generated_at` (RFC3339, optional)
- `wrapper_version` (string, optional)
- `coverage` (array):
  - `path` (array of strings)
  - `level` (`explicit|passthrough|unsupported|intentionally_unsupported|unknown`)
  - `flags` (array, optional):
    - `key` (`--long` or `-s`)
    - `level`
    - `note` (optional)
  - `args` (array, optional):
    - `name`
    - `level`
    - `note` (optional)
  - `note` (optional)

The emitted JSON is the stable comparison input. The source of truth must live near `crates/codex` so it stays aligned with the actual APIs and is refreshed deterministically.

Implementation guidance:
- Prefer generating `wrapper_coverage.json` from code (explicit mapping tables keyed by upstream `path`/flag/arg identity).
- Allow “passthrough” declarations where only generic argument forwarding exists (so the report can highlight candidates for `explicit` promotion).
- Keep narrative notes separate from key identity whenever possible (notes are helpful, but should not churn diffs).

Scoping semantics (normative; see `RULES.json` for exact contract):
- Multiple coverage entries may share the same `path` as long as their `scope` sets are disjoint.
- For a given `target_triple`, the comparer resolves coverage by selecting the single matching entry in order of specificity:
  1) `scope.target_triples` match
  2) `scope.platforms` match (expanded to the expected targets)
  3) no `scope` (applies to all expected targets)
- If more than one entry matches for the same unit (command path, flag key, or arg name), the wrapper coverage manifest is invalid and comparison must error (no “best effort” guessing).

### Coverage Report

The comparer will produce:
- a machine-readable report (`.json`) for automation and task generation, and
- a human-readable report (`.md`) for maintainers.

The report is generated for a pair: `(upstream snapshot version, wrapper coverage manifest)`.

Platform filter modes (normative; see `RULES.json` for exact contract):
- `any`: surface is present if it appears on at least one available input target.
- `all`: surface is present only if it appears on all expected targets.
- `exact_target`: surface is present only if it appears on the specified target triple.

If the union snapshot is incomplete (`complete=false`), generating an `all` report is an error (use `any` and per-target reports instead).

Report sections (conceptual):
- New commands/flags/args in upstream vs wrapper coverage
- Items present but only `passthrough` (candidates for `explicit` promotion)
- Items marked `unsupported` (missing wrapper support; work-queue input)
- Items marked `intentionally_unsupported` (intentionally unwrapped) and the policy rationale
- Feature-gated additions (commands that appear only when all features are enabled)

## Operating Workflow

When a new upstream stable release is identified (Release Watch / manual):

1. Generate or download the target binary in CI/workflow (no runtime downloads).
2. Generate upstream snapshots with all features enabled across CI OS matrix (Linux/macOS/Windows).
3. Run the coverage comparer per-platform (and optionally against a merged union snapshot) for:
   - `min_supported` snapshot,
   - `latest_validated` snapshot,
   - candidate stable snapshot.
4. Use the report to create a triad work queue (code/test/integ) for missing surfaces.
5. Only after integration validations pass on Linux (per ADR 0001) promote:
   - `cli_manifests/codex/latest_validated.txt`
   - `cli_manifests/codex/current.json` (or pointer) to the validated snapshot

## Consequences

### Benefits
- Produces a deterministic, granular “what changed and what we need to support” list.
- Separates “discovery” (upstream snapshot) from “support/coverage” (wrapper manifest) and “validation” (real-binary test matrix).
- Makes feature-gated surfaces explicit and reviewable.

### Tradeoffs / Risks
- The wrapper coverage manifest requires ongoing maintenance.
- Upstream help output is not a perfect source of truth; snapshots are “best available” and should be reconciled with real-binary behavior and fixtures.
- Some surfaces may be intentionally unsupported for safety/policy reasons and must be tracked as such to avoid perpetual churn.

## Follow-ups (out of scope for this ADR)
- JSONL/event-schema parity reporting (separate report stream; likely separate ADR).
- Automatic task creation from reports (allowed later, but human review remains required).
