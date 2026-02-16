# Impact Map — Universal Agent API

Status: Draft  
Date (UTC): 2026-02-16  
Feature directory: `docs/project_management/next/universal-agent-api/`

## Inputs

- ADR(s):
  - `docs/adr/0009-universal-agent-api.md`
- Spec manifest:
  - `docs/project_management/next/universal-agent-api/spec_manifest.md`

## Touch Set (explicit)

### Create (new files / new surfaces)

- New crate:
  - `crates/agent_api/Cargo.toml`
  - `crates/agent_api/src/lib.rs`
  - `crates/agent_api/src/*` (core types/traits + gateway + feature-gated backends)
  - `crates/agent_api/examples/*` (optional; only if required by specs)
  - `crates/agent_api/tests/*` (or `crates/agent_api/src/*` unit tests)
- New CI workflow (enables CP1 multi-OS checkpoints on GitHub-hosted runners):
  - `.github/workflows/universal-agent-api-smoke.yml`
- Planning pack (docs-only; derived from ADR + `spec_manifest.md`):
  - `docs/project_management/next/universal-agent-api/README.md`
  - `docs/project_management/next/universal-agent-api/plan.md`
  - `docs/project_management/next/universal-agent-api/tasks.json`
  - `docs/project_management/next/universal-agent-api/session_log.md`
  - `docs/project_management/next/universal-agent-api/ci_checkpoint_plan.md`
  - `docs/project_management/next/universal-agent-api/decision_register.md`
  - `docs/project_management/next/universal-agent-api/manual_testing_playbook.md`
  - `docs/project_management/next/universal-agent-api/quality_gate_report.md`
  - `docs/project_management/next/universal-agent-api/smoke/linux-smoke.sh`
  - `docs/project_management/next/universal-agent-api/smoke/macos-smoke.sh`
  - `docs/project_management/next/universal-agent-api/smoke/windows-smoke.ps1`
  - `docs/project_management/next/universal-agent-api/C0-spec.md`
  - `docs/project_management/next/universal-agent-api/C1-spec.md`
  - `docs/project_management/next/universal-agent-api/C2-spec.md`
  - `docs/project_management/next/universal-agent-api/kickoff_prompts/C0-code.md`
  - `docs/project_management/next/universal-agent-api/kickoff_prompts/C0-test.md`
  - `docs/project_management/next/universal-agent-api/kickoff_prompts/C0-integ.md`
  - (repeat kickoff prompts for C1/C2)
- Contract/spec docs (authoritative; per `spec_manifest.md`):
  - `docs/project_management/next/universal-agent-api/contract.md`
  - `docs/project_management/next/universal-agent-api/run-protocol-spec.md`
  - `docs/project_management/next/universal-agent-api/event-envelope-schema-spec.md`
  - `docs/project_management/next/universal-agent-api/capabilities-schema-spec.md`
  - `docs/project_management/next/universal-agent-api/platform-parity-spec.md`

### Edit (existing files / existing surfaces)

- Workspace wiring:
  - `Cargo.toml` (add `crates/agent_api` to workspace members; add workspace deps if needed)
  - `Cargo.lock` (will change once the crate lands and builds)
- Documentation index (if the universal API becomes a primary consumer surface):
  - `docs/README.md` (link to new crate docs or examples)
- ADR drift guard:
  - `docs/adr/0009-universal-agent-api.md` (keep `Related Docs` and `ADR_BODY_SHA256` current)
- Sequencing (triad system):
  - `docs/project_management/next/sequencing.json` (add a new track once the Planning Pack is authored)

### Possible edits (dependent on spec choices)

These edits are not guaranteed, but are likely depending on how the universal run protocol is
defined for “streaming” vs “buffered” backends:

- Claude Code streaming support:
  - `crates/claude_code/src/process/*` and/or `crates/claude_code/src/client/*` (add a spawn/stream
    API if “live streaming events” is required rather than post-hoc parsing of `--output stream-json`)
- Wrapper-events alignment:
  - `crates/wrapper_events/src/normalized.rs` (currently uses a closed enum `WrapperAgentKind`;
    if the universal API wants to reuse normalized event types directly, this may need an open-set
    identity, or the universal API must define its own envelope and keep wrapper_events unchanged)

## Cascading implications (behavior/UX) + contradiction risks

### 1) Agent identity: open-set vs closed enums

- Direct impact:
  - The universal API will identify agents via a string-backed `AgentKind` (open set).
- Second-order impact:
  - Any existing “agent kind” enums (e.g., `wrapper_events::WrapperAgentKind`) become a potential
    impedance mismatch for shared consumers.
- Contradiction risks:
  - If consumers mix `wrapper_events` normalized events and `agent_api` events, they may end up with
    two competing “agent kind” concepts (closed enum vs open string). This must be resolved by:
    - keeping the contracts separate (recommended), or
    - evolving `wrapper_events` identity to an open-set type (requires ADR/spec updates for 0007).

### 2) Streaming semantics across heterogeneous backends

- Direct impact:
  - `codex` supports true streaming typed events (`ExecStream`); `claude_code` today returns parsed
    output after process completion in `ClaudeClient::print`.
- Second-order impact:
  - The universal “run protocol” spec must define whether:
    - streaming is required (and Claude must be extended), or
    - buffered backends are acceptable (events emitted after completion), or
    - both modes exist with explicit capability gating.
- Contradiction risks:
  - If the universal API claims a streaming contract but silently buffers for some agents, operator
    expectations (progress UI, cancellation responsiveness) will diverge. This must be pinned in
    `run-protocol-spec.md` and represented as a capability.

### 3) Capability model overlaps (Codex “capabilities” vs universal “capabilities”)

- Direct impact:
  - `crates/codex` already exposes runtime probing (`CodexCapabilities`) and wrapper coverage uses
    “capability-guarded” notes (ADR 0002 and the coverage specs).
- Second-order impact:
  - The universal `AgentCapabilities` must either:
    - treat backend capabilities as opaque strings, or
    - define a stable namespace strategy to avoid collisions and confusion.
- Contradiction risks:
  - Overloading the term “capabilities” to mean both “CLI feature probes” and “universal API
    operations” can create ambiguous docs and mismatched expectations. The specs should explicitly
    distinguish:
    - backend/CLI probe capabilities (agent-reported), and
    - universal API operation capabilities (agent_api-defined).

### 4) Error taxonomy and “unsupported” behavior

- Direct impact:
  - The universal API introduces structured errors like `UnknownBackend` and `UnsupportedCapability`.
- Second-order impact:
  - Adapter layers must map backend-specific errors into the universal taxonomy without losing
    redaction guarantees.
- Contradiction risks:
  - If adapters leak raw output or secrets through error messages, this conflicts with the repo’s
    safety posture (see wrapper_events ADR 0007). The specs must pin redaction expectations.

## Cross-queue scan (ADRs + Planning Packs)

### ADRs in this repo (`docs/adr/*`)

- `docs/adr/0006-agent-wrappers-workspace.md`
  - Overlap: repo is explicitly multi-agent; universal API aligns with the “many wrappers” direction.
  - Conflict: none.
- `docs/adr/0007-wrapper-events-ingestion-contract.md`
  - Overlap: normalized event kinds + safety/redaction defaults.
  - Conflict risk: identity type mismatch (closed enum vs open-set string).
  - Resolution: keep contracts separate unless/until we decide to evolve wrapper_events identity.
- `docs/adr/0005-codex-jsonl-log-parser-api.md` and Codex specs under `docs/specs/`
  - Overlap: Codex JSONL parsing is Codex-specific and remains independent.
  - Conflict: none if `agent_api` consumes Codex via the `codex` crate APIs (no new parsing contract).
- `docs/adr/0002-codex-cli-parity-coverage-mapping.md`
  - Overlap: terminology (“capabilities”) and potential reuse of probe outputs.
  - Conflict: only if the universal API tries to standardize backend probe schemas across agents.
  - Resolution: universal capabilities remain operation/call-surface oriented; backend probes remain
    agent-specific and may be exposed via extension payloads.

### Planning Packs (`docs/project_management/next/*`)

Scan findings (best-effort):

- `docs/project_management/next/codex-jsonl-log-parser-api/`
  - Overlap: event parsing/normalization concepts.
  - Conflict: none if `agent_api` does not redefine Codex JSONL schemas.
- `docs/project_management/next/*parity*` tracks
  - Overlap: none (parity/automation tracks focus on wrapper coverage and snapshotting; universal API
    is a consumer-facing API composition layer).

## Follow-ups / Required pinning (before execution)

These are the concrete “pin or decide” items discovered while mapping impacts.

Pinned decisions (see `decision_register.md`):
- DR-0001: capability-gated buffered vs live streaming (selected: buffered allowed).
- DR-0002: keep `agent_api` event envelope independent of `wrapper_events` (selected).
- DR-0003: namespaced capability ids (selected).
- DR-0004: bounded JSON extension payloads for events (selected).

Remaining execution follow-ups:
1) Ensure `.github/workflows/universal-agent-api-smoke.yml` supports running against an explicit tested ref (branch or SHA) and records OS/job results deterministically.
