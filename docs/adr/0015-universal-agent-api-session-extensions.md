# ADR-0015 — Universal Agent API session extensions (`agent_api.session.*`)
#
# Note: Run `make adr-fix ADR=docs/adr/0015-universal-agent-api-session-extensions.md` after editing
# to update the ADR_BODY_SHA256 drift guard.

## Status
- Status: Draft (implementation plan; normative semantics are pinned in the Universal Agent API specs)
- Date (UTC): 2026-02-26
- Owner(s): spensermcconnell

## Scope

- Universal session/thread semantics exposed via `AgentWrapperRunRequest.extensions`:
  - resume the most recent session in a working directory
  - resume a session by identifier
  - fork a new session from an existing one
- Capability bucketing and tooling support for session semantics:
  - introduce `agent_api.session.*` as a first-class bucket (docs + generators)

## Related Docs

- Universal Agent API baselines:
  - `docs/specs/universal-agent-api/contract.md`
  - `docs/specs/universal-agent-api/run-protocol-spec.md`
  - `docs/specs/universal-agent-api/capabilities-schema-spec.md`
  - `docs/specs/universal-agent-api/extensions-spec.md`
- Tooling:
  - `crates/xtask/src/capability_matrix.rs`
- Follow-on work (not in scope for this ADR):
  - Session/thread id surfacing (resume-by-id UX): `docs/backlog.json` (uaa-0015)

## Executive Summary (Operator)

ADR_BODY_SHA256: 4a68939d08f9a6cc385cdc890e5c0052f6c880cff8632c35032552608bfafb11

### Changes (operator-facing)

- Add a core `agent_api.session.*` bucket for session semantics (orthogonal to exec policy).
- Define universal, capability-gated session extension keys:
  - `agent_api.session.resume.v1` — resume an existing session then send a follow-up prompt
  - `agent_api.session.fork.v1` — fork from an existing session then send a follow-up prompt
- Update capability bucketing tooling so session capabilities group mechanically in generated artifacts.

## Problem / Context

CLI agents generally expose “session” or “thread” semantics:
- continue the most recent conversation in a directory,
- resume a prior session by id, and
- fork a new session from an existing one.

Today these controls are backend-specific and require per-agent branching in orchestrators.

The Universal Agent API is designed to keep optional features behind:
- explicit capability discovery (`AgentWrapperCapabilities.ids`), and
- fail-closed extension validation (`AgentWrapperRunRequest.extensions`).

Session/thread operations fit this model and should be standardized as core extension keys under a
first-class bucket so onboarding and audits can group them mechanically.

## Goals

- Define universal, capability-gated session semantics as core extension keys under `agent_api.session.*`.
- Keep session semantics orthogonal to execution policy (`agent_api.exec.*`) and tool/event semantics.
- Preserve the Universal Agent API run contract shape (no new required fields; session selection is via extensions).

## Non-Goals

- Defining a universal session listing/inspection API.
- Guaranteeing the same session id format across agents (ids remain backend-defined).
- Exposing “resume with no follow-up prompt” as a universal operation (the universal run contract always includes a prompt).

## Proposed Design (Draft)

### Capability bucket

- Introduce `agent_api.session.*` as a first-class bucket in the capability rubric and tooling.

### Core extension keys

Define two core extension keys, owned by `docs/specs/universal-agent-api/extensions-spec.md`:

- `agent_api.session.resume.v1` (object):
  - selects a prior session (most recent or by id) and sends `request.prompt` as a follow-up message.
- `agent_api.session.fork.v1` (object):
  - selects a prior session (most recent or by id), forks to a new session, and sends `request.prompt`.

These keys are mutually exclusive for a single run request.

### Why separate keys (not a single `agent_api.session.v1` key)?

Using separate keys:
- enables incremental capability advertising (a backend can support resume without claiming fork),
- keeps capability gating coarse but accurate (key-level), and
- avoids creating a single “session object” that would require backends to implement all session sub-features at once.

A consolidated session object can be introduced later as a new versioned key if needed.

### Codex: choosing `exec` vs `app-server` (implementation notes)

Codex exposes multiple surfaces that can plausibly implement session semantics:

- `codex exec`:
  - headless, process-per-run
  - designed around a single “turn” with JSONL streaming (`--json`)
  - supports resuming via `codex exec resume`, but does not expose a fork primitive
- `codex app-server`:
  - headless JSON-RPC server surface (stdio by default)
  - exposes explicit thread/turn methods including `thread/list`, `thread/resume`, `thread/fork`,
    `turn/start`, and `turn/interrupt`

Recommendation for `agent_api.session.*`:

- `agent_api.session.resume.v1`:
  - Prefer implementing via `codex exec resume` (simplest lifecycle and already aligned with the
    current `agent_api` Codex adapter design).
  - Using app-server `thread/resume` + `turn/start` is acceptable if we already have an app-server
    runtime running and want to avoid spawning a new exec process.
- `agent_api.session.fork.v1`:
  - Implement via app-server `thread/fork` + `turn/start`.
  - Do not rely on `codex fork` for `agent_api` headless semantics (it is an interactive/TUI flow).

Surface switching nuance:
- “fork via app-server, then resume via exec” is plausible only if the forked thread is persisted
  in a store that `codex exec resume` can address by id, and the id formats are compatible.
- To keep `agent_api` semantics deterministic and reduce cross-surface coupling, prefer staying on
  the app-server surface for the follow-up prompt when implementing fork.

### Claude Code: session semantics in headless `--print` (implementation notes)

Claude Code session semantics are available in headless mode via the `claude --print` surface.
There is no separate “app-server” surface in this repo analogous to Codex; the backend mapping is
flag-driven.

Mapping guidance for `agent_api.session.*`:

- `agent_api.session.resume.v1`:
  - `selector == "last"`:
    - map to `claude --print --continue <PROMPT>`
  - `selector == "id"`:
    - map to `claude --print --resume <SESSION_ID> <PROMPT>`
- `agent_api.session.fork.v1`:
  - `selector == "last"`:
    - map to `claude --print --continue --fork-session <PROMPT>`
  - `selector == "id"`:
    - map to `claude --print --fork-session --resume <SESSION_ID> <PROMPT>`

Notes:
- Claude Code session ids are backend-defined strings (commonly UUIDs).
- `agent_api.session.*` semantics remain “resume/fork + send follow-up prompt”; the universal run
  contract still requires a non-empty prompt.
- Canonical, testable Claude argv + non-interactive mapping is pinned in:
  - `docs/specs/claude-code-session-mapping-contract.md`

## User Contract (Authoritative)

Canonical semantics for the keys and their schemas are defined in:
- `docs/specs/universal-agent-api/extensions-spec.md`

Pinned invariants (restated here):

- Capability gating (fail-closed):
  - Backends MUST advertise each supported session extension key in `AgentWrapperCapabilities.ids`.
  - If a request includes an unsupported session key, the backend MUST fail the run as
    `AgentWrapperError::UnsupportedCapability` before spawning any process.
- Prompt requirements:
  - The universal run contract requires a non-empty prompt; session resume/fork is defined only as
    “resume/fork + send follow-up prompt”.
- Mutual exclusivity:
  - A request MUST NOT include both `agent_api.session.resume.v1` and `agent_api.session.fork.v1`.

## Architecture Shape

- No changes to the `agent_api` public Rust API types are required.
- Session selection is expressed via `AgentWrapperRunRequest.extensions` and enforced via the existing
  fail-closed capability gating rules.

## Validation Plan (Authoritative for this ADR once Accepted)

- `make adr-check ADR=docs/adr/0015-universal-agent-api-session-extensions.md`
- Update the canonical specs:
  - `docs/specs/universal-agent-api/capabilities-schema-spec.md` (bucket rubric)
  - `docs/specs/universal-agent-api/extensions-spec.md` (core key registry)
  - `docs/specs/universal-agent-api/run-protocol-spec.md` (request validation timing for prompt)

## Decision Summary

This ADR introduces `agent_api.session.*` as a first-class bucket and pins a minimal set of
universal session semantics as two core, versioned extension keys (`resume.v1` and `fork.v1`).

## Appendix A — Selection-failure translation guardrails

Implementations MUST NOT treat “resume/fork failed with no assistant output” as equivalent to
“session not found”.

Rationale:
- Generic backend/runtime failures can also produce no assistant output.
- Misclassifying these as selection failures returns a misleading pinned error to callers and can
  mask the true failure cause (especially if the backend suppresses raw error events for safety).

Per `docs/specs/universal-agent-api/extensions-spec.md`, the pinned selection-failure messages
(`"no session found"` / `"session not found"`) are reserved for *actual* not-found outcomes.

Implementation guidance (non-normative; see the specs for requirements):
- Prefer explicit backend “not found” signals (typed/structured error payloads or stable not-found
  tokens) over negative inference from missing assistant output.
- If raw backend error events are suppressed for safety, still retain enough typed/raw context
  internally to classify not-found vs other failures.
- For non-not-found failures, surface a generic safe/redacted `AgentWrapperError::Backend` message
  (and emit a single terminal `AgentWrapperEventKind::Error` event when an events stream exists).

Tracking:
- `docs/backlog.json` (`uaa-0018`)
