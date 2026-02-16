# C0 Spec — Core Universal Agent API Crate

Status: Draft  
Date (UTC): 2026-02-16  
Owner: Universal Agent API triad (C0)

## Scope (required)

Implement the core `agent_api` crate as an agent-agnostic facade and registry.

### In-scope deliverables

- New crate: `crates/agent_api`
- New CI workflow (enables CP1 checkpoints on GitHub-hosted runners):
  - `.github/workflows/universal-agent-api-smoke.yml`
- Core types (names are part of the contract; see `contract.md`):
  - `AgentKind` (open-set agent identity; string-backed)
  - `AgentCapabilities` (namespaced string capability ids)
  - `AgentEvent` + `AgentEventKind` (unified minimal event envelope)
  - `AgentRunRequest` (core run request, with bounded extension options)
  - `AgentBackend` (trait) and `AgentGateway` (registry + routing)
  - Error taxonomy for unknown backend + unsupported capability
- No real backend implementations in C0 (backends land in C1/C2 behind feature flags).

### Out of scope (explicit)

- Codex backend adapter implementation (C1).
- Claude Code backend adapter implementation (C2).
- Any requirement for real agent binaries in tests (fixtures/samples only).
- Any cross-crate refactor of `wrapper_events` identity types.

## Acceptance Criteria (observable)

- `cargo test --workspace --all-targets --all-features` passes on Linux.
- `crates/agent_api` compiles with **no backend features enabled**.
- The smoke workflow exists at `.github/workflows/universal-agent-api-smoke.yml` and runs the feature-local smoke scripts on `ubuntu-latest`, `macos-latest`, and `windows-latest`.
- Public surface matches `docs/project_management/next/universal-agent-api/contract.md` for:
  - type names
  - feature flags
  - error variants and when they are emitted
- `AgentKind` is open-set (no “update enum for every new agent” requirement).

## Notes / constraints

- Streaming semantics are capability-gated (see `decision_register.md` DR-0001 and `run-protocol-spec.md`).
- The event envelope must support bounded extension payloads without leaking raw lines by default (see schema specs).

## CI workflow contract (normative for C0)

The workflow at `.github/workflows/universal-agent-api-smoke.yml` MUST:

- Be triggerable via `workflow_dispatch`.
- Run a 3-OS matrix using GitHub-hosted runners:
  - `ubuntu-latest` → `docs/project_management/next/universal-agent-api/smoke/linux-smoke.sh`
  - `macos-latest` → `docs/project_management/next/universal-agent-api/smoke/macos-smoke.sh`
  - `windows-latest` → `docs/project_management/next/universal-agent-api/smoke/windows-smoke.ps1`
- Run `make preflight` on `ubuntu-latest` for the same tested ref (can be a separate job).
