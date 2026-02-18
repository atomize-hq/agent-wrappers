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

## DR-0005 — Raw backend line capture policy

**A) Allow opt-in raw backend line capture (via extensions)**
- Pros: enables “raw log viewer” and deep debugging.
- Cons: high secret-leak risk; complicates bounds; adds a permanent compatibility burden.

**B) Forbid raw backend line capture in v1 (Selected)**
- Pros: safe-by-default; simpler contract; aligns with security posture.
- Cons: raw capture must be implemented outside this API (ingestion boundary).

**Selected:** B

## DR-0006 — Stable payload fields for core event kinds

**A) Encode core payloads inside `data` JSON only**
- Pros: fewer struct fields.
- Cons: forces consumers to parse JSON; higher drift risk across backends; hurts orthogonality.

**B) Add stable `text`/`message` fields to `AgentWrapperEvent` (Selected)**
- Pros: backend-agnostic consumption for `TextOutput`/`Status`/`Error`; easier to document and enforce.
- Cons: slightly larger struct surface.

**Selected:** B

## DR-0007 — Provided backends without type leakage

**A) Expose backend constructors that use `codex::*` / `claude_code::*` types**
- Pros: reuse existing builders directly.
- Cons: leaks backend types into the universal contract; breaks orthogonality.

**B) Provide feature-gated backends with std/serde-friendly config types (Selected)**
- Pros: avoids leakage; stable, portable contract.
- Cons: some config duplication/mapping.

**Selected:** B

## DR-0008 — Extensions ↔ capabilities validation contract

**A) Best-effort extensions (unknown keys ignored)**
- Pros: flexible; fewer errors.
- Cons: silent degradation; inconsistent behavior across backends; hard to debug.

**B) Fail-closed with 1:1 mapping to capability ids + pre-spawn validation (Selected)**
- Pros: deterministic; explicit; safe.
- Cons: callers must be explicit and check capabilities.

**Selected:** B

## DR-0009 — Bounds enforcement behavior

**A) Hard-error the run on any oversize field**
- Pros: strict.
- Cons: fragile; increases failure rate; can deadlock consumers if not carefully handled.

**B) Deterministic drop/truncate with explicit rules (Selected)**
- Pros: safe; predictable; consistent across backends.
- Cons: some information loss in extreme cases.

**Selected:** B

## DR-0010 — CI checkpoint workflow as a C0 deliverable

**A) Defer CP1 workflow until later (CP1 task creates it)**
- Pros: less work in C0.
- Cons: checkpoint becomes non-deterministic; later triads depend on missing infra.

**B) Require the workflow in C0 (Selected)**
- Pros: CP1 is real and wired early; avoids “forgot to add workflow” drift.
- Cons: small additional work in C0.

**Selected:** B

## DR-0011 — Codex live streaming capability advertisement

**A) Codex advertises only `agent_api.events`**
- Pros: fewer capability ids.
- Cons: loses live-stream signal; violates run protocol semantics.

**B) Codex MUST include `agent_api.events.live` (Selected)**
- Pros: accurate capability signal; supports real-time UX.
- Cons: none meaningful.

**Selected:** B

## DR-0012 — `completion` vs event stream termination

**A) `completion` may resolve before the event stream terminates**
- Pros: slightly simpler backend implementation for buffered modes.
- Cons: consumer footgun; risks dropped events.

**B) `completion` MUST NOT resolve until the event stream is final (Selected)**
- Pros: safe consumer contract; no dropped buffered events.
- Cons: requires adapters to flush/close streams before resolving completion.

**Selected:** B
