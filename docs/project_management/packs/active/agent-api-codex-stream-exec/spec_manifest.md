# Spec Manifest — Agent API Codex `stream_exec` parity

Status: Draft  
Date (UTC): 2026-02-20  
Feature directory: `docs/project_management/packs/active/agent-api-codex-stream-exec/`

## ADR Inputs

- ADR: `docs/adr/0011-agent-api-codex-stream-exec.md`

## Baselines (referenced; not duplicated)

- Universal Agent API (authoritative contracts/specs):
  - `docs/adr/0009-universal-agent-api.md`
  - `docs/project_management/next/universal-agent-api/contract.md`
  - `docs/project_management/next/universal-agent-api/run-protocol-spec.md`
  - `docs/project_management/next/universal-agent-api/event-envelope-schema-spec.md`
  - `docs/project_management/next/universal-agent-api/capabilities-schema-spec.md`
- Codex streaming + JSONL parsing (authoritative for Codex typed events + normalization):
  - `docs/adr/0005-codex-jsonl-log-parser-api.md`
  - `docs/specs/codex-thread-event-jsonl-parser-contract.md`
- Wrapper ingestion boundary (still authoritative; not replaced by this feature):
  - `docs/adr/0007-wrapper-events-ingestion-contract.md`
  - `docs/specs/wrapper-events-ingestion-contract.md`

## Required planning artifacts (always)

- `docs/project_management/packs/active/agent-api-codex-stream-exec/plan.md`
- `docs/project_management/packs/active/agent-api-codex-stream-exec/tasks.json`
- `docs/project_management/packs/active/agent-api-codex-stream-exec/session_log.md`

## Required cross-platform / decision-heavy artifacts

- `docs/project_management/packs/active/agent-api-codex-stream-exec/decision_register.md`
- `docs/project_management/packs/active/agent-api-codex-stream-exec/impact_map.md`
- `docs/project_management/packs/active/agent-api-codex-stream-exec/manual_testing_playbook.md`
- Smoke scripts:
  - `docs/project_management/packs/active/agent-api-codex-stream-exec/smoke/linux-smoke.sh`
  - `docs/project_management/packs/active/agent-api-codex-stream-exec/smoke/macos-smoke.sh`
  - `docs/project_management/packs/active/agent-api-codex-stream-exec/smoke/windows-smoke.ps1`

## Required before execution triads begin

- `docs/project_management/packs/active/agent-api-codex-stream-exec/quality_gate_report.md`
  - MUST contain `RECOMMENDATION: ACCEPT`

## Required spec documents (deterministic set)

### Slice specs (triads)

- `docs/project_management/packs/active/agent-api-codex-stream-exec/C0-spec.md` — Codex wrapper: add the minimum additive API required so `agent_api` can apply per-run environment overrides (while still using `CodexClient::stream_exec`).
- `docs/project_management/packs/active/agent-api-codex-stream-exec/C1-spec.md` — `agent_api` Codex backend: refactor to consume `codex::CodexClient::stream_exec` and map typed events to `AgentWrapperEvent` live (preserving DR-0012 finality).
- `docs/project_management/packs/active/agent-api-codex-stream-exec/C2-spec.md` — validation hardening: add fake-binary fixtures and integration tests proving (a) at least one event is emitted before completion resolves, (b) env precedence is preserved, and (c) redaction rules prevent raw JSONL lines from leaking through universal errors/events.

### Contract spec (user-facing)

Trigger: ADR changes user-facing library behavior/semantics (even if types are unchanged):
- how the `agent_api` Codex backend is spawned
- how request/config/env/timeout semantics are applied
- what error redaction guarantees exist
- whether/how `final_text` is populated.

- `docs/project_management/packs/active/agent-api-codex-stream-exec/contract.md` — authoritative contract for:
  - `agent_api` Codex backend runtime semantics (streaming, completion, redaction, bounds, absence)
  - any additive `crates/codex` public API required for per-run env overrides
  - the stable mapping between Codex typed events (`ThreadEvent`) and the universal envelope (`AgentWrapperEvent`).

### Adapter protocol spec (streaming + completion semantics)

Trigger: ADR changes the execution protocol that `agent_api` uses to stream Codex events (typed stream + completion), including failure/cancellation behavior.

- `docs/project_management/packs/active/agent-api-codex-stream-exec/codex-stream-exec-adapter-protocol-spec.md` — authoritative semantics for:
  - `codex::ExecStream` → `agent_api::AgentWrapperRunHandle` adaptation
  - ordering rules and “live” guarantees (what must happen before process exit)
  - error mapping/redaction rules (especially for `ExecStreamError` variants that include raw lines)
  - cancellation and backpressure behavior (consumer drops stream; bounded channels).

### Platform parity spec

Trigger: ADR affects cross-platform process I/O and must remain correct on GitHub-hosted runners.

- `docs/project_management/packs/active/agent-api-codex-stream-exec/platform-parity-spec.md` — authoritative guarantees/divergences for Linux/macOS/Windows for:
  - spawning + stdout streaming assumptions (CRLF, UTF-8, pipe behavior)
  - timeouts and cancellation behavior envelopes
  - required validation evidence (CI + manual).

## Coverage matrix (surface → authoritative doc)

| Surface | Owner doc |
|---|---|
| `agent_api` run handle finality (completion waits for stream termination) | `docs/project_management/next/universal-agent-api/run-protocol-spec.md` |
| `agent_api` event envelope bounds + raw-line prohibition | `docs/project_management/next/universal-agent-api/event-envelope-schema-spec.md` |
| Capability id naming/stability rules | `docs/project_management/next/universal-agent-api/capabilities-schema-spec.md` |
| Codex backend capability advertisement (`agent_api.events.live`) | `contract.md` |
| Codex typed event source-of-truth (what `ThreadEvent` means) | `docs/specs/codex-thread-event-jsonl-parser-contract.md` |
| Codex → universal event kind mapping rules | `contract.md` |
| `AgentWrapperRunRequest` absence semantics (unset timeout/working_dir/env/extensions) for Codex backend | `contract.md` |
| Core exec-policy key schema + defaults (`agent_api.exec.non_interactive`) | `docs/project_management/next/universal-agent-api/extensions-spec.md` |
| Codex exec-policy backend keys (`backend.codex.exec.*`) | `contract.md` |
| Config/env precedence rules (backend defaults vs per-run overrides) | `contract.md` |
| Per-run env override mechanism (how `AgentWrapperRunRequest.env` affects spawned process when using `CodexClient`) | `contract.md` |
| Streaming ordering + “live” guarantee definition for Codex backend | `codex-stream-exec-adapter-protocol-spec.md` |
| Error taxonomy and redaction rules for Codex streaming adaptation | `codex-stream-exec-adapter-protocol-spec.md` |
| Cancellation/backpressure behavior for Codex backend | `codex-stream-exec-adapter-protocol-spec.md` |
| Platform guarantees/divergences + required evidence | `platform-parity-spec.md` |
| Cross-platform CI workflow evidence for this feature | `platform-parity-spec.md` |
| Slice-specific deliverables/acceptance/out-of-scope | `C0-spec.md` / `C1-spec.md` |
| Validation hardening acceptance/out-of-scope | `C2-spec.md` |
| Architectural decisions (A/B pinned) | `decision_register.md` |
| Touch set + cascading implications + cross-queue conflicts | `impact_map.md` |
| Manual/non-gating validation steps | `manual_testing_playbook.md` |

## Determinism checklist (per spec)

### `contract.md`

- Pins the exact observable semantics for:
  - how the Codex process is spawned (via `CodexClient::stream_exec`)
  - how `timeout` and `working_dir` are derived/applied (including “absent” behavior)
  - how `env` maps onto the spawned process environment (including precedence rules)
  - whether/how `AgentWrapperCompletion.final_text` is populated for Codex runs
- Explicitly defines “absent” behavior for:
  - `request.timeout = None`
  - `request.working_dir = None`
  - `config.default_timeout = None`
  - `config.default_working_dir = None`
  - `config.codex_home = None`
  - empty `request.env`
  - `request.extensions`:
    - unknown keys fail-closed
    - supported keys have explicit defaults + validation rules
- Pins the exact error surface:
  - which failures return `AgentWrapperError::Backend`
  - what redacted message shapes are permitted (and what is forbidden).

### `codex-stream-exec-adapter-protocol-spec.md`

- Defines exact adaptation semantics:
  - mapping of `codex::ThreadEvent` values to `AgentWrapperEvent` (kind/channel/text/message rules)
  - how `codex::ExecStream.completion` maps to `AgentWrapperCompletion` (including exit status)
  - what happens on `ExecStreamError` from the Codex wrapper stream
- Defines redaction rules (normative):
  - emitted error messages MUST NOT include raw JSONL lines from `ExecStreamError::{Parse,Normalize}`
  - emitted error messages MUST be bounded per `event-envelope-schema-spec.md`
- Defines ordering/backpressure:
  - event ordering relative to upstream typed stream
  - bounded channel sizes and what “drain after receiver drop” means for Codex (to preserve DR-0012 semantics).

### `platform-parity-spec.md`

- Pins per-OS expectations for:
  - newline handling (LF/CRLF) and UTF-8 behavior
  - cancellation/kill behavior envelope
  - timeout behavior envelope
- Defines required evidence:
  - exact CI commands/jobs that must pass without a real Codex binary
  - what is manual-only and why.

### `decision_register.md`

Must include, at minimum, A/B selections for:
- Whether Codex backend populates `AgentWrapperCompletion.final_text` from Codex `ExecCompletion.last_message` (and the exact determinism rule if yes).
- The redaction strategy for `ExecStreamError` variants that embed raw JSONL lines.
- The per-run env override strategy required to keep `AgentWrapperRunRequest.env` semantics intact while using `CodexClient`.

### `C2-spec.md`

- Pins the exact fake-binary / fixture strategy (cross-platform) used to validate streaming behavior without a real Codex install.
- Defines acceptance criteria for:
  - “live” evidence (at least one event emitted before completion resolves)
  - env precedence evidence (request env overrides backend env keys)
  - redaction evidence (no raw JSONL line leakage into universal errors/events).
