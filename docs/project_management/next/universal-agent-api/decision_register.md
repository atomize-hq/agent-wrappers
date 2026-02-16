# Decision Register — Universal Agent API

Status: Draft  
Date (UTC): 2026-02-16  
Feature directory: `docs/project_management/next/universal-agent-api/`

This register records the non-trivial architectural decisions required to make ADR 0009 execution-ready.
Each decision is exactly two options (A/B) with explicit tradeoffs and one selection.

## DR-0001 — Streaming semantics across heterogeneous backends

**A) Require live streaming for all backends**
- Pros: single mental model; cancellation/progress is uniform.
- Cons: forces Claude Code (and future CLIs) to implement true streaming APIs immediately; higher complexity and higher coupling to backend I/O.

**B) Allow buffered runs; gate “live streaming” via capabilities (Selected)**
- Pros: fits current backend reality (Codex streams, Claude may buffer); enables incremental backend upgrades; keeps core API stable.
- Cons: consumers must check capabilities for real-time UX; “event stream” may be post-hoc for some agents.

**Selected:** B

## DR-0002 — Relationship to `wrapper_events` normalized events

**A) Keep `agent_api` event envelope independent (Selected)**
- Pros: avoids breaking ADR 0007 contract; preserves wrapper_events as ingestion-only; keeps identity open-set without changing wrapper_events enums.
- Cons: two envelopes exist in the repo; consumers must choose which they want.

**B) Evolve `wrapper_events` to an open-set identity and reuse its normalized event types**
- Pros: fewer shapes; could unify ingestion and universal API views.
- Cons: breaks/churns ADR 0007; forces additional migration work; risks mixing concerns (ingestion vs orchestration).

**Selected:** A

## DR-0003 — Capability namespace strategy

**A) Flat string set (e.g., `\"run\"`, `\"tools\"`, `\"stream\"`)**
- Pros: minimal, simple.
- Cons: collision risk as agent-specific capabilities grow; unclear ownership/stability.

**B) Namespaced capability ids (Selected)**
- Pros: avoids collisions; makes ownership explicit; supports stable core + agent extensions.
- Cons: slightly more verbose.

**Selected:** B

Namespace rules (normative for specs):
- Core operation capabilities: `agent_api.<cap>` (example: `agent_api.run`, `agent_api.events`).
- Backend-specific capabilities: `backend.<agent_kind>.<cap>` (example: `backend.codex.exec_stream`).

## DR-0004 — Event payload extensibility

**A) Strict schema only (no extension payload)**
- Pros: strongest stability; easiest to validate.
- Cons: blocks surfacing agent-specific structured data without schema churn.

**B) Allow bounded extension payload as JSON (Selected)**
- Pros: supports heterogeneity; avoids forcing least-common-denominator tool schemas.
- Cons: requires explicit size/redaction rules; less type safety for extensions.

**Selected:** B

