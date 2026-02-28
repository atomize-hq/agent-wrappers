# Cross-Documentation Verification Report

**Target**: `agent_api.session.*` (session extensions + handle surfacing)  
**Date (UTC)**: 2026-02-28  
**Documents Checked**: backlog items + ADR-0015/0017 + relevant Universal Agent API specs

## Executive Summary

Session/thread semantics are now consistently defined across:
- backlog work items (`docs/backlog.json`),
- ADRs (0015/0017), and
- the canonical Universal Agent API specs under `docs/specs/universal-agent-api/`.

The core extension keys (`agent_api.session.resume.v1`, `agent_api.session.fork.v1`) remain owned by
the extensions registry, while session/thread id discovery is standardized via capability
`agent_api.session.handle.v1` and a bounded `data` facet emitted on an early `Status` event and on
`AgentWrapperCompletion.data`.

## Consistency Score: 100/100

- Conflicts: 0
- Gaps: 0
- Duplication: 0
- Drift: 0

Recommendation: **PROCEED** (documentation/spec alignment is consistent; implementation remains tracked in backlog)

## Documents Checked

- Backlog:
  - `docs/backlog.json` (`uaa-0004`, `uaa-0005`, `uaa-0007`, `uaa-0011`, `uaa-0013`, `uaa-0015`, `uaa-0017`)
- ADRs:
  - `docs/adr/0015-universal-agent-api-session-extensions.md`
  - `docs/adr/0017-universal-agent-api-session-thread-id-surfacing.md`
- Normative anchors (specs):
  - `docs/specs/universal-agent-api/contract.md`
  - `docs/specs/universal-agent-api/run-protocol-spec.md`
  - `docs/specs/universal-agent-api/capabilities-schema-spec.md`
  - `docs/specs/universal-agent-api/extensions-spec.md`
  - `docs/specs/universal-agent-api/event-envelope-schema-spec.md`
- Backend parser inputs (authoritative for extraction points):
  - `docs/specs/claude-stream-json-parser-contract.md` (`session_id`)
  - `docs/specs/codex-thread-event-jsonl-parser-contract.md` (ThreadEvent parsing context)
  - `crates/codex/src/events.rs` (`ThreadEvent::ThreadStarted.thread_id`)

## Positive Findings

- ✅ Session controls are consistently expressed as capability-gated extension keys under `agent_api.session.*`,
  with mutual exclusivity and closed `.v1` schemas centralized in `extensions-spec.md`.
- ✅ Session/thread id discovery is now a first-class, bounded, capability-gated surface (`agent_api.session.handle.v1`)
  and does not require per-backend log parsing by downstream consumers.
- ✅ Backlog mapping guidance for Codex fork now matches the canonical spec posture (headless `codex app-server` JSON-RPC).

## Remediation Landed During This Review

- Marked `uaa-0013` (session bucket rubric + tooling) as **done** and marked `uaa-0011` as **done** (redundant design item; decision is separate keys).
- Updated `uaa-0015` to reflect the ADR-0017 decision and to include required spec + implementation steps.
- Added `uaa-0017` as an explicit prerequisite task to add typed id accessor helpers (`ThreadEvent::thread_id()`, `ClaudeStreamJsonEvent::session_id()`) for reuse by both `wrapper_events` and `agent_api`.
- Registered `agent_api.session.handle.v1` in:
  - `docs/specs/universal-agent-api/capabilities-schema-spec.md` (capability semantics)
  - `docs/specs/universal-agent-api/event-envelope-schema-spec.md` (facet schema + emission rules)
  - `docs/specs/universal-agent-api/extensions-spec.md` (cross-reference for resume/fork-by-id flows)
