# C0 Spec — Codex Wrapper Enablement (per-run env overrides for `stream_exec`)

Status: Draft  
Date (UTC): 2026-02-20  
Owner: agent-api-codex-stream-exec triad (C0)

## Scope (required)

Enable `agent_api` to apply per-run environment overrides while still executing Codex via the
Codex wrapper streaming surface (`codex::CodexClient::stream_exec`).

### In-scope deliverables

- Additive Codex wrapper API that allows **per-invocation** environment variable injection for the
  `stream_exec` spawn path (without mutating the parent env).
- The API MUST allow `agent_api` to enforce env precedence:
  - request env overrides backend env keys (request keys win).
- The API MUST support overriding wrapper-injected keys (e.g., allow request `CODEX_HOME` to
  override a configured `codex_home`).

### Out of scope (explicit)

- Any `agent_api` refactor (C1).
- Any new universal API surface or extension keys.
- Replacing Codex JSONL parsing/normalization contracts.
- Bounded ingestion refactors inside the Codex wrapper (ADR 0007 is a reference posture only).

## Requirements (normative)

### R0 — No parent env mutation

The Codex wrapper changes in C0 MUST NOT call `std::env::set_var`, `std::env::remove_var`, or any
equivalent global mutation. Env overrides MUST apply only to the spawned `tokio::process::Command`.

### R1 — Precedence

For the `stream_exec` spawn path, per-invocation env overrides MUST be applied **after** the Codex
wrapper’s internal environment injection (e.g., `CODEX_HOME`, `CODEX_BINARY`, default `RUST_LOG`)
so that the caller can override those keys when needed.

### R2 — API surface (normative)

The `codex` crate MUST provide an additive API usable by `agent_api` without changing existing
call sites that use `CodexClient::stream_exec`.

Minimum required capability (exact contract surface; name + signature are normative):

- A new `CodexClient` method:
  - `codex::CodexClient::stream_exec_with_env_overrides(request, env_overrides)`
  - Signature (normative):
    - `pub async fn stream_exec_with_env_overrides(
         &self,
         request: ExecStreamRequest,
         env_overrides: &std::collections::BTreeMap<String, String>,
       ) -> Result<ExecStream, ExecStreamError>;`

The new API MUST:

- apply the provided env map to the spawned process environment for that invocation only
- not persist env overrides across calls
- preserve existing `stream_exec` behavior when the env map is empty

Explicit v1 scope boundary (normative):
- This feature only requires the above API for `exec` streaming.
- A parallel per-invocation env override API for `codex resume` is explicitly out of scope for C0.

## Acceptance Criteria (observable)

- `codex` crate builds with no breaking changes to existing `ExecStreamRequest` call sites.
- The new per-invocation env override API exists and is documented in rustdoc at the public item.
- A code inspection (or a dedicated test in C2) can verify:
  - override application occurs after wrapper env injection
  - no parent env mutation occurs
