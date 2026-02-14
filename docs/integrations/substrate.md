# Substrate Integration Guide (Agent Wrappers)

This guide describes a recommended integration pattern for using wrappers in this repo from
Substrate’s async shell/orchestrator. It is intentionally **not** a Substrate envelope contract:
Substrate remains the source of truth for its own `AgentEvent` bus and correlation fields.

## What the wrapper provides

- Live execution + streaming: spawn a CLI agent wrapper and consume typed events (Codex) or
  structured JSON output (Claude Code `--output-format=stream-json`).
- Raw line tee: optionally append each raw stdout line to an artifact file for replay/debug.
- Offline ingestion: parse a saved JSONL/NDJSON log file back into typed and/or normalized events.

Normative contracts:

- Shared ingestion contract (all wrappers): `docs/specs/wrapper-events-ingestion-contract.md`
- Codex offline parsing contract: `docs/specs/codex-thread-event-jsonl-parser-contract.md`
- Codex parsing scenarios: `docs/specs/codex-thread-event-jsonl-parser-scenarios-v1.md`
- Codex normalization semantics: `crates/codex/JSONL_COMPAT.md`

## Recommended Substrate pattern

### 0) Prefer the shared ingestion boundary

Substrate should treat wrapper stdout as **line-oriented artifacts** and, when it needs
interpretation, ingest them through `crates/wrapper_events`:

- bounded-memory line reader (8192-byte chunking + discard-mode on oversize)
- raw capture off by default (explicit policy knob)
- per-line error isolation (bad lines do not stop iteration)
- adapter opt-in (Codex / Claude Code)

This keeps `agent-wrappers` orthogonal: it does not invent Substrate correlation fields and does
not define Substrate’s envelope. Substrate attaches correlation and routing context explicitly.

### 1) Live run (primary UX)

- Use `CodexClient::stream_exec` / `stream_resume`.
- Configure:
  - `.json(true)` + `.mirror_stdout(false)` so Substrate owns rendering.
  - `.quiet(true)` unless debugging.
  - `.cd(<workspace>)` to pin execution context.
  - `.codex_home(<isolated>)` to avoid mutating a user’s global Codex state.
  - `.json_event_log(<artifact path>)` to tee raw JSONL for replay/debug.

Substrate then maps `ThreadEvent` into its own `AgentEvent` bus for UI and telemetry.

### 2) Offline replay / context extraction

The offline ingestion boundary is intentionally synchronous and line-oriented. In Substrate (Tokio
control plane), run it in `tokio::task::spawn_blocking` (or a dedicated thread) and forward results
over an async channel.

Use the tolerant iterator and decide strictness in Substrate:

- Continue on per-line errors for “best-effort replay”.
- Optionally fail-fast in tools/tests by stopping at the first error.

## Parsing behavior (v1, locked)

These behaviors are shared by streaming normalization and offline parsing:

- Blank / whitespace-only lines are ignored.
- A single trailing `\r` is trimmed (CRLF tolerance); the parser MUST NOT apply full `.trim()`.
- Unknown or unrecognized `type` values surface as per-line parse errors and do not stop parsing.
- Synthetic `turn_id` generation uses a monotonic counter scoped to the parser instance and does
  not reset on new threads within a concatenated log.

## Artifact hygiene (important for Substrate)

Treat raw wrapper logs (JSONL/NDJSON) as sensitive artifacts:

- Store them under a per-session directory (e.g., `~/.substrate/agents/codex/<session_id>/`).
- Do not mirror raw JSONL lines into Substrate’s global trace by default.
- Prefer emitting redacted, high-level summaries into Substrate’s `AgentEvent` stream; keep the full
  detail in the artifact file.
