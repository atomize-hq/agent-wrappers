# ADR-0011 — Agent API Codex backend uses `codex::CodexClient::stream_exec` (streaming parity)
#
# Note: Run `make adr-fix ADR=docs/adr/0011-agent-api-codex-stream-exec.md` after editing to update
# the ADR_BODY_SHA256 drift guard.

## Status
- Status: Draft
- Date (UTC): 2026-02-20
- Owner(s): spensermcconnell

## Scope
- Feature directory: `docs/project_management/packs/active/agent-api-codex-stream-exec/`
- Intended branch: `feat/agent-api-codex-stream-exec`
- Standards:
  - `/Users/spensermcconnell/__Active_Code/atomize-hq/substrate/docs/project_management/standards/ADR_STANDARD_AND_TEMPLATE.md`

## Related Docs
- Planning pack (to be created for execution readiness):
  - Spec manifest: `docs/project_management/packs/active/agent-api-codex-stream-exec/spec_manifest.md`
  - Plan: `docs/project_management/packs/active/agent-api-codex-stream-exec/plan.md`
  - Tasks: `docs/project_management/packs/active/agent-api-codex-stream-exec/tasks.json`
  - Slice specs:
    - `docs/project_management/packs/active/agent-api-codex-stream-exec/C0-spec.md`
    - `docs/project_management/packs/active/agent-api-codex-stream-exec/C1-spec.md`
    - `docs/project_management/packs/active/agent-api-codex-stream-exec/C2-spec.md`
  - Contract: `docs/project_management/packs/active/agent-api-codex-stream-exec/contract.md`
  - Adapter protocol: `docs/project_management/packs/active/agent-api-codex-stream-exec/codex-stream-exec-adapter-protocol-spec.md`
  - Platform parity: `docs/project_management/packs/active/agent-api-codex-stream-exec/platform-parity-spec.md`
  - Decision Register: `docs/project_management/packs/active/agent-api-codex-stream-exec/decision_register.md`
  - Impact Map: `docs/project_management/packs/active/agent-api-codex-stream-exec/impact_map.md`
  - Manual Playbook: `docs/project_management/packs/active/agent-api-codex-stream-exec/manual_testing_playbook.md`
  - Smoke scripts:
    - `docs/project_management/packs/active/agent-api-codex-stream-exec/smoke/linux-smoke.sh`
    - `docs/project_management/packs/active/agent-api-codex-stream-exec/smoke/macos-smoke.sh`
    - `docs/project_management/packs/active/agent-api-codex-stream-exec/smoke/windows-smoke.ps1`
  - Quality gate report: `docs/project_management/packs/active/agent-api-codex-stream-exec/quality_gate_report.md`
- Baseline universal contract:
  - `docs/adr/0009-universal-agent-api.md`
  - `docs/project_management/next/universal-agent-api/contract.md`
  - `docs/project_management/next/universal-agent-api/run-protocol-spec.md`
  - `docs/project_management/next/universal-agent-api/event-envelope-schema-spec.md`
  - `docs/project_management/next/universal-agent-api/capabilities-schema-spec.md`
  - `docs/project_management/next/universal-agent-api/extensions-spec.md`
- Codex JSONL parsing contract (offline parsing; reused by streaming):
  - `docs/adr/0005-codex-jsonl-log-parser-api.md`
  - `docs/specs/codex-thread-event-jsonl-parser-contract.md`
- Wrapper ingestion boundary (not replaced):
  - `docs/adr/0007-wrapper-events-ingestion-contract.md`
  - `docs/specs/wrapper-events-ingestion-contract.md`
- Claude streaming parity context:
  - `docs/adr/0010-claude-code-live-stream-json.md`
- Current Codex universal backend implementation (to be refactored):
  - `crates/agent_api/src/backends/codex.rs#L221`
- Codex wrapper streaming API (to be used by `agent_api`):
  - `crates/codex/src/exec.rs#L63`

## Executive Summary (Operator)

ADR_BODY_SHA256: ba95012b0762bc039fdb1dcd9cd1fe0830c7230c5abe1dc4c6dd8db0d59ab15f

### Changes (operator-facing)
- Codex backend streaming parity in `agent_api`
  - Existing: `agent_api`’s Codex backend spawns `codex exec --json` directly and performs its own
    stdout ingestion loop using `tokio` + `BufReader(...).lines()` and `codex::JsonlThreadEventParser`.
  - New: `agent_api`’s Codex backend uses the Codex wrapper crate’s streaming API
    (`codex::CodexClient::stream_exec`) to obtain a typed event stream (`ThreadEvent`) plus a
    completion future, and then maps those typed events into the universal event envelope
    (`AgentWrapperEvent`) live.
  - Why: Ensure Codex and Claude backends share the same architectural “wrapper streaming handle”
    shape (typed events + completion), reduce duplicated spawn/IO logic in `agent_api`, and make
    onboarding future CLI agents mechanical: wrapper crate exposes a streaming handle; `agent_api`
    maps typed events into the universal envelope.
  - Links:
    - Current manual spawn + ingestion: `crates/agent_api/src/backends/codex.rs#L221`
    - Codex wrapper streaming API: `crates/codex/src/exec.rs#L63`
    - Claude wrapper streaming API (parity reference): `crates/claude_code/src/client/mod.rs#L156`
    - Universal run/stream finality rule (DR-0012): `docs/project_management/next/universal-agent-api/run-protocol-spec.md`

## Problem / Context

- The repo now has two “streaming” backends under `agent_api` (`codex` and `claude_code`), but they
  do not share the same implementation shape:
  - Claude: wrapper crate exposes a streaming handle (typed events stream + completion future) and
    `agent_api` forwards mapped events live.
  - Codex: `agent_api` spawns the CLI directly and reimplements ingestion/spawn concerns locally.
- This shape mismatch undermines the “orthogonal wrapper onboarding” goal:
  - Adding a future agent is slower if `agent_api` must reimplement process management and
    per-line parsing logic per backend rather than consuming a wrapper-provided streaming handle.
- The Codex wrapper crate already defines the intended streaming surface (`CodexClient::stream_exec`)
  that produces typed events and a completion future, but `agent_api` is not using it.

## Goals

- Refactor `agent_api`’s Codex backend implementation to use `codex::CodexClient::stream_exec` as
  the sole source of live typed events and completion status.
- Preserve the universal run contract invariants:
  - capability gating (`agent_api.*` ids)
  - event bounds enforcement (`event-envelope-schema-spec.md`)
  - completion gating / finality semantics (DR-0012).
- Preserve config precedence guarantees from ADR 0009:
  - backend config provides defaults
  - per-run request overrides defaults
  - request env overrides backend env keys.
- Keep safety posture unchanged:
  - `agent_api` MUST NOT emit raw backend line content in v1 (including through error strings).

## Non-Goals

- Replacing `wrapper_events` with `agent_api` (ADR 0007 remains authoritative for ingestion).
- Forcing identical tool payload schemas across agents (Codex vs Claude vs future agents).
- Introducing new universal event kinds beyond the existing envelope.
- Adding interactive/TUI support for Codex or Claude CLIs.

## User Contract (Authoritative)

This ADR changes implementation shape but preserves the public Rust contract of `agent_api`.

### Rust API (`agent_api`)
- `agent_api::backends::codex::CodexBackend` continues to:
  - register under `AgentWrapperKind("codex")`
  - advertise:
    - `agent_api.run`
    - `agent_api.events`
    - `agent_api.events.live`
    - `backend.codex.exec_stream`
    - `agent_api.exec.non_interactive`
    - `backend.codex.exec.sandbox_mode`
    - `backend.codex.exec.approval_policy`
- `AgentWrapperRunRequest` fields retain semantics:
  - `prompt`: required (non-empty)
  - `working_dir` and `timeout`: best-effort forwarded
  - `env`: applied only to the spawned backend process; MUST NOT mutate parent process env
  - `extensions`: fail-closed for unknown keys per `capabilities-schema-spec.md`.

### Error taxonomy (library contract)
- `AgentWrapperError` remains authoritative for `agent_api` consumers:
  - failures to start/stream MUST surface as `AgentWrapperError::Backend { message }` with
    redacted, bounded error messages.
- Redaction requirement (normative):
  - When the Codex wrapper reports parsing/normalization errors, `agent_api` MUST NOT embed the raw
    JSONL line in `AgentWrapperEvent.message` or `AgentWrapperError::Backend.message`.

### Config (`agent_api::backends::codex::CodexBackendConfig`)
- No new config files are introduced by this ADR.
- Existing config fields retain meaning:
  - `binary`: Codex CLI binary path
  - `codex_home`: optional `CODEX_HOME` override
  - `default_timeout`, `default_working_dir`, `env`.

### Platform guarantees
- Linux/macOS/Windows MUST be supported (GitHub-hosted runner parity is required; see ADR 0009’s
  platform spec set under `docs/project_management/next/universal-agent-api/`).
- Tests MUST NOT require a real Codex binary on CI runners (fixture/fake-binary strategy only).

## Architecture Shape

### Components
- `crates/agent_api`:
  - Update `backends::codex` to:
    - build a `codex::CodexClient` configured from backend config + per-run request
    - call `CodexClient::stream_exec` to obtain:
      - a typed `ThreadEvent` stream
      - a completion future (exit status + optional last message path/value)
    - map typed events into `AgentWrapperEvent` as events arrive
    - enforce bounds and redaction before emitting any universal event
    - preserve DR-0012 completion gating semantics (completion resolves only once the universal
      stream is final).
- `crates/codex` (required enabling work):
  - Provide an API to apply per-process environment overrides (so `agent_api` can respect
    `AgentWrapperRunRequest.env` while still using `CodexClient`).
  - This MUST be additive (no breaking changes to existing public structs).

### End-to-end flow
- Inputs:
  - `AgentWrapperRunRequest` (prompt, working_dir, timeout, env, extensions)
  - `CodexBackendConfig` (binary, codex_home, defaults)
- Derived state:
  - merged working_dir/timeout/env (request overrides config)
  - a per-run `CodexClient` configured for JSON streaming
- Actions:
  - spawn `codex exec --json ...` via the Codex wrapper crate
  - stream typed `ThreadEvent` values
  - map → bound → emit `AgentWrapperEvent`s
  - await wrapper completion; resolve universal completion only after event stream termination
- Outputs:
  - `AgentWrapperRunHandle` with `events` + gated `completion`.

## Dependencies

Prerequisites:
  - `docs/adr/0009-universal-agent-api.md` implemented (baseline universal contract exists).
  - `crates/codex` streaming API exists (`CodexClient::stream_exec`).
Integration dependency notes:
  - If `crates/codex` requires additive API to support per-run env overrides, that change MUST land
    before `agent_api` can fully switch to `CodexClient::stream_exec` without regressing the
    universal contract.

## Security / Safety Posture

- Fail-closed rules:
  - If `extensions` contains any key not present in backend capabilities, the run MUST fail before
    spawning any backend process.
  - If the Codex process cannot be spawned, the run MUST fail with `AgentWrapperError::Backend`
    (no panics).
- Redaction invariants (v1):
  - `agent_api` MUST NOT emit raw backend lines in events or errors.
  - `agent_api` MUST NOT use `ExecStreamError::to_string()` as an emitted message when it may
    contain the raw JSONL line; emitted messages MUST be redacted summaries.
- Observability:
  - All events MUST carry `agent_kind = "codex"`.

## Validation Plan (Authoritative)

### Tests
- Unit tests (`crates/agent_api`):
  - Codex backend continues to advertise `agent_api.events.live`.
  - Streaming error redaction: ensure no raw JSONL lines appear in `AgentWrapperEvent.message`.
- Integration tests (`crates/agent_api`):
  - Fake-binary-based test that proves at least one event is emitted before completion resolves.
  - Env precedence test that proves request env overrides backend env keys for the spawned process.

### Manual validation
- Manual playbook lives in the feature planning pack:
  - `docs/project_management/packs/active/agent-api-codex-stream-exec/manual_testing_playbook.md`

### Smoke scripts
- Smoke scripts live in the feature planning pack:
  - `docs/project_management/packs/active/agent-api-codex-stream-exec/smoke/`

## Rollout / Backwards Compatibility
- Policy: greenfield breaking is allowed for implementation details, but `agent_api` public API
  remains stable for downstream consumers.
- Compat work: none planned (behavioral parity is expected; any behavior deltas must be explicitly
  documented in this ADR before acceptance).

## Decision Summary
- This ADR records a single mandated architectural direction: the universal Codex backend MUST
  consume the Codex wrapper crate’s streaming surface (`CodexClient::stream_exec`) rather than
  reimplement process + ingestion logic directly in `agent_api`.
- Execution-readiness A/B decisions (exec policy, redaction mapping, env override strategy, and
  `final_text` policy) are tracked in the feature pack’s decision register:
  - `docs/project_management/packs/active/agent-api-codex-stream-exec/decision_register.md`
