# ADR 0008: Public API for Parsing Claude Code stream-json Logs

Date: 2026-02-14  
Status: Proposed

## Context

The `crates/claude_code` crate wraps the Claude Code CLI for non-interactive operation, including
`claude --print --output-format=stream-json`.

Consumers (notably Substrate and other orchestrators) want to ingest captured Claude stream-json
artifacts *after the fact*:

- Offline replay/summarization of a prior run.
- Debugging failures from stored stdout artifacts.
- Converting Claude-native output into a wrapper-agnostic normalized event stream.

We need a small, public, offline parsing API that:

- is stateful and line-oriented (JSONL/NDJSON style)
- is tolerant (per-line error isolation)
- is forward-compatible with upstream drift (unknown outer types are not fatal)
- does not collide with Substrate’s `AgentEvent` envelope or correlation model

## Decision

Add a public Claude stream-json parser API to `crates/claude_code` that:

1. Parses a single logical line into a typed raw event model (`ClaudeStreamJsonEvent`).
2. Exposes both `parse_line(&str)` and `parse_json(&Value)` entrypoints.
3. Emits a narrowly scoped error taxonomy (`JsonParse`, `TypedParse`, `Normalize`).
4. Treats unknown outer `type` strings as `Unknown` events (not errors).

## Normative reference

If there is any conflict between this ADR and the following spec, the spec takes precedence:

- `docs/specs/claude-stream-json-parser-contract.md`

## Consequences

### Positive

- Offline ingestion is a supported contract surface (not re-implemented downstream).
- The wrapper can align live and offline parsing semantics over time.
- Unknown upstream drift can be tolerated without breaking consumers.

### Tradeoffs

- Adds a public API surface that must be maintained with semver discipline.
- Requires a fixture corpus to validate behavior and prevent regressions.

