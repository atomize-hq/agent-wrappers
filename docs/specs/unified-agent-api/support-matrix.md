# Support Matrix Spec — Unified Agent API

Status: Approved
Approved (UTC): 2026-04-15
Canonical location: `docs/specs/unified-agent-api/support-matrix.md`

This document is the authoritative contract for support publication semantics in the Unified Agent API spec set.

Normative language: this spec uses RFC 2119 requirement keywords (`MUST`, `MUST NOT`, `SHOULD`).

## Purpose

This spec defines the meaning of published support truth for the Unified Agent API documentation layer.
It separates support publication from capability advertising so the two concerns do not drift together.

## Support layers

Support publication MUST distinguish the following four layers:

- `manifest support`: what the committed CLI manifest evidence says about a target or version.
- `backend support`: what the agent-specific wrapper/backend crate can safely support based on its implementation and manifest inputs.
- `UAA unified support`: what the Unified Agent API can claim as a deterministic cross-agent support statement.
- `passthrough visibility`: backend-specific surface area that remains visible but is not promoted into unified support.

These layers MUST NOT be conflated with workflow status fields, pointer files, or generated overview artifacts.

For onboarding and publication purposes, backend support is crate-first:

- a new agent MAY have committed manifest evidence before any wrapper crate exists
- manifest evidence alone MUST NOT be promoted into backend support
- backend support MUST remain `unsupported` until crate-owned wrapper evidence exists for that target

## Publication targets

The phase-1 publication targets are:

- `cli_manifests/support_matrix/current.json`
- `docs/specs/unified-agent-api/support-matrix.md`

The JSON artifact is the machine-readable publication surface.
This Markdown document is the normative human-readable projection.

Both publication targets MUST describe the same support model.
If they disagree, the repository is in an invalid publication state.

## Support debt inventory

Published support truth for enrolled automated-maintenance agents MUST stay aligned with:

- `docs/specs/unified-agent-api/non-tui-support-debt.md`

That inventory is the machine-checkable baseline for temporary non-TUI blockers and preexisting
unsupported surface. Publication rules:

- support publication MUST NOT normalize deliberate non-TUI unsupported posture as a permanent
  steady-state note
- any remaining enrolled non-TUI caveat MUST point to a concrete debt-inventory row or a concrete
  external blocker that would also qualify for the maintenance-request deferral taxonomy
- `evidence_notes` MAY explain partial support, but MUST NOT become a shadow debt ledger

## Target-first primacy

Support truth MUST be target-scoped first.
Per-target rows are the primary publication unit.
Per-version summaries, if present elsewhere in the repository, MUST be treated as projections derived from those rows.

Support publication MUST preserve these distinctions:

- a target can be supported even when another target is not.
- a version summary MUST NOT collapse partial target support into a version-global claim.
- pointer state MAY inform publication, but pointer state alone is not support truth.

## Shared support row model

Both publication targets MUST be derived in a single pass from one shared support row model.
Each published row MUST describe exactly one `(agent, version, target)` tuple.
Publication MUST contain exactly one row for every `(agent, version, target)` tuple implied by the committed root set, each root's `current.json.expected_targets`, and each root's committed `versions/*.json` metadata.

The publication row set is exact:

- omitting an implied tuple is invalid publication state
- duplicating a tuple is invalid publication state
- publishing a tuple not implied by committed manifest metadata is invalid publication state

The canonical row field order is:

- `agent`
- `version`
- `target`
- `manifest_support`
- `backend_support`
- `uaa_support`
- `pointer_promotion`
- `evidence_notes`

These fields have the following meanings:

- `agent`: the manifest-root identifier whose committed evidence produced the row.
- `version`: the semantic version associated with the committed version metadata and reports.
- `target`: the root-native target identifier for the published row.
- `manifest_support`: the support state claimed by committed manifest evidence for that target.
- `backend_support`: the backend support state derived from implementation and manifest inputs for that target.
- `uaa_support`: the Unified Agent API support state published for that target after applying the support semantics in this spec.
- `pointer_promotion`: the pointer-derived promotion posture relevant to the row's target.
- `evidence_notes`: deterministic notes that explain intentional partial support or other non-contradictory caveats grounded in committed evidence.

For phase 1, `backend_support` is specifically the support posture proven by the agent-specific wrapper crate and the committed wrapper-derived reports under `cli_manifests/<agent>/reports/**`.
A manifest root without wrapper-derived report evidence MUST publish `backend_support` as `unsupported`.

The shared row model MUST preserve target-scoped truth even when multiple rows share the same version.
Publication code MUST NOT collapse multiple target rows into one version-global support claim.

The canonical row ordering MUST be deterministic:

- rows MUST sort by `agent` ascending
- within an agent, rows MUST sort by `target` ascending
- within an `(agent, target)` grouping, rows MUST sort by semantic `version` descending

`evidence_notes` rules are:

- notes MUST be derived from committed evidence and MUST NOT be speculative
- notes MAY explain intentional partial support, passthrough visibility, or other non-contradictory caveats visible in committed evidence
- notes MUST NOT redefine `validated` or `supported`, and MUST NOT stand in for contradiction policy
- for enrolled automated-maintenance agents, notes MUST NOT be the only place a non-TUI blocker is
  recorded; the blocker MUST also exist in `non-tui-support-debt.md`
- when no such caveat exists, publication MUST treat `evidence_notes` as empty rather than inventing explanatory prose

## Markdown projection boundary

`docs/specs/unified-agent-api/support-matrix.md` is the human-readable Markdown projection of the shared support row model.
It MUST render the same support truth as `cli_manifests/support_matrix/current.json`.

The Markdown projection MAY:

- group rows for readability
- add stable headings and table structure
- restate this spec's already-approved semantics

The Markdown projection MUST NOT:

- derive support truth independently from manifest roots
- introduce row fields or support states that are absent from the shared row model
- collapse target-scoped rows into version-global claims
- become a second mutable support ledger separate from the JSON artifact

## Authority rules

Published support rows MUST be derived from committed repository evidence.

For this spec set:

- manifest evidence is authoritative for manifest support.
- backend implementation evidence is authoritative for backend support.
- Unified Agent API publication text is authoritative for unified support semantics.
- passthrough visibility MUST remain explicit when a backend exposes behavior that is not part of unified support.

For phase 1, backend implementation evidence means wrapper-crate-owned evidence materialized into committed coverage reports.
Manifest metadata by itself MUST NOT be treated as backend support evidence.

The following MUST remain separate from published support truth:

- `validated` and `supported` status fields in version metadata
- generated capability inventory
- runtime backend capability checks

## Neutral root intake

The support-matrix pipeline MUST consume committed evidence from each agent root through one neutral root-intake contract.

For phase 1, that intake contract MUST be limited to these evidence categories under `cli_manifests/<agent>/`:

- `versions/*.json` version metadata
- `pointers/latest_supported/*.txt` and `pointers/latest_validated/*.txt`
- `current.json`
- `reports/**`

This intake contract MUST remain shape-driven rather than agent-name-driven:

- the pipeline MUST reason about root-local evidence categories and paths, not special-case Codex or Claude by name inside shared intake logic.
- the contract MAY preserve root-native target identifiers as loaded evidence; later derivation decides how publication rows compare or project them.
- the contract MUST NOT introduce a second support evidence store outside the committed manifest roots.

This intake contract governs evidence loading only. It MUST NOT change publication targets, support-layer meanings, or the distinction between `validated` and `supported`.

## Validated versus supported

`validated` and `supported` are distinct workflow states.

- `validated` means a version passed the validation matrix and is promotion-grade for the version pointer flow.
- `supported` means wrapper coverage satisfies the stronger support policy for the version and target surface.

A version MAY be manifest-supported before it is backend-supported.
A version MAY be backend-supported before it is promoted into UAA unified support.

The repository MUST NOT treat `validated` as equivalent to `supported`.
The repository MUST NOT treat workflow status as a published support row.
The repository MUST NOT use workflow status as a substitute for target-scoped support evidence.

## Separation from the capability matrix

The support matrix MUST remain separate from `docs/specs/unified-agent-api/capability-matrix.md`.

- The capability matrix documents backend capability advertising.
- The support matrix documents published support truth.
- The two artifacts MAY share source evidence, but they MUST NOT share meaning.
- A change to one artifact MUST NOT be assumed to update the other.

If a reader needs backend capability coverage, they SHOULD use the capability matrix.
If a reader needs published support truth, they MUST use the support matrix.

## Verification checklist

Before downstream work consumes this contract, reviewers MUST confirm:

- the canonical support publication targets are named exactly once and without ambiguity.
- the four support layers have distinct meanings and no overlap with workflow metadata.
- target-scoped rows are described as primary and per-version summaries as derived projections.
- the neutral root-intake contract is limited to committed root evidence and does not introduce agent-name-specific loading semantics.
- backend support is explicitly crate-first and cannot be claimed from manifest metadata alone.
- `validated` is not treated as equivalent to `supported`.
- the support matrix is explicitly separate from the capability matrix.
- the spec is sufficient for downstream implementation without reopening authority or output-path decisions.

## Change control

Any future update to support publication semantics MUST update this spec first.
If the publication model changes, the README index and any dependent publication docs MUST be updated in the same change.

## Published support matrix

This section is generated by `cargo run -p xtask -- support-matrix`.
Do not edit this section by hand.

<!-- support-matrix-published:start -->
### `aider`

| agent | version | target | manifest_support | backend_support | uaa_support | pointer_promotion | evidence_notes |
|---|---|---|---|---|---|---|---|
| `aider` | `0.0.0` | `darwin-arm64` | `supported` | `unsupported` | `unsupported` | `latest_supported_and_validated` | — |

### `claude_code`

| agent | version | target | manifest_support | backend_support | uaa_support | pointer_promotion | evidence_notes |
|---|---|---|---|---|---|---|---|
| `claude_code` | `2.1.236` | `darwin-arm64` | `unsupported` | `unsupported` | `unsupported` | `none` | backend report includes backend-only surface outside unified support |
| `claude_code` | `2.1.29` | `darwin-arm64` | `supported` | `supported` | `supported` | `latest_supported_and_validated` | — |
| `claude_code` | `2.1.236` | `linux-x64` | `unsupported` | `unsupported` | `unsupported` | `none` | backend report includes backend-only surface outside unified support |
| `claude_code` | `2.1.29` | `linux-x64` | `supported` | `supported` | `supported` | `latest_supported_and_validated` | — |
| `claude_code` | `2.1.236` | `win32-x64` | `unsupported` | `unsupported` | `unsupported` | `none` | backend report includes intentionally unsupported surface outside unified support; backend report includes backend-only surface outside unified support |
| `claude_code` | `2.1.29` | `win32-x64` | `supported` | `partial` | `partial` | `latest_supported_and_validated` | backend report includes intentionally unsupported surface outside unified support |

### `codex`

| agent | version | target | manifest_support | backend_support | uaa_support | pointer_promotion | evidence_notes |
|---|---|---|---|---|---|---|---|
| `codex` | `0.144.6` | `aarch64-apple-darwin` | `supported` | `partial` | `partial` | `none` | backend report includes intentionally unsupported surface outside unified support; backend report includes passthrough surface outside unified support; backend report includes backend-only surface outside unified support |
| `codex` | `0.129.0` | `aarch64-apple-darwin` | `unsupported` | `unsupported` | `unsupported` | `none` | — |
| `codex` | `0.125.0` | `aarch64-apple-darwin` | `supported` | `partial` | `partial` | `latest_supported_and_validated` | backend report includes intentionally unsupported surface outside unified support; backend report includes backend-only surface outside unified support |
| `codex` | `0.97.0` | `aarch64-apple-darwin` | `unsupported` | `unsupported` | `unsupported` | `none` | — |
| `codex` | `0.92.0` | `aarch64-apple-darwin` | `unsupported` | `unsupported` | `unsupported` | `none` | — |
| `codex` | `0.91.0` | `aarch64-apple-darwin` | `unsupported` | `unsupported` | `unsupported` | `none` | — |
| `codex` | `0.61.0` | `aarch64-apple-darwin` | `unsupported` | `unsupported` | `unsupported` | `none` | — |
| `codex` | `0.144.6` | `aarch64-unknown-linux-musl` | `supported` | `partial` | `partial` | `none` | backend report includes intentionally unsupported surface outside unified support; backend report includes passthrough surface outside unified support; backend report includes backend-only surface outside unified support |
| `codex` | `0.129.0` | `aarch64-unknown-linux-musl` | `unsupported` | `unsupported` | `unsupported` | `none` | — |
| `codex` | `0.125.0` | `aarch64-unknown-linux-musl` | `supported` | `partial` | `partial` | `latest_supported_and_validated` | backend report includes intentionally unsupported surface outside unified support; backend report includes backend-only surface outside unified support |
| `codex` | `0.97.0` | `aarch64-unknown-linux-musl` | `unsupported` | `unsupported` | `unsupported` | `none` | — |
| `codex` | `0.92.0` | `aarch64-unknown-linux-musl` | `unsupported` | `unsupported` | `unsupported` | `none` | — |
| `codex` | `0.91.0` | `aarch64-unknown-linux-musl` | `unsupported` | `unsupported` | `unsupported` | `none` | — |
| `codex` | `0.61.0` | `aarch64-unknown-linux-musl` | `unsupported` | `unsupported` | `unsupported` | `none` | — |
| `codex` | `0.144.6` | `x86_64-pc-windows-msvc` | `supported` | `partial` | `partial` | `none` | backend report includes intentionally unsupported surface outside unified support; backend report includes passthrough surface outside unified support; backend report includes backend-only surface outside unified support |
| `codex` | `0.129.0` | `x86_64-pc-windows-msvc` | `unsupported` | `unsupported` | `unsupported` | `none` | — |
| `codex` | `0.125.0` | `x86_64-pc-windows-msvc` | `supported` | `partial` | `partial` | `latest_supported_and_validated` | backend report includes intentionally unsupported surface outside unified support; backend report includes backend-only surface outside unified support |
| `codex` | `0.97.0` | `x86_64-pc-windows-msvc` | `unsupported` | `unsupported` | `unsupported` | `none` | — |
| `codex` | `0.92.0` | `x86_64-pc-windows-msvc` | `unsupported` | `unsupported` | `unsupported` | `none` | — |
| `codex` | `0.91.0` | `x86_64-pc-windows-msvc` | `unsupported` | `unsupported` | `unsupported` | `none` | — |
| `codex` | `0.61.0` | `x86_64-pc-windows-msvc` | `unsupported` | `unsupported` | `unsupported` | `none` | — |
| `codex` | `0.144.6` | `x86_64-unknown-linux-musl` | `supported` | `partial` | `partial` | `none` | backend report includes intentionally unsupported surface outside unified support; backend report includes passthrough surface outside unified support; backend report includes backend-only surface outside unified support |
| `codex` | `0.129.0` | `x86_64-unknown-linux-musl` | `supported` | `partial` | `partial` | `none` | backend report includes intentionally unsupported surface outside unified support; backend report includes backend-only surface outside unified support |
| `codex` | `0.125.0` | `x86_64-unknown-linux-musl` | `supported` | `partial` | `partial` | `latest_supported_and_validated` | backend report includes intentionally unsupported surface outside unified support; backend report includes backend-only surface outside unified support |
| `codex` | `0.97.0` | `x86_64-unknown-linux-musl` | `supported` | `partial` | `partial` | `none` | backend report includes intentionally unsupported surface outside unified support; backend report includes backend-only surface outside unified support |
| `codex` | `0.92.0` | `x86_64-unknown-linux-musl` | `supported` | `partial` | `partial` | `none` | backend report includes intentionally unsupported surface outside unified support; backend report includes backend-only surface outside unified support |
| `codex` | `0.91.0` | `x86_64-unknown-linux-musl` | `unsupported` | `unsupported` | `unsupported` | `none` | backend report includes intentionally unsupported surface outside unified support; backend report includes backend-only surface outside unified support |
| `codex` | `0.61.0` | `x86_64-unknown-linux-musl` | `supported` | `partial` | `partial` | `none` | backend report includes intentionally unsupported surface outside unified support; backend report includes backend-only surface outside unified support |

### `gemini_cli`

| agent | version | target | manifest_support | backend_support | uaa_support | pointer_promotion | evidence_notes |
|---|---|---|---|---|---|---|---|
| `gemini_cli` | `0.38.2` | `darwin-arm64` | `unsupported` | `unsupported` | `unsupported` | `none` | — |

### `opencode`

| agent | version | target | manifest_support | backend_support | uaa_support | pointer_promotion | evidence_notes |
|---|---|---|---|---|---|---|---|
| `opencode` | `1.14.47` | `darwin-arm64` | `unsupported` | `unsupported` | `unsupported` | `none` | — |
| `opencode` | `1.4.11` | `darwin-arm64` | `unsupported` | `unsupported` | `unsupported` | `none` | current root snapshot omits this target |
| `opencode` | `1.4.9` | `darwin-arm64` | `unsupported` | `unsupported` | `unsupported` | `none` | — |
| `opencode` | `1.14.47` | `darwin-x64` | `unsupported` | `unsupported` | `unsupported` | `none` | — |
| `opencode` | `1.4.11` | `darwin-x64` | `unsupported` | `unsupported` | `unsupported` | `none` | current root snapshot omits this target |
| `opencode` | `1.4.9` | `darwin-x64` | `unsupported` | `unsupported` | `unsupported` | `none` | — |
| `opencode` | `1.14.47` | `linux-arm64` | `unsupported` | `unsupported` | `unsupported` | `none` | — |
| `opencode` | `1.4.11` | `linux-arm64` | `unsupported` | `unsupported` | `unsupported` | `none` | current root snapshot omits this target |
| `opencode` | `1.4.9` | `linux-arm64` | `unsupported` | `unsupported` | `unsupported` | `none` | — |
| `opencode` | `1.14.47` | `linux-x64` | `unsupported` | `unsupported` | `unsupported` | `none` | — |
| `opencode` | `1.4.11` | `linux-x64` | `supported` | `unsupported` | `unsupported` | `latest_validated` | — |
| `opencode` | `1.4.9` | `linux-x64` | `unsupported` | `unsupported` | `unsupported` | `none` | — |
| `opencode` | `1.14.47` | `win32-arm64` | `unsupported` | `unsupported` | `unsupported` | `none` | — |
| `opencode` | `1.4.11` | `win32-arm64` | `unsupported` | `unsupported` | `unsupported` | `none` | current root snapshot omits this target |
| `opencode` | `1.4.9` | `win32-arm64` | `unsupported` | `unsupported` | `unsupported` | `none` | — |
| `opencode` | `1.14.47` | `win32-x64` | `unsupported` | `unsupported` | `unsupported` | `none` | — |
| `opencode` | `1.4.11` | `win32-x64` | `unsupported` | `unsupported` | `unsupported` | `none` | current root snapshot omits this target |
| `opencode` | `1.4.9` | `win32-x64` | `unsupported` | `unsupported` | `unsupported` | `none` | — |
<!-- support-matrix-published:end -->
