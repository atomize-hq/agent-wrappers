# ADR 0005: Public API for Parsing Codex JSONL Logs (Reusing Normalization)

Date: 2026-02-02  
Status: Proposed

## Context

The `crates/codex` crate provides an async wrapper around the Codex CLI, including:

- Typed parsing of `codex exec --json` output as a JSONL stream (`ThreadEvent`).
- A compatibility/normalization layer that tolerates upstream drift and missing context (documented in `crates/codex/JSONL_COMPAT.md`).
- A raw JSONL tee facility (`json_event_log`) that appends each stdout line to a file before parsing.

In practice, consumers (including multi-agent/orchestrator CLIs) want to ingest Codex session logs *after the fact*:

- Replaying or summarizing a prior run from a saved `json_event_log`.
- Debugging failures using logs captured by the wrapper or stored under `CODEX_HOME` (e.g., transcripts under `conversations/`).
- Building higher-level adapters that convert Codex-native events into an agent-agnostic `AgentWrapperEvent` stream.

Today, the tolerant parsing and context inference logic is effectively available only through the live streaming APIs (`stream_exec`, `stream_resume`) that spawn a Codex process and read stdout in real time. There is no small, public, offline parsing API that reuses the same normalization rules.

## Problem

Without a public “parse JSONL log” API:

- Downstream tools must re-implement Codex JSONL parsing and normalization (high risk of drift and divergence).
- `json_event_log` is less useful as an interoperability boundary: it produces a file that cannot be parsed by the wrapper itself without reusing internal functions.
- We cannot cleanly test “log ingestion” paths as a first-class behavior without reaching into internals.

We want to support offline ingestion while keeping the wrapper’s responsibility bounded to Codex-specific concerns (not analytics/reporting).

## Decision

Add a small, public parsing API to `crates/codex` that:

1. Parses Codex JSONL event logs into the existing typed event model (`ThreadEvent`).
2. Reuses the same normalization and context-inference logic used by live streaming.
3. Is tolerant by default (per-line success/error), without aborting the entire parse on the first bad line.
4. Does not introduce cross-agent concepts (no agent-agnostic schema in this crate).

This API will enable downstream orchestration tooling to treat `json_event_log` (and other JSONL captures) as a stable “best-effort” boundary: Codex JSONL → `ThreadEvent` stream.

## Normative references

If there is any conflict between this ADR and the following specs, the specs take precedence:

- Contract: `docs/specs/codex-thread-event-jsonl-parser-contract.md`
- Scenario catalog: `docs/specs/codex-thread-event-jsonl-parser-scenarios-v1.md`
- Normalization policy: `crates/codex/JSONL_COMPAT.md`

## Detailed Design (non-normative summary)

The v1 API is intentionally small and Codex-specific:

- A stateful parser that can parse individual JSONL lines while maintaining thread/turn context.
- Reader/file helpers that produce a deterministic sequence of “line number + parse outcome” records.

## API stability

This API is “best-effort” but is part of the wrapper’s supported surface:

- The public parsing API version tracks the crate’s semver.
- Breaking changes to parsing semantics or public signatures require a major version bump of `crates/codex`.
- Changes in accepted upstream shapes should be handled by extending normalization and/or adding aliases, not by breaking downstream callers.

## Testing (normative)

Add unit tests that exercise the public API using the existing versioned JSONL fixtures (currently used by `crates/codex/tests/jsonl_compat.rs`), ensuring:

- Fixture logs parse identically via live streaming normalization and the new offline API.
- Malformed lines produce `Err(ExecStreamError)` without stopping subsequent lines from being parsed.
- Context inference behaves the same as the streaming path.

## Non-goals

- Do not add an agent-agnostic event schema (`AgentWrapperEvent`) to `crates/codex`.
- Do not add analytics/reporting (summaries, metrics, dashboards) to the wrapper.
- Do not attempt to parse non-`ThreadEvent` logs (e.g., arbitrary `codex-*.log` trace logs) as part of this ADR.

## Alternatives Considered

1. Keep parsing internal; force downstream tools to parse JSONL themselves
   - Rejected: creates duplicated, drifting normalization logic and undermines the value of `json_event_log`.

2. Expose a “raw JSON” parser only (`serde_json::Value`), leaving normalization to callers
   - Rejected: normalization and context inference are exactly the hard parts callers would otherwise re-implement.

3. Create a separate crate (e.g., `codex-jsonl`) for parsing
   - Deferred: possible later if the parsing surface must be shared without the rest of the wrapper, but today it adds packaging overhead without clear benefit.

4. Only provide example code (not a supported library API)
   - Rejected: examples are not a stable contract and cannot be relied upon by downstream crates.

## Consequences

### Benefits

- Downstream tools can ingest `json_event_log` outputs and other JSONL captures without duplicating normalization logic.
- The wrapper becomes self-consistent: “tee raw JSONL” and “parse raw JSONL” are both supported.
- Improves testability and debuggability for log-based workflows.

### Tradeoffs / Risks

- Increases the public API surface area of `crates/codex`, requiring semver discipline.
- Parser semantics are coupled to upstream drift; maintenance cost is mitigated by existing fixtures and the compatibility policy in `crates/codex/JSONL_COMPAT.md`.

## Follow-ups

- Implement the public parser as a thin wrapper around the existing normalization logic (refactor internal normalization/context types as needed).
- Add tests that run the fixture corpus through the new public API.
- Update `crates/codex/README.md` to document the offline parsing entrypoint and recommended usage with `json_event_log`.
