# Charter — Onboarding new CLI agent wrapper crates + `agent_api` backends

Status: Draft  
Date (UTC): 2026-02-20  
Owner(s): atomize-hq wrappers team

This charter defines the canonical rules for onboarding new CLI agent support in this repo.

It is designed to keep the system **orthogonal**:
- wrapper crates can evolve independently, and
- the universal facade (`agent_api`) can onboard new backends mechanically with minimal drift.

## Goals

- Make adding “CLI Agent X” a deterministic process:
  - predictable contract surfaces
  - predictable validation and fake-binary evidence
  - predictable capability + extension declaration rules
- Keep the universal event envelope small and stable while allowing backend-specific expansion.
- Prevent cross-document contradictions by having exactly one owner doc per contract surface.

## Non-Goals

- Forcing semantic parity across all agents (capabilities differ; the API must represent that).
- Turning planning docs into a CI gate (planning artifacts are for humans and execution triads).

## Canonical architecture layers

### 1) Wrapper crates (per CLI agent)

Example: `crates/codex/`, `crates/claude_code/`.

A wrapper crate SHOULD provide:
- A deterministic spawn surface (builder + request types).
- A typed streaming surface:
  - `events: Stream<Item = Result<TypedEvent, ParseError>>`
  - `completion: Future<Output = Result<Completion, Error>>`
- A pure parsing API for offline JSONL/stream parsing (testable without spawning).
- A fake-binary or fixture strategy for cross-platform tests when the real CLI is unavailable.

A wrapper crate MUST:
- avoid leaking secrets by default (no raw-line echoing as “errors” in library APIs unless explicitly opted in),
- support deterministic disabling of interactive prompting when the upstream CLI supports it,
- document its CLI parity surface (what flags and flows are covered by the wrapper).

### 2) Universal facade (`crates/agent_api/`)

`agent_api` is the stable, agent-agnostic surface:
- `AgentWrapperRunRequest` (prompt/working_dir/timeout/env/extensions)
- `AgentWrapperEvent` (stable envelope + optional JSON `data`)
- `AgentWrapperCompletion` (exit status + optional `final_text`)
- capability gating for optional features

The universal API MUST:
- be safe by default:
  - bounded payloads
  - redacted error messages
  - no raw backend line leakage in v1
- preserve protocol invariants:
  - “completion finality” (DR-0012): completion resolves only once the event stream is final/dropped
- keep backend types out of the public API (guarded in CI).

## Capabilities + extensions rules (canonical)

### Capability ids

Rules are owned by:
- `docs/project_management/next/universal-agent-api/capabilities-schema-spec.md`

### Extension keys

Core extension key registry + ownership rules are owned by:
- `docs/project_management/next/universal-agent-api/extensions-spec.md`

Required invariants:
- Every supported extension key MUST be advertised in `AgentWrapperCapabilities.ids`.
- Backends MUST fail-closed on unknown extension keys before spawning.
- Every extension key MUST have exactly one authoritative owner document:
  - `agent_api.*` keys are owned by the universal registry
  - `backend.<agent_kind>.*` keys are owned by the backend’s contract/spec docs

## Streaming event mapping rubric (recommended buckets)

To keep onboarding orthogonal, treat backend output as mapping into these buckets:

- **TextOutput**: assistant text (snapshots and deltas)
- **ToolCall**: tool use intent/start (command execution, file ops, MCP tool call, web search, etc.)
- **ToolResult**: tool result/finish (where a backend provides a stable “result” event)
- **Status**: lifecycle markers (thread/turn start/complete, progress)
- **Error**: redacted backend errors (transport, parse, normalize, tool failures)
- **Unknown**: parseable but unmapped events (safe placeholder)

Rules of thumb:
- Prefer “best-effort parity” rather than forcing identical payload schemas.
- Use `data` only for stable, bounded, redacted payloads; never for raw backend lines.

## Non-interactive + sandbox posture (canonical)

Backends should be automation-safe by default, and hosts should be able to override explicitly per run.

Core key:
- `agent_api.exec.non_interactive` (owned by `extensions-spec.md`)

Backend-specific exec-policy knobs (pattern):
- `backend.<agent_kind>.exec.*` keys (owned by backend contract/spec docs)

## Onboarding checklist (new CLI agent)

1) Create wrapper crate `crates/<agent>/`:
   - builder + request types
   - streaming typed events + completion
   - offline parser API
   - fixtures/fake binary strategy
2) Add wrapper coverage manifest (or equivalent) proving which CLI flags/flows are supported.
3) Add `agent_api` backend adapter:
   - map typed events → universal envelope
   - enforce redaction + bounds
   - preserve completion gating (DR-0012)
   - advertise capabilities + extension keys
4) Add C2-style tests in `agent_api` that do not require a real CLI:
   - “live event before completion”
   - redaction (no raw line leakage)
   - exec-policy default behavior (non-interactive) and override levers if applicable
5) Ensure required CI workflows pass (see below).

## CI expectations (must stay green)

The following workflows are expected to remain green for onboarding work:
- `.github/workflows/ci.yml`
  - `cargo test --workspace --all-targets`
  - `cargo test -p agent_api --all-features`
  - public API type leak guard for `agent_api`
- Smoke workflows for feature packs (when present), e.g.:
  - `.github/workflows/universal-agent-api-smoke.yml`
  - `.github/workflows/claude-code-live-stream-json-smoke.yml`

