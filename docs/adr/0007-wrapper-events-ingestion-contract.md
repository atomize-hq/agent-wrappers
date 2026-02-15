# ADR 0007: Wrapper-Level JSONL/NDJSON Ingestion Contract (Shared)

Date: 2026-02-14  
Status: Proposed

## Context

This repository contains multiple CLI wrapper crates (Codex, Claude Code, and future agents).
Several of these wrappers emit line-oriented output streams (JSONL, NDJSON, “stream-json”).

Consumers (notably Substrate) need a stable, safe ingestion boundary for these streams:

- bounded memory (no accidental “allocate the whole line” behavior)
- per-line error isolation (bad lines don’t stop the run)
- raw retention off by default (raw output may contain secrets)
- opt-in adapters per wrapper (keep the base crate independent)
- no collision with Substrate’s own `AgentEvent` envelope and correlation model

Codex already has a public, Codex-specific offline parsing contract (ADR 0005 + spec). We want a
separate, wrapper-level contract that is **shared** across wrappers and is explicitly **not**
Substrate’s envelope.

## Decision

Introduce a new crate:

- `crates/wrapper_events` (`wrapper_events`)

It provides:

1. A **bounded** line reader for sync + optional tokio ingestion (8192-byte chunking + discard-mode).
2. A generic parser trait (`LineParser`) with redacted-by-default errors.
3. A safe raw capture mechanism (off by default, budgeted, deterministic).
4. Minimal normalized event types (`NormalizedWrapperEvent`) for consumers that want a unified view.
5. Feature-gated adapters (`codex`, `claude_code`) so consumers only pull wrapper dependencies they
   choose.

## Normative references

If there is any conflict between this ADR and the following spec, the spec takes precedence:

- `docs/specs/wrapper-events-ingestion-contract.md`

Codex-specific parsing remains governed by:

- `docs/specs/codex-thread-event-jsonl-parser-contract.md`

## Consequences

### Positive

- Substrate and other consumers gain a stable “ingest wrapper logs” boundary with explicit safety
  knobs and bounded memory.
- Adding new wrapper adapters becomes mechanical: implement `LineParser` + optional normalization.
- Raw retention defaults to off and is budgeted when enabled.

### Tradeoffs / costs

- Introduces a new shared crate that must remain conservative and stable.
- Adapter authors must provide redacted summaries and keep “full details” behind an explicit sink.

