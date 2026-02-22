# Impact Map — Agent API Codex `stream_exec` parity

Status: Draft  
Date (UTC): 2026-02-20  
Feature directory: `docs/project_management/packs/active/agent-api-codex-stream-exec/`

## Inputs

- ADR: `docs/adr/0011-agent-api-codex-stream-exec.md`
- Spec manifest: `docs/project_management/packs/active/agent-api-codex-stream-exec/spec_manifest.md`
- Baselines (referenced; not duplicated):
  - `docs/adr/0009-universal-agent-api.md`
  - `docs/project_management/next/universal-agent-api/contract.md`
  - `docs/project_management/next/universal-agent-api/run-protocol-spec.md`
  - `docs/project_management/next/universal-agent-api/event-envelope-schema-spec.md`
  - `docs/project_management/next/universal-agent-api/capabilities-schema-spec.md`
  - `docs/specs/codex-thread-event-jsonl-parser-contract.md`

## Touch set (explicit)

### Create
- `docs/project_management/packs/active/agent-api-codex-stream-exec/impact_map.md`
- `docs/project_management/packs/active/agent-api-codex-stream-exec/plan.md`
- `docs/project_management/packs/active/agent-api-codex-stream-exec/tasks.json`
- `docs/project_management/packs/active/agent-api-codex-stream-exec/session_log.md`
- `docs/project_management/packs/active/agent-api-codex-stream-exec/decision_register.md`
- `docs/project_management/packs/active/agent-api-codex-stream-exec/manual_testing_playbook.md`
- `docs/project_management/packs/active/agent-api-codex-stream-exec/contract.md`
- `docs/project_management/packs/active/agent-api-codex-stream-exec/codex-stream-exec-adapter-protocol-spec.md`
- `docs/project_management/packs/active/agent-api-codex-stream-exec/platform-parity-spec.md`
- `docs/project_management/packs/active/agent-api-codex-stream-exec/C0-spec.md`
- `docs/project_management/packs/active/agent-api-codex-stream-exec/C1-spec.md`
- `docs/project_management/packs/active/agent-api-codex-stream-exec/C2-spec.md`
- `docs/project_management/packs/active/agent-api-codex-stream-exec/kickoff_prompts/`
- `docs/project_management/packs/active/agent-api-codex-stream-exec/smoke/`
- `docs/project_management/packs/active/agent-api-codex-stream-exec/smoke/linux-smoke.sh`
- `docs/project_management/packs/active/agent-api-codex-stream-exec/smoke/macos-smoke.sh`
- `docs/project_management/packs/active/agent-api-codex-stream-exec/smoke/windows-smoke.ps1`
- `.github/workflows/agent-api-codex-stream-exec-smoke.yml`
- `crates/agent_api/src/bin/fake_codex_stream_json_agent_api.rs`
- `crates/agent_api/tests/c2_codex_stream_exec_parity.rs`

### Edit
- `docs/adr/0011-agent-api-codex-stream-exec.md`
- `.github/workflows/ci.yml`
- `crates/agent_api/Cargo.toml`
- `crates/agent_api/src/backends/codex.rs`
- `crates/agent_api/src/backends/claude_code.rs`
- `crates/agent_api/src/bin/fake_claude_stream_json_agent_api.rs`
- `crates/codex/src/builder/mod.rs`
- `crates/codex/src/exec/streaming.rs`
- `crates/codex/src/home.rs`
- `crates/codex/src/lib.rs`

### Deprecate
- None

### Delete
- None

## Cascading implications (behavior/UX) + contradiction risks

### 1) Spawn semantics drift (Codex backend)

- Direct impact:
  - `agent_api` Codex backend will no longer be the “source of truth” for `codex exec` argv,
    process I/O wiring, stderr handling, or timeout handling; those will come from `crates/codex`.
- Second-order impact:
  - Any behavior differences between the current `agent_api` spawn path and `CodexClient::stream_exec`
    must be made explicit in `contract.md` (defaults, stderr behavior, and working directory rules).
- Contradiction risks:
  - Current `agent_api` Codex backend drops stderr (`Stdio::null()`), while `crates/codex` captures
    stderr and may include it in errors; if surfaced unredacted, this can violate the repo’s
    “safe-by-default” posture in the universal API.

### 2) Environment precedence and “per-run env” support

- Direct impact:
  - `AgentWrapperRunRequest.env` MUST still apply to the spawned `codex` process even when the run
    is executed through `CodexClient::stream_exec`.
- Second-order impact:
  - `crates/codex` currently owns process environment setup (`CODEX_HOME`, `CODEX_BINARY`,
    default `RUST_LOG=error` when unset). Adding per-run env overrides must not introduce
    accidental parent-process mutation or cross-run leakage.
- Contradiction risks:
  - `agent_api` currently supports per-run env overrides by directly calling `Command::env`.
    A refactor that loses `request.env` behavior would silently violate ADR 0009 expectations and
    break future multi-agent onboarding assumptions.

### 3) Exec policy defaults + per-run overrides (non-interactive + sandbox)

- Direct impact:
  - The universal backend must be automation-safe by default (no interactive prompts/hangs) while
    still allowing hosts to select a different execution posture per run (e.g., Substrate wanting
    `danger-full-access` because it enforces sandboxing externally).
- Second-order impact:
  - Exec policy MUST be represented as explicit, validated extension inputs (not implied by
    downstream CLI defaults), and mapped deterministically into:
    - Codex: `--ask-for-approval` + `--sandbox`
    - Claude: `--permission-mode bypassPermissions` (for non-interactive default)
- Contradiction risks:
  - If non-interactive defaults are not pinned, the CLI can prompt and hang CI/services.
  - If sandbox defaults are not pinned, different host environments can observe different behavior
    without any changes to the universal API request.

### 4) Error redaction and raw-line leakage

- Direct impact:
  - `codex::ExecStreamError::{Parse, Normalize}` embed the raw JSONL line in their error display
    (and the type carries `line`), which is forbidden to emit in `agent_api` v1 error messages.
- Second-order impact:
  - The Codex backend adaptation layer must introduce a deterministic redaction mapping for all
    upstream error types and MUST NOT use `to_string()` as an operator-facing message when it can
    include raw lines.
- Contradiction risks:
  - If the new adapter forwards `ExecStreamError` text directly into `AgentWrapperEvent.message` or
    `AgentWrapperError::Backend.message`, the universal API will regress its safety guarantees and
    diverge from Claude’s redaction posture.

### 5) Completion semantics and “final_text”

- Direct impact:
  - `crates/codex` streaming completion (`ExecCompletion`) may include `last_message`, which is a
    candidate for `AgentWrapperCompletion.final_text` population.
- Second-order impact:
  - Whether `final_text` is populated must be pinned as a deterministic contract (or explicitly
    disallowed) and made consistent across backends (Codex vs Claude) to avoid consumer confusion.
- Contradiction risks:
  - If Codex populates `final_text` but Claude does not (or vice versa) without explicit capability
    signaling, consumers will infer a false universal guarantee.

### 6) Backpressure, channel bounds, and deadlock avoidance

- Direct impact:
  - `agent_api` currently uses bounded `mpsc` channels and explicitly drains Claude’s backend stream
    even after the consumer drops the universal event stream.
- Second-order impact:
  - The Codex backend adaptation must maintain the DR-0012 invariant (completion resolves only
    after the event stream is final) without introducing deadlocks when the consumer is slow or
    drops the stream.
- Contradiction risks:
  - The Codex wrapper streaming path uses its own channel and tasks; if `agent_api` stops polling
    the upstream stream after its downstream receiver drops, the codex wrapper may block and/or
    the child may be cancelled unexpectedly.

## Cross-queue scan (ADRs + Planning Packs)

### ADRs (`docs/adr/*.md`)

- `docs/adr/0009-universal-agent-api.md`
  - Overlap: run protocol + event envelope + capability model.
  - Conflict risk: none if this feature remains an implementation refactor and does not change the
    universal envelope or capability naming.
- `docs/adr/0005-codex-jsonl-log-parser-api.md`
  - Overlap: Codex typed event normalization semantics and parser contracts.
  - Conflict risk: if `agent_api` attempts to reinterpret Codex events beyond the Codex contracts,
    or leaks raw JSONL lines via `ExecStreamError`.
- `docs/adr/0007-wrapper-events-ingestion-contract.md`
  - Overlap: repo-wide ingestion safety posture and bounded parsing patterns.
  - Conflict risk: the Codex wrapper uses `BufReader(...).lines()` for streaming ingestion, which is
    not bounded in the same way as `wrapper_events`; if the universal API needs hard bounds before
    mapping, this should be tracked as a follow-up decision (not silently assumed).
- `docs/adr/0010-claude-code-live-stream-json.md`
  - Overlap: “wrapper crate provides a streaming handle; `agent_api` forwards mapped events live”.
  - Conflict risk: event mapping parity (ToolCall/ToolResult/text deltas) must remain best-effort
    consistent across backends, otherwise universal consumers will need backend-specific branching.

### Planning packs (`docs/project_management/next/*`)

- `docs/project_management/next/universal-agent-api/`
  - Overlap: authoritative specs for universal envelope/protocol/capabilities.
  - Conflict risk: none if this feature references those docs as baselines and does not redefine.
- `docs/project_management/next/claude-code-live-stream-json/`
  - Overlap: demonstrates the desired streaming-parity shape for Claude; Codex refactor should not
    weaken safety or finality semantics compared to Claude’s backend.

## Concrete follow-ups (Decision Register / spec updates)

Required decision-register entries for this feature (must be pinned as A/B with one selection):
- Populate `AgentWrapperCompletion.final_text` for Codex (yes/no; determinism rule).
- Redaction policy for `ExecStreamError` variants that carry raw JSONL lines (exact mapping).
- Per-run env override strategy for `CodexClient` (where env lives, precedence, no leakage).
- Exec policy extension surface (non-interactive + sandbox/approvals) and default behavior.

Required spec updates (must be reflected in the new spec docs listed in `spec_manifest.md`):
- `contract.md`: absence semantics for `timeout`, `working_dir`, and `env`; and explicit stderr/redaction behavior.
- `codex-stream-exec-adapter-protocol-spec.md`: ordering/backpressure/drain behavior and error mapping.
- `platform-parity-spec.md`: cross-platform evidence requirements for fake-binary streaming tests.
