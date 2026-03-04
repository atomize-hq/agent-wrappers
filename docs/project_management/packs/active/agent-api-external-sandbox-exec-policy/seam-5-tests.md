# SEAM-5 — Tests

- **Name**: regression coverage for `agent_api.exec.external_sandbox.v1`
- **Type**: integration (contract conformance)
- **Goal / user value**: prevent regressions that accidentally advertise or accept the dangerous
  key by default, or that allow interactive hangs/unsafe spawn behavior.

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
  - Mapping tests (required; pinned):
    - Codex (exec + resume): argv MUST contain `--dangerously-bypass-approvals-and-sandbox` and MUST
      NOT contain any of: `--full-auto`, `--ask-for-approval`, `--sandbox`.
    - Codex (fork/app-server): RPC MUST use:
      - `approvalPolicy="never"` (thread/fork + turn/start)
      - `sandbox="danger-full-access"` (thread/fork)
      per `docs/specs/codex-external-sandbox-mapping-contract.md`.
    - Claude Code: argv MUST contain `--dangerously-skip-permissions`, and MUST include/exclude
      `--allow-dangerously-skip-permissions` exactly per the pinned help-preflight strategy in
      `docs/specs/claude-code-session-mapping-contract.md`:
      - allow-flag supported → argv includes `--allow-dangerously-skip-permissions`
      - allow-flag not supported → argv excludes `--allow-dangerously-skip-permissions`
      - preflight failure → fail before spawn as `AgentWrapperError::Backend { .. }`
- Out:
  - End-to-end live CLI integration tests are **not required** for v1 acceptance.
    - Trigger to add e2e tests (objective): once CI provides real `codex` + `claude` binaries and a
      dedicated lane sets `AGENT_API_E2E_LIVE=1`, add e2e coverage and wire that lane to run it.
    - Opt-in mechanism (pinned): environment-variable gated.
      - Local: set `AGENT_API_E2E_LIVE=1` and point binaries via `CODEX_E2E_BINARY` (Codex) and
        `CLAUDE_BINARY` (Claude).
      - CI: keep `AGENT_API_E2E_LIVE` unset in default lanes; only the dedicated lane sets it.

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
