# ADR-0017 — Session/thread id surfacing (for resume-by-id UX)
#
# Note: Run `make adr-fix ADR=docs/adr/0017-universal-agent-api-session-thread-id-surfacing.md`
# after editing to update the ADR_BODY_SHA256 drift guard.

## Status
- Status: Draft (implementation plan; normative semantics are pinned in the Universal Agent API specs)
- Date (UTC): 2026-02-28
- Owner(s): spensermcconnell

## Scope

- Define a universal surface for a backend-defined session/thread identifier so “resume-by-id” is ergonomic.
- Define:
  - where the identifier is emitted (event stream vs completion),
  - the stable JSON schema/name,
  - capability gating, and
  - bounds + redaction posture.
- Map the universal surface to built-in backends:
  - Codex (`thread_id` from `thread.started` / `thread.resumed`)
  - Claude Code (`session_id` from `--output-format=stream-json`)

## Related Docs

- Session extension keys (resume/fork):
  - `docs/adr/0015-universal-agent-api-session-extensions.md`
  - `docs/specs/universal-agent-api/extensions-spec.md`
- Event + completion envelope bounds:
  - `docs/specs/universal-agent-api/contract.md`
  - `docs/specs/universal-agent-api/event-envelope-schema-spec.md`
  - `docs/specs/universal-agent-api/run-protocol-spec.md`
- Backend parser inputs (authoritative for extraction points):
  - `docs/specs/codex-thread-event-jsonl-parser-contract.md`
  - `docs/specs/claude-stream-json-parser-contract.md`
  - `crates/codex/src/events.rs` (`ThreadEvent::ThreadStarted.thread_id`)
- Backlog:
  - `docs/backlog.json` (`uaa-0015`)

## Executive Summary (Operator)

ADR_BODY_SHA256: 6b4c9723a55d8b29586b59a5f59af7ccfe8613aeb9cbe90a2e0ea3e17d0406bb

### Decision (draft)

- Introduce capability id: `agent_api.session.handle.v1`.
- When a backend advertises `agent_api.session.handle.v1`, it MUST surface the current run’s
  backend-defined session/thread identifier as a small, bounded JSON facet:
  - emitted once as an early `AgentWrapperEventKind::Status` event `data` payload (as soon as the
    id is known and within bounds), and
  - attached to `AgentWrapperCompletion.data` whenever a completion is produced and the id is known
    and within bounds.
- Stable schema (closed, versioned):

```json
{
  "schema": "agent_api.session.handle.v1",
  "session": { "id": "string" }
}
```

- `session.id` is an **opaque** backend-defined identifier suitable for round-tripping into:
  - `agent_api.session.resume.v1` with selector `"id"`, and
  - `agent_api.session.fork.v1` with selector `"id"`.

### Why

- Resume-by-id UX requires callers to *discover and persist* an id. Today the id exists in backend
  streams but is not exposed in a cross-backend stable surface.
- Using `AgentWrapperEvent.data` and `AgentWrapperCompletion.data` avoids breaking the v1 Rust API
  surface while staying within existing bounds/redaction rules.

## Problem / Context

The Universal Agent API introduces universal session semantics via extension keys like:

- `agent_api.session.resume.v1`
- `agent_api.session.fork.v1`

These keys require an id for “resume/fork by identifier” flows, but callers currently have no
universal, stable way to *obtain* that id from a run.

Both built-in backends already expose an identifier in their native streaming/log shapes:

- Codex: `ThreadEvent::ThreadStarted.thread_id` (`thread.started` / `thread.resumed`)
- Claude Code: `session_id` on stream-json events (e.g., `SystemInit`)

We want to surface this identifier as bounded metadata so orchestration code can implement
resume-by-id without per-backend parsing.

## Goals

- Make a session/thread identifier discoverable to callers in a cross-backend consistent way.
- Keep the id opaque (backend-defined string) while making it easy to persist and round-trip.
- Keep the surface safe-by-default:
  - bounded,
  - redacted (no raw backend lines), and
  - capability-gated.
- Support both consumer styles:
  - streaming consumers (observing events), and
  - completion-only consumers (dropping events).

## Non-Goals

- Standardizing the id format across agents (backend-defined, opaque).
- Guaranteeing the backend will persist sessions forever (backend-defined retention).
- Adding a “session listing” API to the Universal Agent API.
- Introducing new required fields on `AgentWrapperEvent` or `AgentWrapperCompletion` (breaking).

## Proposed Design (Draft)

### Capability

- Capability id: `agent_api.session.handle.v1`.
- Meaning: when present in `AgentWrapperCapabilities.ids`, the backend will emit the session handle
  facet as described below.

### Facet schema (stable, versioned)

`AgentWrapperEvent.data` and `AgentWrapperCompletion.data` MAY carry backend-specific structured
payloads. This ADR defines one such payload:

```json
{
  "schema": "agent_api.session.handle.v1",
  "session": {
    "id": "string"
  }
}
```

Rules:
- `schema` MUST equal `agent_api.session.handle.v1`.
- `session.id` MUST be a non-empty string after trimming.
- The payload MUST be treated as **metadata-only** (not a raw log line, not tool I/O).

### Emission points

When `agent_api.session.handle.v1` is advertised:

1) **Event stream (early)**
   - The backend MUST emit exactly one `AgentWrapperEventKind::Status` event carrying the facet in
     `AgentWrapperEvent.data` as soon as the session/thread id is known and within bounds.
   - Preferred attachment points:
     - Codex: on the mapped `ThreadEvent::ThreadStarted` event.
     - Claude Code: on the mapped `ClaudeStreamJsonEvent::SystemInit` event.
   - If a backend cannot reliably attach the facet to an existing status event, it MUST emit a
     synthetic `Status` event immediately after capturing the id.

2) **Completion**
   - If the backend produces an `AgentWrapperCompletion` (including non-zero exit completions), and
     the session/thread id is known and within bounds, it MUST attach the facet to
     `AgentWrapperCompletion.data`.
   - This supports consumers that drop the event stream and only await completion.

### Bounds + safety

- The facet MUST obey the existing bounds + enforcement behavior for `data`:
  - `docs/specs/universal-agent-api/event-envelope-schema-spec.md` (“Completion payload bounds” and
    `AgentWrapperEvent.data` constraints).
- The backend MUST NOT derive the id by embedding or parsing raw stdout/stderr lines. The id MUST
  come from the typed parsed backend event models (Codex `ThreadEvent`, Claude stream-json events).
- Additional bound (facet-level):
  - `len(session.id) <= 1024` bytes (UTF-8).
  - If the bound is violated, the backend MUST omit the facet (do not truncate, since truncation
    breaks round-tripping for resume-by-id) and SHOULD emit a safe/redacted `Status` warning.
  - In this oversize case, the id MUST be treated as “not known” for purposes of the emission
    points above (event stream + completion).

### Storage / run-local state

- Backends SHOULD store the captured id as run-local state so it can be reused for:
  - emitting the early status facet, and
  - attaching the completion facet,
  even if later backend events omit the id.

## Backend Mapping (Built-in Backends)

### Codex

- Source: `ThreadEvent::ThreadStarted.thread_id` from `codex exec --json` streaming events.
- Mapping: surface `thread_id` as `session.id` in the universal facet.
- Note: Codex `thread.started` and `thread.resumed` both map to `ThreadEvent::ThreadStarted`; this
  ADR does not require a universal “started vs resumed” discriminator in v1.

### Claude Code

- Source: `ClaudeStreamJsonEvent` session id extracted by the `claude_code` stream-json parser
  (e.g., `SystemInit { session_id, ... }`).
- Mapping: surface `session_id` as `session.id` in the universal facet.

## Alternatives Considered

- Add a new `session_id: Option<String>` field to `AgentWrapperCompletion` and/or `AgentWrapperEvent`.
  - Rejected: adding public fields to v1 structs is a breaking change for downstream consumers.
- Surface only in `AgentWrapperCompletion.data`.
  - Rejected: streaming consumers may need the id before completion (and completion may not exist
    for some terminal errors/cancellation flows).
- Surface only in the event stream.
  - Rejected: consumers who intentionally drop `events` and await `completion` would miss the id.

## Rollout / Backwards Compatibility

- Additive: backends that do not advertise `agent_api.session.handle.v1` are unchanged.
- For built-in backends, the capability should be advertised only once both:
  - extraction is implemented, and
  - emission obeys bounds/redaction posture.

## Validation Plan (Authoritative for this ADR once Accepted)

- Add tests per backend proving:
  - a `Status` event is emitted containing the facet once the id is observed,
  - `AgentWrapperCompletion.data` contains the facet on completion when id is known, and
  - bounds enforcement omits (does not truncate) oversize ids.
- Use fake-binary / fixture-backed event streams when the behavior depends on backend output shape.

## Spec Updates (landed)

The canonical specs now register this capability id and structured `data` facet:

- `docs/specs/universal-agent-api/capabilities-schema-spec.md`:
  - register `agent_api.session.handle.v1` and its semantics.
- `docs/specs/universal-agent-api/event-envelope-schema-spec.md`:
  - register the facet schema and emission rules for `AgentWrapperEvent.data` and `AgentWrapperCompletion.data`.
