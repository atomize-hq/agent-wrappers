# Codex ThreadEvent JSONL Parser Scenario Catalog (v1)

Status: **Normative** (paired with the parser contract)  
Scope: acceptance criteria for the offline JSONL parsing API in ADR 0005

## Normative language

This document uses RFC 2119-style requirement keywords (`MUST`, `MUST NOT`).

## Purpose

Define a **zero-ambiguity** set of acceptance scenarios that lock down:

- which fixture corpora MUST parse successfully,
- how malformed input MUST be surfaced (and that parsing continues),
- and that the offline parser matches the streaming normalization semantics.

## Normative references

- Contract: `docs/specs/codex-thread-event-jsonl-parser-contract.md` (authoritative API/signatures)
- Normalization semantics: `crates/codex/JSONL_COMPAT.md`
- Existing fixture corpus: `crates/codex/examples/fixtures/versioned/**`

## Fixture corpus (normative)

The following fixture files are authoritative inputs for v1:

- `crates/codex/examples/fixtures/versioned/0.61.0/streaming.jsonl`
- `crates/codex/examples/fixtures/versioned/0.61.0/resume.jsonl`
- `crates/codex/examples/fixtures/versioned/0.61.0/malformed.jsonl`
- `crates/codex/examples/fixtures/versioned/0.77.0/streaming.jsonl`
- `crates/codex/examples/fixtures/versioned/0.77.0/resume.jsonl`
- `crates/codex/examples/fixtures/versioned/0.77.0/malformed.jsonl`

If this repository adds a new versioned JSONL fixture directory under
`crates/codex/examples/fixtures/versioned/<version>/`, this scenario catalog MUST be updated to
include that version’s fixtures (as part of the same change).

## Scenario A: Parse `streaming.jsonl` fixtures without errors

For each `<version>` fixture set listed above, parsing the corresponding `streaming.jsonl` via the
offline API MUST satisfy:

- The emitted record stream MUST contain at least one successful `thread.started` (or normalized
  equivalent) `ThreadEvent`.
- The emitted record stream MUST contain at least one successful `turn.started` `ThreadEvent`.
- The emitted record stream MUST contain at least one successful `item.*` `ThreadEvent`.
- The emitted record stream MUST contain **zero** error outcomes for that file.

## Scenario B: Parse `resume.jsonl` fixtures without errors

For each `<version>` fixture set listed above, parsing the corresponding `resume.jsonl` via the
offline API MUST satisfy:

- The emitted record stream MUST contain at least one successful `ThreadEvent::ThreadStarted`
  (note: `thread.resumed` must normalize to `ThreadEvent::ThreadStarted`).
- The emitted record stream MUST contain at least one successful `turn.started` `ThreadEvent`.
- The emitted record stream MUST contain at least one successful `item.*` `ThreadEvent`.
- The emitted record stream MUST contain **zero** error outcomes for that file.

## Scenario C: Malformed input yields per-line errors and continues

For each `<version>` fixture set listed above, parsing the corresponding `malformed.jsonl` via the
offline API MUST satisfy:

- The emitted record stream MUST contain **at least one** error outcome corresponding to a malformed
  line (e.g., invalid JSON).
- The emitted record stream MUST contain **at least one** successful `ThreadEvent::ThreadStarted`
  outcome after the malformed line.
- The emitted record stream MUST contain **at least one** successful `turn.started` outcome after
  the malformed line.

The offline parser MUST NOT abort parsing early due solely to a malformed line.

## Scenario D: Streaming normalization equivalence (offline vs streaming)

The offline parser MUST share the same normalization behavior as the streaming parser. At minimum,
the following equivalence checks MUST exist in tests:

- For each `<version>` `streaming.jsonl`, collect the sequence of successfully parsed `ThreadEvent`
  values using the offline parser.
- Collect the sequence of successfully parsed `ThreadEvent` values using the live streaming parser
  normalization path (as exercised by `crates/codex/tests/jsonl_compat.rs` or an equivalent
  streaming-backed test harness).
- The two success sequences MUST be equivalent under `serde_json` semantic equality when each
  event is serialized with `serde_json::to_value`.

Rationale: this avoids brittle direct `Eq` requirements while still asserting identical shapes and
normalization outcomes.

## Scenario E: CRLF tolerance (trailing `\\r`)

Offline parsing MUST tolerate Windows CRLF artifacts:

- Given any valid JSON event line `L` from a fixture file, constructing `L + "\\r"` MUST parse to
  the same `ThreadEvent` as `L`.
- This MUST be tested at least for:
  - one `thread.started` line, and
  - one `item.*` line.

Note: the parser MUST trim only the trailing `\\r` (not full `.trim()`), per `crates/codex/JSONL_COMPAT.md`.

## Scenario F: Unknown `type` yields a per-line error and continues

Offline parsing MUST surface unknown event types as a per-line error without aborting the parse:

- Given an input stream containing:
  1. a valid `thread.started` line
  2. a line with `"type": "some.new.event"` (valid JSON, unknown `type`)
  3. a valid `turn.started` line
- The emitted record stream MUST include:
  - success for (1),
  - an error outcome for (2) (typically `ExecStreamError::Parse` from typed deserialization),
  - success for (3).

## Scenario G: `line_number` reflects physical lines (including blanks)

The offline parser MUST preserve physical line numbering even when blank lines emit no record.
At minimum, a test MUST construct an input buffer with interleaved blank lines and assert that
the emitted `ThreadEventJsonlRecord.line_number` values match the original physical line numbers.

Example input (physical line numbers shown for clarity):

1. `{"type":"thread.started", ...}`
2. *(blank line)*
3. `{"type":"turn.started", ...}`
4. *(blank line)*
5. `{"type":"item.output_text.delta", ...}`

Required outcomes:

- Records are emitted only for lines 1, 3, and 5.
- The emitted records MUST carry `line_number` values `1`, `3`, and `5` respectively.
- Any error outcomes in this scenario MUST still carry the correct `line_number` so downstream
  tooling can report precise locations.
