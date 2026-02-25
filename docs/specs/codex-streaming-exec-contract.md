# Codex Streaming Exec Contract (v1)

Status: **Normative**  
Scope: live streaming via `codex::CodexClient::stream_exec*` / `stream_resume`

## Normative language

This document uses RFC 2119-style requirement keywords (`MUST`, `MUST NOT`, `SHOULD`).

## Purpose

Define a **zero-ambiguity** contract for the Codex wrapper crate’s *live streaming* APIs,
specifically around:

- when runtime driver tasks begin executing
- how `timeout` is interpreted for streaming handles
- how termination is requested and observed

This contract is intentionally scoped to the `codex` crate’s streaming runtime behavior. It does
not define cross-backend universal semantics; see `docs/specs/universal-agent-api/run-protocol-spec.md`
for the `agent_api` run lifecycle rules.

## Runtime semantics (v1, pinned)

### Spawn + driver start (pinned)

When a `codex::CodexClient` streaming method (e.g. `stream_exec`, `stream_resume`,
`stream_exec_with_env_overrides`) returns `Ok(...)`, the wrapper MUST:

- have already spawned the underlying `codex exec --json ...` process (or equivalent), and
- have started the internal driver tasks responsible for:
  - reading stdout (JSONL) and producing the typed event stream, and
  - waiting for process exit and producing the completion outcome.

Critically: starting and driving those tasks MUST NOT depend on the consumer polling/awaiting the
returned `completion` future.

Rationale: downstream orchestrators (e.g. Substrate) commonly drain `events` first and only await
`completion` later. Streaming timeouts and explicit termination must still take effect in that
pattern.

### Timeout semantics (pinned)

`CodexClientBuilder::timeout(...)` MUST be interpreted as a wall-clock bound on the streaming run
starting at handle creation time.

Concretely:

- The timeout countdown MUST start no later than the moment the streaming method returns `Ok(...)`.
- The timeout MUST NOT be delayed until the consumer first polls/awaits `completion`.

If the timeout triggers, the wrapper MUST request best-effort termination of the underlying child
process (e.g., via kill-on-drop and/or an explicit kill request).

### Explicit termination semantics (pinned)

For streaming entrypoints that expose a termination handle (e.g., `ExecStreamControl`):

- `ExecTerminationHandle::request_termination()` MUST be idempotent and best-effort.
- A termination request MUST be observed by the streaming driver regardless of whether the consumer
  is polling/awaiting `completion`.
- Once termination is requested, the wrapper MUST request best-effort termination of the underlying
  child process.
- If the consumer continues polling `events`, the `events` stream SHOULD reach finality (`None`)
  once the underlying process has been terminated and stdout has been fully observed/closed.

## Notes / non-goals

- This contract does not require that `completion` implies the `events` stream has been fully
  drained by the consumer.
- This contract does not supersede the universal `agent_api` completion gating rules (DR-0012);
  `agent_api` is responsible for enforcing those semantics at the universal boundary.

