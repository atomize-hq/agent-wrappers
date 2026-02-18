# Impact Map — Claude Code live stream-json

Status: Draft  
Date (UTC): 2026-02-17  
Feature directory: `docs/project_management/next/claude-code-live-stream-json/`

## Inputs

- ADR(s):
  - `docs/adr/0010-claude-code-live-stream-json.md`
- Spec manifest:
  - `docs/project_management/next/claude-code-live-stream-json/spec_manifest.md`

## Touch Set (explicit)

### Create (new files / new surfaces)

- Planning pack (docs-only; derived from ADR + `spec_manifest.md`):
  - `docs/project_management/next/claude-code-live-stream-json/README.md`
  - `docs/project_management/next/claude-code-live-stream-json/plan.md`
  - `docs/project_management/next/claude-code-live-stream-json/tasks.json`
  - `docs/project_management/next/claude-code-live-stream-json/session_log.md`
  - `docs/project_management/next/claude-code-live-stream-json/ci_checkpoint_plan.md`
  - `docs/project_management/next/claude-code-live-stream-json/manual_testing_playbook.md`
  - `docs/project_management/next/claude-code-live-stream-json/smoke/linux-smoke.sh`
  - `docs/project_management/next/claude-code-live-stream-json/smoke/macos-smoke.sh`
  - `docs/project_management/next/claude-code-live-stream-json/smoke/windows-smoke.ps1`
  - `docs/project_management/next/claude-code-live-stream-json/C0-spec.md`
  - `docs/project_management/next/claude-code-live-stream-json/C1-spec.md`
  - `docs/project_management/next/claude-code-live-stream-json/kickoff_prompts/C0-code.md`
  - `docs/project_management/next/claude-code-live-stream-json/kickoff_prompts/C0-test.md`
  - `docs/project_management/next/claude-code-live-stream-json/kickoff_prompts/C0-integ.md`
  - `docs/project_management/next/claude-code-live-stream-json/kickoff_prompts/C1-code.md`
  - `docs/project_management/next/claude-code-live-stream-json/kickoff_prompts/C1-test.md`
  - `docs/project_management/next/claude-code-live-stream-json/kickoff_prompts/C1-integ.md`
- Contract/spec docs (authoritative; per `spec_manifest.md`):
  - `docs/project_management/next/claude-code-live-stream-json/contract.md`
  - `docs/project_management/next/claude-code-live-stream-json/stream-json-print-protocol-spec.md`
  - `docs/project_management/next/claude-code-live-stream-json/platform-parity-spec.md`
- CI wiring (per `ci_checkpoint_plan.md` decision):
  - Either a new workflow: `.github/workflows/claude-code-live-stream-json-smoke.yml`, or
  - a small update to reuse/extend an existing workflow (see DR-0005).

### Edit (existing files / existing surfaces)

- Claude wrapper crate (`crates/claude_code`) public API expansion:
  - `crates/claude_code/Cargo.toml` (add any new deps required by the streaming API surface)
  - `crates/claude_code/src/lib.rs` (export new streaming handle/types)
  - `crates/claude_code/src/client/mod.rs` (add `ClaudeClient::print_stream_json(...)`)
  - `crates/claude_code/src/process.rs` (new streaming spawn/run helper or shared process I/O utilities)
  - `crates/claude_code/src/error.rs` (if new error variants are required for streaming I/O/cancel)
  - `crates/claude_code/src/stream_json.rs` (parser remains the source of truth; edits only if streaming reveals missing invariants)
- Universal API crate (`crates/agent_api`) Claude backend behavior change:
  - `crates/agent_api/src/backends/claude_code.rs` (switch to streaming API; emit events live; advertise `agent_api.events.live`)
  - `crates/agent_api/src/run_handle_gate.rs` (no semantic change expected; verify integration remains DR-0012 compliant)
  - `crates/agent_api/tests/c2_claude_event_mapping.rs` (if assumptions about buffering are encoded in tests)
  - `crates/agent_api/src/backends/mod.rs` (only if module wiring changes)
- Repo docs + sequencing:
  - `docs/adr/0010-claude-code-live-stream-json.md` (keep `Related Docs` and `ADR_BODY_SHA256` current)
  - `docs/project_management/next/sequencing.json` (add a new feature track and phase dependencies)
- Workspace artifacts:
  - `Cargo.lock` (dependency graph will change once streaming API deps land)

### Possible edits (dependent on pinned spec choices)

- Universal Agent API planning pack docs may become stale once Claude advertises live events:
  - `docs/project_management/next/universal-agent-api/plan.md` (Claude described as buffered today)
  - `docs/project_management/next/universal-agent-api/C2-spec.md` (capability expectations and narrative)
  - `docs/project_management/next/universal-agent-api/contract.md` (only if it currently states Claude must not be live; otherwise no change)
- CI workflow reuse vs duplication:
  - `.github/workflows/universal-agent-api-smoke.yml` (if reused or extended for this feature’s checkpoint)
  - `.github/workflows/ci.yml` (if adding a dedicated multi-OS gate instead of a feature-local smoke workflow)

## Cascading implications (behavior/UX) + contradiction risks

### 1) `claude_code` shifts from “buffered output only” to “streaming available”

- Direct impact:
  - A new streaming API will let consumers observe `ClaudeStreamJsonEvent` values before the `claude` process exits.
- Second-order impact:
  - Consumers must be prepared to handle a stream item type that can represent parse errors in-order (`Result<..., ClaudeStreamJsonParseError>`).
- Contradiction risks:
  - If streaming continues to buffer stderr (or fails to drain it), the process can deadlock. The protocol spec MUST pin “stderr is drained (discarded or mirrored) but not retained” to match DR-0004.

### 2) `agent_api` Claude backend becomes “live” (`agent_api.events.live`)

- Direct impact:
  - `AgentWrapperCapabilities` for Claude will include `agent_api.events.live`, changing consumer behavior for UIs that gate on this capability.
  - Consumers will observe events earlier in the run lifecycle (before process exit) rather than post-hoc.
- Second-order impact:
  - Completion safety semantics remain mandatory (DR-0012): `completion` MUST still wait for stream finality (or stream drop).
- Contradiction risks:
  - Any docs/tests that currently assume “Claude is buffered” will drift and must be updated as part of the integration reconciliation.

### 3) Backpressure and buffering behavior becomes observable

- Direct impact:
  - Streaming introduces explicit buffering choices:
    - bounded channel sizes in `agent_api` adapters,
    - whether per-line parse errors become events,
    - what happens when the consumer is slow.
- Contradiction risks:
  - If the implementation silently drops events under backpressure, it conflicts with the universal run-protocol expectations. The protocol spec MUST define bounded buffering and behavior under consumer slowness deterministically.

### 4) Platform parity becomes a real contract surface (stdout framing)

- Direct impact:
  - Windows CRLF vs LF framing must be handled without corrupting JSON parsing (already partially addressed by the parser stripping `\r`).
- Contradiction risks:
  - If the streaming reader splits lines incorrectly (multi-byte UTF-8, long lines, partial reads), the observable event stream will differ by OS. The parity spec + tests must pin and validate the framing algorithm.

## Cross-queue scan (ADRs + Planning Packs)

### ADRs in this repo (`docs/adr/*`)

- `docs/adr/0008-claude-stream-json-parser-api.md`
  - Overlap: this feature depends on the typed `ClaudeStreamJsonParser`/`ClaudeStreamJsonEvent` model and extends usage from “parse captured logs” to “parse live stdout”.
  - Conflict: none if the parser remains the source of truth and streaming does not introduce a second, divergent parser.
- `docs/adr/0009-universal-agent-api.md`
  - Overlap: `agent_api` Claude backend behavior and capability advertisement.
  - Conflict risk: documentation drift if `universal-agent-api` planning pack asserts Claude is buffered.
  - Resolution: update the universal-agent-api planning pack docs/tests as needed during integration reconciliation, or explicitly annotate “superseded by ADR-0010” where appropriate.
- `docs/adr/0006-agent-wrappers-workspace.md`
  - Overlap: Claude wrapper crate remains a stable library surface; this feature adds a new headless capability rather than changing workspace shape.
  - Conflict: none.

Note: this repo does not currently store ADR drafts/queued items under `docs/project_management/adrs/{draft,queued}/`; the active ADR set is `docs/adr/*`.

### Planning Packs (`docs/project_management/next/*`)

- `docs/project_management/next/universal-agent-api/`
  - Overlap: `crates/agent_api/src/backends/claude_code.rs` and capability semantics (`agent_api.events.live`).
  - Conflict risk: tests/docs that currently assert Claude is not live will fail or become incorrect.
  - Resolution: treat this feature as an incremental follow-on to universal-agent-api; reconcile and keep the universal pack accurate.
- `docs/project_management/next/claude-code-cli-parity-2.1.29/`
  - Overlap: touches `crates/claude_code` and may land concurrently; likely merge conflicts in `crates/claude_code/src/client/mod.rs` or process helpers.
  - Resolution: sequence merges to avoid long-lived divergence; prefer rebasing parity work onto the streaming API changes once this feature lands.
- Other `codex-*` planning packs
  - Overlap: none meaningful (Codex parity and Codex JSONL parsing are distinct surfaces).

## Follow-ups / Required pinning (before execution)

Pinned decisions (see `decision_register.md`):
- DR-0001: implement streaming in `crates/claude_code` (selected).
- DR-0002/DR-0003: per-line parse errors are redacted and emitted in-order on the stream (selected).
- DR-0004: completion yields `ExitStatus` only (no stdout/stderr buffering) (selected).

Remaining “pin or decide” items discovered while mapping impacts:
1) CI checkpoint workflow strategy for this feature (new workflow vs reuse an existing one) MUST be pinned (see DR-0005).
2) Backpressure behavior for live streaming MUST be deterministic (selected: apply backpressure; see DR-0009).
3) Ensure `agent_api` capability ids are pinned deterministically (selected: `agent_api.events.live` only; see DR-0006).
