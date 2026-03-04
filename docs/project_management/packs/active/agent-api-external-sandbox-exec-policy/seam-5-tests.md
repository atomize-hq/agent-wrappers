# SEAM-5 — Tests

- **Name**: regression coverage for `agent_api.exec.external_sandbox.v1`
- **Type**: integration (contract conformance)
- **Goal / user value**: prevent regressions that would accidentally advertise or accept the
  dangerous key by default, or that would allow interactive hangs/unsafe spawn behavior.

## Scope

- In:
  - Capability advertising tests (default off; opt-in on).
  - Harness ordering tests:
    - unsupported extension keys fail closed before any value/contradiction validation.
  - Backend validation tests:
    - boolean type validation for the key,
    - contradiction handling with `agent_api.exec.non_interactive`,
    - exec-policy combination rule: `external_sandbox=true` rejects any `backend.*.exec.*` keys,
    - no spawn when invalid / contradictory.
  - Mapping tests (best-effort, unit-level):
    - Codex argv/builder contains dangerous bypass override when enabled + requested.
    - Claude argv contains dangerous permission bypass flag(s) when enabled + requested.
- Out:
  - End-to-end live CLI integration tests (can be added later behind an opt-in if needed).

## Primary interfaces (contracts)

- **Inputs**: `AgentWrapperRunRequest.extensions` combinations
- **Outputs**: `UnsupportedCapability` / `InvalidRequest` errors, and deterministic argv/mapping behavior

## Dependencies

- Blocked by: SEAM-1..4 (final semantics + mapping).

## Touch surface

- Harness tests:
  - `crates/agent_api/src/backend_harness/normalize/tests.rs`
- Backend tests:
  - `crates/agent_api/src/backends/codex/tests.rs`
  - `crates/agent_api/src/backends/claude_code/tests.rs`

## Verification

- Run targeted tests while iterating:
  - `cargo test -p agent_api backend_harness::normalize`
  - `cargo test -p agent_api codex`
  - `cargo test -p agent_api claude_code`

## Risks / unknowns

- None (pinned: help-preflight is a unit-testable seam; see `docs/specs/claude-code-session-mapping-contract.md`).
