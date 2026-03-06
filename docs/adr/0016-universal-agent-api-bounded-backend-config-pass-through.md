# ADR-0016 — Universal Agent API bounded backend config pass-through (`agent_api.config.*` + `backend.*`)
#
# Note: Run `make adr-fix ADR=docs/adr/0016-universal-agent-api-bounded-backend-config-pass-through.md`
# after editing to update the ADR_BODY_SHA256 drift guard.

## Status
- Status: Draft
- Date (UTC): 2026-02-27
- Owner(s): spensermcconnell

## Scope

This ADR defines how the Universal Agent API should expose *configuration/flag surfaces* from
multiple CLI agent backends, specifically:

- what becomes universal (core) extension keys under `agent_api.config.*` (and related universal
  buckets like `agent_api.exec.*`), vs
- what remains backend-specific and is exposed only via **bounded** backend extension keys under
  `backend.<agent_kind>.*`.

This is the “final sweep before implementation” for backlog work item `uaa-0012`.

## Related Docs

- Core ownership + gating rules:
  - `docs/specs/universal-agent-api/extensions-spec.md`
  - `docs/specs/universal-agent-api/capabilities-schema-spec.md`
- Inventory inputs:
  - `cli_manifests/codex/current.json`
  - `cli_manifests/claude_code/current.json`
  - `cli_manifests/codex/wrapper_coverage.json`
  - `cli_manifests/claude_code/wrapper_coverage.json`
- Relevant wrapper surfaces:
  - `crates/codex/src/builder/cli_overrides.rs`
  - `crates/codex/src/builder/types.rs`
  - `crates/claude_code/src/commands/print.rs`
- Backlog tasks:
  - `docs/backlog.json` (`uaa-0012`, `uaa-0002`, `uaa-0003`, `uaa-0016`)

## Executive Summary (Operator)

ADR_BODY_SHA256: 7624ac7330a8195a7e5e228d7f5f3c351a7d4145741ba3a44d9377ee10fc5c97

### Decision (draft)

- Promote only **backend-neutral, stable semantics** into core extension keys under `agent_api.*`.
  - Concrete near-term universals: model selection + extra context roots.
- When we need to express “the host provides isolation externally”, use an explicit, dangerous core
  key (`agent_api.exec.external_sandbox.v1`) (see
  `docs/adr/0019-universal-agent-api-external-sandbox-exec-policy.md`) rather than ad-hoc backend
  keys or implicit behavior.
- Everything else remains backend-specific, but may still be surfaced via **bounded pass-through**
  keys under `backend.<agent_kind>.*`.
- “Bounded pass-through” means: versioned keys, closed schemas, strict bounds, deterministic
  validation before spawn, and explicit “dangerous” knobs (never implied by `non_interactive`).

### Proposed key list (draft)

**Universal (`agent_api.*`)**

- `agent_api.exec.non_interactive` (boolean, already Approved)
- `agent_api.exec.external_sandbox.v1` (boolean; explicit dangerous mode for externally sandboxed hosts;
  see `docs/adr/0019-universal-agent-api-external-sandbox-exec-policy.md`)
- `agent_api.config.model.v1` (string; backlog `uaa-0002`)
- `agent_api.exec.add_dirs.v1` (object; backlog `uaa-0003`)

**Backend-scoped (`backend.*`)**

- Codex (existing):
  - `backend.codex.exec.approval_policy` (string enum; already shipped)
  - `backend.codex.exec.sandbox_mode` (string enum; already shipped)
- Codex (new, proposed):
  - `backend.codex.exec.cli_overrides.v1` (object; bounded subset of Codex config/feature knobs)
- Claude Code (new, proposed):
  - `backend.claude_code.settings.v1` (object; bounded settings file inputs)
  - `backend.claude_code.print.overrides.v1` (object; bounded print-mode knobs)

## Problem / Context

We have two primary CLI backends with large, partially overlapping sets of flags/config surfaces:

- **Codex**: has a generic `--config key=value` and generic `--enable/--disable <feature>` toggles.
  The wrapper currently forwards these as strings (coverage level: `passthrough`), which means the
  Universal Agent API cannot validate the effective configuration (keys are an open set).
- **Claude Code**: has many explicit flags for settings, tool policy, MCP config, permissions, etc.
  While these are “explicit” flags, naive exposure as raw CLI pass-through would still create an
  unbounded surface (`--plugin-dir`, free-form lists, settings file paths, etc.).

If we promote too many backend-specific knobs into `agent_api.*`, we bake in semantics that are not
actually universal and we make future backend onboarding harder.

If we allow arbitrary CLI arg pass-through, we lose the core Universal Agent API properties:

- deterministic validation (“fail before spawn”),
- capability-gated optionality, and
- bounded/inspectable request shape.

## Goals

- Provide a concrete rubric for promoting a knob to `agent_api.*` vs keeping it backend-scoped.
- Replace “raw string pass-through” with bounded, versioned backend keys when backend-specific knobs
  are still required.
- Keep safe defaults for general Universal Agent API consumers, while allowing internally sandboxed
  hosts (e.g. Substrate) to opt into explicitly dangerous exec policy via
  `agent_api.exec.external_sandbox.v1`.

## Non-Goals

- Implementing these keys in code in this ADR (this ADR is the plan + decision record).
- Exposing a generic “extra args” array for any backend.
- Universalizing tool allow/deny policy across all agents (candidate future core keys, but not
  promoted in this pass).
- Supporting Claude Code `--plugin-dir` via the Universal Agent API (intentionally unbounded and
  difficult to validate safely).
- Making `agent_api.exec.non_interactive` imply “full bypass” behavior. Dangerous behavior MUST be
  explicitly requested via a dedicated dangerous key (`agent_api.exec.external_sandbox.v1`) and MUST
  be capability-gated.

## Inventory (from manifests)

This inventory is derived from:
- `cli_manifests/*/current.json` (enumeration of flags by command), and
- `cli_manifests/*/wrapper_coverage.json` (what our wrappers already expose vs pass-through).

### Codex

Root-level flags include: `--model`, `--add-dir`, `--ask-for-approval`, `--sandbox`, `--profile`,
`--local-provider`, `--oss`, `--search`, plus safety overrides like `--full-auto` and
`--dangerously-bypass-approvals-and-sandbox`, and:

- `--config` (coverage level: `passthrough`)
- `--enable` / `--disable` (coverage level: `passthrough`)

These three “generic” flags are the primary motivation for a bounded config surface.

### Claude Code

Root-level flags include: `--model`, `--add-dir`, `--settings`, `--setting-sources`, `--mcp-config`,
`--strict-mcp-config`, `--permission-mode`, `--dangerously-skip-permissions`, tool allow/deny flags,
and others.

Wrapper coverage is largely “explicit” (no generic passthrough flag), but several knobs still
represent open-ended or path-based surfaces that should only be exposed with strict bounds.

## Promotion rubric: universal vs backend-scoped

Promote a knob to a core `agent_api.*` extension key **only** if all are true:

1) **Cross-backend semantic match**: it exists (or can be implemented) in both Codex and Claude Code
   with the same user meaning.
2) **Stable & versionable**: we can define a small, closed schema that is unlikely to churn with CLI
   releases.
3) **Safe defaults**: the default (when absent) is safe and non-surprising for automation.
4) **Deterministic validation**: invalid values can be rejected *before spawn*.

If any condition fails, keep the knob backend-specific under `backend.<agent_kind>.*`.

## Bounded backend pass-through (definition)

A bounded backend pass-through key:

- is always **versioned** (`.v1`) when it carries an object payload,
- has a **closed schema** (unknown keys are invalid),
- enforces **strict bounds** (list lengths, string lengths, numeric ranges),
- validates **paths** deterministically (normalization + containment rules), and
- never smuggles “dangerous” behavior behind a benign universal key.

## Proposed key specs (draft)

This ADR proposes the following keys. Core keys must be defined in
`docs/specs/universal-agent-api/extensions-spec.md` before shipping; backend keys must be defined in
backend-owned docs (per ownership rule R1).

### Core: `agent_api.config.model.v1` (string)

Schema:
- Type: string
- Bounds:
  - trimmed, non-empty
  - length: 1..=128 bytes (UTF-8)

Meaning:
- Select a model identifier in a backend-neutral way.

Backend mapping:
- Codex: `--model <id>`
- Claude Code: `--model <id>`

Notes:
- Track in `docs/backlog.json` as `uaa-0002`.

### Core: `agent_api.exec.add_dirs.v1` (object)

Schema (closed):
- Type: object
- Keys:
  - `dirs` (array of string, required)
- Bounds:
  - `dirs`: 0..=16 entries
  - each entry: trimmed, non-empty, length 1..=1024 bytes

Path validation (proposed):
- `effective working directory` is defined in `docs/specs/universal-agent-api/contract.md` ("Working directory resolution (effective working directory)").
- Each entry MAY be absolute or relative.
- If relative, resolve against the run’s effective working directory.
- Backends SHOULD reject paths that normalize outside the effective working directory (containment
  check), unless the backend explicitly documents an exception.

Backend mapping:
- Codex: repeat `--add-dir <dir>` per entry
- Claude Code: repeat `--add-dir <dir>` per entry

Notes:
- Track in `docs/backlog.json` as `uaa-0003`.

### Backend (Codex): `backend.codex.exec.cli_overrides.v1` (object)

Intent: replace open-ended `--config`/`--enable/--disable` pass-through with a typed subset of
stable knobs we actually want to support.

Schema (closed):
- Type: object
- Optional keys:
  - `profile` (string; 1..=64)
  - `cd` (string path; 1..=1024)
  - `local_provider` (string enum): `lmstudio | ollama | custom`
  - `oss` (boolean)
  - `search` (boolean)
  - `reasoning` (object; closed):
    - `effort` (string enum): `minimal | low | medium | high`
    - `summary` (string enum): `auto | concise | detailed | none`
    - `verbosity` (string enum): `low | medium | high`
    - `summary_format` (string enum): `none | experimental`
    - `supports_summaries` (boolean)

Defaults:
- When absent: no overrides.
- When present: absent fields are treated as “no override” (inherit backend defaults).

Validation rules (proposed):
- Unknown keys invalid.
- `cd` must resolve to a directory path within the effective working directory (containment).
- This key MUST NOT provide an unbounded escape hatch for arbitrary `--config` keys or arbitrary
  feature toggles.

Backend mapping:
- Implement via `crates/codex/src/builder/types.rs` + `crates/codex/src/builder/cli_overrides.rs`
  (map typed fields into the appropriate Codex flags/config keys).

### Backend (Claude Code): `backend.claude_code.settings.v1` (object)

Schema (closed):
- Type: object
- Required keys:
  - `path` (string path; 1..=1024)
- Optional keys:
  - `sources` (string; 1..=256) — wrapper-level passthrough of `--setting-sources`

Defaults:
- When absent: do not pass `--settings` / `--setting-sources`.

Validation rules (proposed):
- Unknown keys invalid.
- `path` must resolve within the effective working directory (containment).

Backend mapping:
- `--settings <path>`
- when `sources` present: `--setting-sources <sources>`

### Backend (Claude Code): `backend.claude_code.print.overrides.v1` (object)

Schema (closed; intentionally small to start):
- Type: object
- Optional keys:
  - `allowed_tools` (array of string; 0..=64; each 1..=128)
  - `disallowed_tools` (array of string; 0..=64; each 1..=128)
  - `mcp_config_path` (string path; 1..=1024)
  - `strict_mcp_config` (boolean)
  - `max_budget_usd` (number; 0..=100)
  - `system_prompt` (string; 0..=8192)
  - `append_system_prompt` (string; 0..=8192)

Defaults:
- When absent: do not override these knobs.

Validation rules (proposed):
- Unknown keys invalid.
- `mcp_config_path` must resolve within the effective working directory (containment).
- `allowed_tools` and `disallowed_tools` MUST NOT both be non-empty unless explicitly documented as
  compatible by the backend (prefer fail-closed).

Backend mapping:
- Map to `crates/claude_code/src/commands/print.rs` fields on `ClaudePrintRequest`.

## Consequences

Pros:
- Preserves the Universal Agent API “fail-closed, capability-gated” model while still allowing
  backend-specific knobs where required.
- Avoids creating an implied, unbounded CLI-arg passthrough surface.
- Keeps core `agent_api.config.*` minimal and stable.

Cons:
- Requires ongoing curation: adding a new backend knob means adding a schema field (or a new
  versioned backend key), not “just forwarding args”.
- Some workflows that rely on arbitrary `--config` or plugin mechanisms will require explicit new
  schema work (intentional friction).

## Follow-on Work (out of scope for this ADR)

- Define + implement:
  - `agent_api.config.model.v1` (uaa-0002)
  - `agent_api.exec.add_dirs.v1` (uaa-0003)
  - `agent_api.exec.external_sandbox.v1` (uaa-0016; see
    `docs/adr/0019-universal-agent-api-external-sandbox-exec-policy.md`; needs owner-doc spec entry)
- Define backend-owned docs for the new backend keys and implement them in `agent_api`:
  - `backend.codex.exec.cli_overrides.v1`
  - `backend.claude_code.settings.v1`
  - `backend.claude_code.print.overrides.v1`
