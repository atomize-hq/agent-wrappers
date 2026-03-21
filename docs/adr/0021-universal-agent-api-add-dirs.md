# ADR-0021 — Universal extra context roots (`agent_api.exec.add_dirs.v1`)
#
# Note: Run `make adr-fix ADR=docs/adr/0021-universal-agent-api-add-dirs.md`
# after editing to update the ADR_BODY_SHA256 drift guard.

## Status
- Status: Draft (implementation plan; normative semantics are pinned in the Universal Agent API specs)
- Date (UTC): 2026-03-12
- Owner(s): spensermcconnell

## Scope

- Define a single core extension key for extra context directories:
  - `agent_api.exec.add_dirs.v1`
- Pin:
  - schema and bounds,
  - absolute vs relative path semantics,
  - normalization/validation behavior before spawn, and
  - backend mapping for Codex and Claude Code.

This ADR corresponds to backlog item `uaa-0003` (`bucket=agent_api.exec`, `type=extension_key`).

## Related Docs

- Backlog:
  - `docs/backlog.json` (`uaa-0003`)
- Prior bounded pass-through decision:
  - `docs/adr/0016-universal-agent-api-bounded-backend-config-pass-through.md`
- Universal Agent API baselines:
  - `docs/specs/universal-agent-api/contract.md`
  - `docs/specs/universal-agent-api/run-protocol-spec.md`
  - `docs/specs/universal-agent-api/capabilities-schema-spec.md` (Standard capability ids:
    `agent_api.exec.add_dirs.v1`)
  - `docs/specs/universal-agent-api/extensions-spec.md` (owner doc for the core key)
- Backend mapping contracts:
  - `docs/specs/codex-streaming-exec-contract.md` (Codex exec/resume mapping)
  - `docs/specs/codex-app-server-jsonrpc-contract.md` (Codex fork rejection boundary)
  - `docs/specs/claude-code-session-mapping-contract.md`
- Backend mapping seams:
  - `crates/codex/src/builder/mod.rs`
  - `crates/codex/src/capabilities/guard.rs`
  - `crates/claude_code/src/commands/print.rs`

## Executive Summary (Operator)

ADR_BODY_SHA256: 0e76730d0a6ea502ce41c803540e2578c7bb82959fc08113f33552f715ac7c26

### Decision (draft)

- Introduce a new core extension key:
  - `agent_api.exec.add_dirs.v1`
- Capability advertisement follows the standard capability id entry in
  `docs/specs/universal-agent-api/capabilities-schema-spec.md`.
- Schema (closed) (illustrative shape; not fixed-length):

```json
{
  "dirs": ["string", "string"]
}
```

- Note: the snippet above shows an array-of-strings shape only; `dirs` length is `1..=16` (not
  fixed-length).
- Bounds:
  - `dirs` is required
  - `dirs` length: `1..=16`
  - each entry is trimmed before validation/mapping
  - each trimmed entry: non-empty, UTF-8 length `1..=1024` bytes
- Default when absent:
  - no extra context directories are requested
  - the backend MUST NOT emit `--add-dir`
- Path semantics:
  - entries MAY be absolute or relative
  - relative entries resolve against the run’s effective working directory (defined in
    `docs/specs/universal-agent-api/contract.md` "Working directory resolution (effective working directory)")
  - there is no containment requirement that keeps paths under the effective working directory
  - after resolution, each path MUST exist and MUST be a directory before spawn
- Normalization:
  - backends MUST trim each entry, resolve relative paths before spawn, and lexically normalize the
    resulting paths
  - normalization is lexical only for v1; shell-style `~`/env expansion, filesystem
    canonicalization, and symlink resolution are not required
  - de-duplicate normalized resolved paths while preserving first occurrence order
  - the normalized unique paths are what get mapped into backend argv
- Backend mapping:
  - Codex: repeat `--add-dir <DIR>`
  - Claude Code: one variadic `--add-dir <DIR...>` group in normalized order
- Session compatibility:
  - the key is valid for new-session, resume, and fork flows
  - session selection remains owned by `AgentWrapperRunRequest.extensions`; this key does not add a
    separate request field or alternate selector surface
  - a selected session flow MUST either apply the accepted normalized directory set unchanged or
    take a pinned safe backend-rejection path; it MUST NOT silently ignore accepted add-dir inputs
  - malformed or otherwise invalid add-dir payloads still fail as `AgentWrapperError::InvalidRequest`;
    the Codex fork rejection applies only to accepted inputs

### Why

- Both built-in backends already expose the same root-level flag for adding extra directories.
- The purpose of the feature is explicitly to widen backend-visible context roots, so restricting
  paths to the working directory would defeat many legitimate use cases.
- A bounded, explicit list preserves deterministic validation and avoids backend-specific drift.

## Problem / Context

Both supported CLI agents expose `--add-dir`, and both wrapper crates already model it:

- Codex: `CodexClientBuilder::add_dir(...)`
- Claude Code: `ClaudePrintRequest::add_dirs(...)`

This feature is needed to give a caller one cross-agent way to expand the set of directories the
backend may use for file reads, indexing, or tool access beyond the primary working directory.

ADR-0016 is historical context for why this feature needed a dedicated follow-up, but path
semantics are no longer open for `agent_api.exec.add_dirs.v1` in this ADR. The resolved v1 rules
are:

- paths MAY be absolute or relative,
- relative paths resolve against the run's effective working directory, and
- v1 intentionally does not enforce containment to the effective working directory.

No path-semantics follow-up remains open inside this feature. Any future change to those rules would
require a new ADR/spec revision rather than reinterpretation of ADR-0016.

Containment is the wrong default here. The point of “extra context roots” is to allow intentional
access to additional directories, including sibling repos, shared docs trees, or checked-out assets
that sit outside the primary working directory.

At the same time, the feature still needs deterministic validation so it does not become an
unbounded or ambiguous path-pass-through surface.

## Goals

- Provide one bounded, capability-gated universal key for extra context directories.
- Make path resolution deterministic across backends.
- Support both absolute and relative inputs without making callers pre-normalize everything.
- Preserve the feature’s real utility by allowing directories outside the working directory.

## Non-Goals

- Defining a sandbox or security policy for what the host should allow.
- Inventing a backend-neutral meaning stronger than “additional roots for backend context/file
  access”; individual CLIs may still differ in exact internal indexing behavior.
- Supporting files in v1; the key is directories only.
- Exposing an unbounded raw path array at `backend.<agent_kind>.*`.

## Proposed Design (Draft)

### Core extension key

`agent_api.exec.add_dirs.v1`

Owner:
- `docs/specs/universal-agent-api/extensions-spec.md`

Schema (closed):
- Type: object
- Required keys:
  - `dirs` (array of string)
- Unknown keys:
  - invalid in v1

Bounds:
- `dirs` length MUST be `>= 1` and `<= 16`
- the backend MUST trim leading/trailing Unicode whitespace before validation and mapping
- after trimming, each entry MUST be non-empty
- after trimming, each entry MUST be `<= 1024` bytes (UTF-8)

Absence semantics:
- When absent, no extra directories are requested.
- The backend MUST NOT synthesize any additional directories.
- The backend MUST NOT emit `--add-dir` or any equivalent backend-specific override on behalf of
  this key.

### Path resolution semantics

For each `dirs[i]` entry:

1. Trim leading/trailing Unicode whitespace. The trimmed value is the effective entry.
2. If the value is relative, resolve it against the run’s effective working directory.
3. If the value is absolute, keep it absolute.
4. Lexically normalize the resolved path using platform path rules sufficient to fold redundant
   separators and `.` / `..` segments.
5. Do not apply shell-style `~` expansion or environment-variable expansion.
6. Do not require filesystem canonicalization or symlink resolution in v1.
7. Validate that the resolved path exists and is a directory before spawn.

Notes:
- There is intentionally no “must stay under working_dir” rule.
- Callers that need predictable relative resolution SHOULD set `AgentWrapperRunRequest.working_dir`,
  because the effective working directory may otherwise be backend-defaulted.

### Duplicate handling

- After resolution/normalization, duplicate directories MUST be removed while preserving first
  occurrence order.
- De-duplication is not treated as an error because repeated roots do not carry additional meaning.

### Mapping into backend argv

The backend MUST pass the normalized unique directory list, in order, to its underlying
CLI/backend mapping.

#### Codex

- CLI form: `codex exec --add-dir <DIR> ...`
- Implementation seams:
  - `crates/codex/src/builder/mod.rs` (`CodexClientBuilder::{add_dir,...}`)
  - `crates/codex/src/capabilities/guard.rs` (`guard_add_dir`)
- Exec/resume argv contract:
  - `docs/specs/codex-streaming-exec-contract.md`
  - pinned placement: repeated `--add-dir <DIR>` pairs stay after any accepted `--model` pair
- Fork-flow contract:
  - `docs/specs/codex-app-server-jsonrpc-contract.md`
  - current pinned truth: Codex fork has no add-dir transport field on `thread/fork` or
    `turn/start`, so accepted add-dir inputs are rejected before any app-server request with
    `AgentWrapperError::Backend { message: "add_dirs unsupported for codex fork" }`

#### Claude Code

- CLI form: `claude --print --add-dir <DIR...>`
- Implementation seam:
  - `crates/claude_code/src/commands/print.rs` (`ClaudePrintRequest::add_dirs(...)`)
- Resume/fork argv contract:
  - `docs/specs/claude-code-session-mapping-contract.md`
  - pinned placement: one variadic add-dir group after any accepted `--model` pair and before
    `--continue` / `--fork-session` / `--resume`, the final `--verbose` token, and the final
    prompt token

### Capability advertising

- A backend MUST advertise `agent_api.exec.add_dirs.v1` only when it has a deterministic contract
  for every run surface it exposes for this key: either a pinned mapping that honors the accepted
  directory list or a pinned backend-owned safe rejection path.
- For the current built-in backends, advertising is expected to be unconditional once
  implementation lands.
- Capability advertising is about support for the request surface, not per-run path contents.

### Validation and failure model

Before spawn:
- If the capability id is unsupported, fail per R0 with `AgentWrapperError::UnsupportedCapability`.
- The value MUST be an object; otherwise fail with `AgentWrapperError::InvalidRequest`.
- Unknown object keys MUST fail with `AgentWrapperError::InvalidRequest` (closed schema for `.v1`).
- `dirs` MUST be present and MUST be an array; otherwise fail with
  `AgentWrapperError::InvalidRequest`.
- `dirs` MUST contain at least 1 and at most 16 entries (`1..=16`); otherwise fail with
  `AgentWrapperError::InvalidRequest`.
- Each `dirs[i]` entry MUST be a string; otherwise fail with `AgentWrapperError::InvalidRequest`.
- After trimming, each `dirs[i]` entry MUST be non-empty; otherwise fail with
  `AgentWrapperError::InvalidRequest`.
- After trimming, each `dirs[i]` entry MUST be `<= 1024` UTF-8 bytes; otherwise fail with
  `AgentWrapperError::InvalidRequest`.
- After resolution and lexical normalization, each effective path MUST exist and MUST be a
  directory before spawn; otherwise fail with `AgentWrapperError::InvalidRequest`.
- InvalidRequest messages for this key MUST be safe, MUST NOT echo raw path values, and MUST use
  one of these exact templates:
  - `invalid agent_api.exec.add_dirs.v1`
  - `invalid agent_api.exec.add_dirs.v1.dirs`
  - `invalid agent_api.exec.add_dirs.v1.dirs[<i>]`
- `<i>` is the zero-based decimal index of the failing `dirs[i]` entry.
- Backends MUST NOT invent any other InvalidRequest message shape for this key.

After spawn:
- If the key passed R0 capability gating and pre-spawn validation, but the backend later determines
  that the requested directories cannot be honored by the installed CLI/runtime/selected flow, the
  backend MUST fail as `AgentWrapperError::Backend` with a safe/redacted message.
- This includes resume/fork flows that cannot deterministically apply the accepted add-dir set.
- If the backend can determine that inability before spawning its backend surface, it MUST return
  the backend error without returning an `AgentWrapperRunHandle`.
- If this occurs after an event stream has been returned and the stream is still open, the backend
  MUST emit exactly one terminal `AgentWrapperEventKind::Error` event with the same safe/redacted
  message before closing the stream.

## Alternatives Considered

1. Relative paths only
   - Rejected: forces callers to rebase external roots through the working directory and makes the
     API awkward for common multi-repo or shared-assets workflows.

2. Enforce containment under the effective working directory
   - Rejected: contradicts the feature’s purpose, which is to add roots beyond that directory.

3. Raw array value instead of an object
   - Rejected: an object with `dirs` gives the spec room for additive future fields without
     changing the key name.

4. Backend-specific path keys only
   - Rejected: both built-in backends already share the same flag shape and high-level meaning.

## Rollout / Compatibility

- Additive only.
- Existing backend wrapper support lowers the implementation risk; the main remaining work is
  universal validation, capability advertising, backend-contract updates, and test coverage.
- The key remains usable with resume/fork flows under one deterministic rule: Claude maps the
  accepted list directly, while the current Codex fork contract fails before app-server requests
  with the pinned safe backend message instead of silently dropping the list.
- That Codex fork rejection is an accepted-input path only; invalid add-dir payloads still fail
  earlier as `AgentWrapperError::InvalidRequest`.

## Canonical authority + sync workflow

- Normative semantics for `agent_api.exec.add_dirs.v1` are owned by
  `docs/specs/universal-agent-api/extensions-spec.md`.
- Foundational terms used by the key (including effective working directory) are owned by
  `docs/specs/universal-agent-api/contract.md`.
- This ADR is rationale + an implementation plan. If any ADR wording conflicts with the normative
  specs, resolve conflicts by updating this ADR (and the implementation pack) to match the specs.

## Validation Plan (Authoritative for this ADR once Accepted)

- `make adr-check ADR=docs/adr/0021-universal-agent-api-add-dirs.md`
- Verify the canonical owner-doc semantics already exist and remain synchronized:
  - `docs/specs/universal-agent-api/extensions-spec.md` (`agent_api.exec.add_dirs.v1`)
  - `docs/specs/universal-agent-api/capabilities-schema-spec.md` (`agent_api.exec.add_dirs.v1`)
- Regenerate the canonical capability artifact with:
  - `cargo run -p xtask -- capability-matrix`
- Add backend tests proving:
  - unsupported key fails before spawn,
  - invalid shape / bounds fail before spawn,
  - absent key does not emit `--add-dir`,
  - relative paths resolve against the effective working directory,
  - absolute paths outside the working directory are accepted when valid,
  - non-directory / missing paths fail before spawn, and
  - lexical normalization/dedup behaves deterministically without requiring canonicalization,
  - Codex emits repeated `--add-dir <dir>` pairs in order after any accepted `--model` pair,
  - Claude Code emits one variadic `--add-dir <dir...>` group in order, and
  - Claude resume selector `"last"` places that variadic group before `--continue`, the final
    `--verbose` token, and the final prompt token,
  - Claude resume selector `"id"` places that variadic group before `--resume <id>`, the final
    `--verbose` token, and the final prompt token,
  - Claude fork selector `"last"` places that variadic group before `--continue --fork-session`,
    the final `--verbose` token, and the final prompt token,
  - Claude fork selector `"id"` places that variadic group before `--fork-session --resume <id>`,
    the final `--verbose` token, and the final prompt token,
  - Codex fork rejects accepted add-dir inputs for selector `"last"` and selector `"id"` before
    `thread/list` / `thread/fork` / `turn/start` with
    `AgentWrapperError::Backend { message: "add_dirs unsupported for codex fork" }`,
  - invalid fork + add-dir payloads fail as `AgentWrapperError::InvalidRequest` before the
    Codex-specific backend rejection path and before any `thread/list` / `thread/fork` /
    `turn/start` request,
  - any handle-returning surface that later cannot honor the accepted add-dir set emits exactly one
    terminal `AgentWrapperEventKind::Error` event whose safe/redacted message exactly matches the
    `AgentWrapperError::Backend { message }` surfaced through completion, and
  - deterministic runtime-rejection parity uses only
    `crates/agent_api/src/bin/fake_codex_stream_exec_scenarios_agent_api.rs` scenarios
    `add_dirs_runtime_rejection_exec`, `add_dirs_runtime_rejection_resume_last`, and
    `add_dirs_runtime_rejection_resume_id`, plus
    `crates/agent_api/src/bin/fake_claude_stream_json_agent_api.rs` scenarios
    `add_dirs_runtime_rejection_fresh`, `add_dirs_runtime_rejection_resume_last`,
    `add_dirs_runtime_rejection_resume_id`, `add_dirs_runtime_rejection_fork_last`, and
    `add_dirs_runtime_rejection_fork_id`,
  - each runtime-rejection fixture emits at least one pre-failure event before the safe message
    `add_dirs rejected by runtime`, and
  - the parity assertions explicitly prove no `ADD_DIR_RAW_PATH_SECRET`,
    `ADD_DIR_STDOUT_SECRET`, or `ADD_DIR_STDERR_SECRET` leak through user-visible events or the
    completion error, and
  - `docs/specs/universal-agent-api/capability-matrix.md` gains the
    `agent_api.exec.add_dirs.v1` row for both built-in backends in the same change that lands the
    capability.

## Decision Summary

`agent_api.exec.add_dirs.v1` is promoted as a bounded core key with explicit path semantics:
absolute and relative directory inputs are allowed, relative paths resolve against the effective
working directory, and the wrapper intentionally does not impose a containment rule that would
neutralize the feature’s purpose.
