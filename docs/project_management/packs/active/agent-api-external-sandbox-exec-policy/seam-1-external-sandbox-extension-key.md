# SEAM-1 — Core extension key contract

- **Name**: `agent_api.exec.external_sandbox.v1` (external sandbox execution policy)
- **Type**: risk (dangerous execution policy) + integration (cross-backend)
- **Goal / user value**: allow externally sandboxed hosts to explicitly request that built-in
  backends relax internal approvals/sandbox/permissions guardrails while remaining non-interactive.

## Scope

- In:
  - Add the key to the normative core registry in `docs/specs/universal-agent-api/extensions-spec.md`.
  - Define:
    - schema (boolean),
    - default/absence semantics,
    - validation timing (before spawn),
    - contradiction rule with `agent_api.exec.non_interactive`.
  - Define backend mapping requirements at the contract level (Codex + Claude Code).
- Out:
  - Backend-specific implementation details beyond the mapping requirements.
  - Host-side sandbox implementation (this key assumes it exists).

## Primary interfaces (contracts)

- **Extension key**: `agent_api.exec.external_sandbox.v1`
  - **Type**: boolean
  - **Default when absent**: `false` (per `docs/specs/universal-agent-api/extensions-spec.md`)
  - **Meaning**: when `true`, the host asserts it provides isolation externally and requests the
    backend relax internal guardrails accordingly.
  - **Validation**: MUST be validated before spawn; non-boolean values fail with
    `AgentWrapperError::InvalidRequest`.
  - **Capability gating**: key presence requires backend support (fail-closed via
    `UnsupportedCapability` when not advertised).

- **Cross-key contradiction rule**:
  - When `agent_api.exec.external_sandbox.v1 == true`, the backend MUST remain non-interactive.
  - If `agent_api.exec.non_interactive == false` is explicitly requested alongside it (and both
    keys are supported), the backend MUST fail before spawn with `AgentWrapperError::InvalidRequest`
    (contradictory intent).

## Key invariants / rules

- This key is explicitly dangerous:
  - built-in backends MUST NOT advertise it by default,
  - it MUST remain capability-gated,
  - and it MUST remain non-interactive (no hangs on prompts).
- Observability / audit signal (v1, pinned; canonical):
  - See `docs/specs/universal-agent-api/extensions-spec.md` under
    `agent_api.exec.external_sandbox.v1` for the required `Status` warning event and emission timing.

## Dependencies

- Blocks: SEAM-2/3/4/5 (they need pinned semantics).
- Blocked by: none.

## Touch surface

- `docs/specs/universal-agent-api/extensions-spec.md`

## Verification

- Spec review: confirm schema, defaults, and contradiction rules are unambiguous.
- Local validation (pinned commands + pass criteria):
  - Capability matrix (required once any backend capability advertisement changes land):
    - `cargo run -p xtask -- capability-matrix` (must exit 0; output is deterministic)
    - `cargo run -p xtask -- capability-matrix-audit` (must exit 0)
  - Integration gate (WS-INT): `make preflight` must pass before merge.

## Risks / unknowns

- None (pinned: `external_sandbox=true` rejects `backend.<agent_kind>.exec.*` keys; see `extensions-spec.md`).

## Rollout / safety

- Default posture: not advertised by built-in backends.
- Hosts must opt in explicitly (backend config + per-run extension key).
