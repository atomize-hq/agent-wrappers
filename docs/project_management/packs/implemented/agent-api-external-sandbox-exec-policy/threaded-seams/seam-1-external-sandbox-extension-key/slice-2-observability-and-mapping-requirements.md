### S2 — Observability and mapping requirements

- **User/system value**: provides an explicit audit signal when dangerous mode is enabled and pins
  deterministic backend mapping requirements needed for safe adoption.
- **Scope (in/out)**:
  - In:
    - Specify a pinned warning `Status` event when `external_sandbox=true` is accepted.
    - Specify backend mapping requirements at the contract level (no prompts; deterministic; no
      spawn+retry loops).
    - Reference canonical mapping contracts and the universal “dangerous opt-in” contract section.
  - Out:
    - Implementing backend event emission (SEAM-3/4) and opt-in advertising behavior (SEAM-2).
- **Acceptance criteria**:
  - `extensions-spec.md` requires a single `Status` warning event when the key is accepted, and
    pins emission ordering relative to other event kinds.
  - The warning MUST NOT be emitted when the key is absent/`false`, unsupported, invalid, or
    contradictory.
  - `extensions-spec.md` includes mapping requirements that:
    - prevent interactive hangs (no approvals/permissions prompts),
    - bypass backend-specific “internal sandbox required” checks as needed,
    - remain deterministic (no spawn then retry with different flags).
  - `extensions-spec.md` links to:
    - `docs/specs/codex-external-sandbox-mapping-contract.md`
    - `docs/specs/claude-code-session-mapping-contract.md`
    - `docs/specs/universal-agent-api/contract.md` ("Dangerous capability opt-in ...")
- **Dependencies**:
  - `S1` (the key and its validation rules exist).
  - Canonical mapping contracts exist (owned by SEAM-3 and SEAM-4).
- **Verification**:
  - Ensure the warning event content is “safe” and fully pinned.
  - Verify referenced docs/section titles resolve and are stable.
- **Rollout/safety**:
  - Warning event is an audit aid; it does not alter default advertising posture (SEAM-2 owns that).

#### S2.T1 — Pin the “dangerous mode enabled” warning event

- **Outcome**: the spec requires a single, safe warning `Status` event when
  `agent_api.exec.external_sandbox.v1=true` is accepted.
- **Inputs/outputs**:
  - Input: SEAM-1 brief (observability requirement) + `threading.md`
  - Output: updates to `docs/specs/universal-agent-api/extensions-spec.md`
- **Implementation notes**:
  - Pin the exact `message` string and channel.
  - Pin emission ordering so it appears before other consumer-visible output/events.
- **Acceptance criteria**:
  - Event kind is `AgentWrapperEventKind::Status` with pinned `channel`, `message`, and `data=None`.
  - Non-emission cases are explicitly listed.
- **Test notes**:
  - Downstream seams should be able to assert ordering in tests (SEAM-5).
- **Risk/rollback notes**:
  - N/A (spec-only).

Checklist:
- Implement: write the event requirement under the key’s registry entry.
- Test: N/A.
- Validate: `rg -n "DANGEROUS: external sandbox exec policy enabled" docs/specs/universal-agent-api/extensions-spec.md` and confirm it’s gated on “accepted” (post-R0 + post-validation).
- Cleanup: keep warning wording stable and safe.

#### S2.T2 — Pin mapping requirements and canonical references

- **Outcome**: the spec includes contract-level mapping requirements plus canonical doc pointers
  that downstream seams must follow.
- **Inputs/outputs**:
  - Input: contract registry references in `threading.md` (ES-C04/05/07 context; not owned here)
  - Output: updates to `docs/specs/universal-agent-api/extensions-spec.md`
- **Implementation notes**:
  - Avoid duplicating backend-specific details; point at mapping contracts instead.
  - Include the explicit “not advertised by default; opt-in required” reference (owned by SEAM-2)
    as a contract requirement without taking ownership of its implementation.
- **Acceptance criteria**:
  - Requirements cover non-interactivity and determinism (no spawn+retry).
  - References to mapping contracts and the universal opt-in contract section are present and
    correct.
- **Test notes**:
  - N/A (spec-only); downstream seams will validate behavior in code/tests.
- **Risk/rollback notes**:
  - N/A (spec-only).

Checklist:
- Implement: update mapping requirements bullets and doc references.
- Test: N/A.
- Validate: confirm this seam only touches `docs/specs/universal-agent-api/extensions-spec.md`; verify referenced docs exist (`test -f docs/specs/codex-external-sandbox-mapping-contract.md` etc.).
- Cleanup: keep the section concise and consistent with the rest of the registry.
