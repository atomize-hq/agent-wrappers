### S1 — Contract definition and validation rules

- **User/system value**: unblocks downstream seams by pinning the exact semantics and pre-spawn
  validation behavior for `agent_api.exec.external_sandbox.v1`.
- **Scope (in/out)**:
  - In:
    - Define ES-C01/02/06 in `docs/specs/universal-agent-api/extensions-spec.md`:
      - boolean schema, default `false`, and “dangerous” meaning,
      - validation ordering (before spawn; after R0 support gate),
      - contradiction rules:
        - with `agent_api.exec.non_interactive=false`,
        - with any `backend.<agent_kind>.exec.*` keys (ambiguous precedence).
  - Out:
    - Any backend capability advertising or implementation details (SEAM-2/3/4).
- **Acceptance criteria**:
  - The key `agent_api.exec.external_sandbox.v1` is defined as a boolean with default `false`.
  - Non-boolean values fail before spawn with `AgentWrapperError::InvalidRequest`.
  - If `external_sandbox=true` and `agent_api.exec.non_interactive=false` is explicitly requested
    (and both keys are supported per R0), the request fails before spawn with
    `AgentWrapperError::InvalidRequest`.
  - If `external_sandbox=true` and any `backend.<agent_kind>.exec.*` key is present (and supported
    per R0), the request fails before spawn with `AgentWrapperError::InvalidRequest`.
  - The doc is explicit that cross-key contradiction rules apply only after all keys pass R0.
- **Dependencies**:
  - R0 “Fail-closed capability gating” global rule in `extensions-spec.md` (no changes required).
- **Verification**:
  - Spec review for ambiguity: contradiction rules are precise and fail-closed.
  - Cross-check wording against `docs/project_management/packs/.../threading.md` (ES-C01/02/06).
- **Rollout/safety**:
  - Contract-only; does not change runtime posture without SEAM-2 (opt-in advertising) and explicit
    per-run usage.

#### S1.T1 — Define the key schema/default/meaning

- **Outcome**: `agent_api.exec.external_sandbox.v1` is present in the core extension registry with
  concrete semantics that are not implied by benign keys.
- **Inputs/outputs**:
  - Input: `docs/project_management/packs/.../seam-1-external-sandbox-extension-key.md`
  - Output: updates to `docs/specs/universal-agent-api/extensions-spec.md`
- **Implementation notes**:
  - Keep the definition self-contained and explicitly “dangerous”.
  - Ensure the “validated before spawn” requirement is stated alongside the key.
- **Acceptance criteria**:
  - Key string, type, and default are unambiguous.
  - Meaning statement makes the host assertion explicit (“external isolation boundary”).
- **Test notes**:
  - N/A (spec-only); downstream seams will pin behavior with tests (SEAM-5).
- **Risk/rollback notes**:
  - N/A (spec-only).

Checklist:
- Implement: add/update the registry entry under core keys.
- Test: N/A.
- Validate: `rg -n "agent_api\\.exec\\.external_sandbox\\.v1" docs/specs/universal-agent-api/extensions-spec.md` and confirm schema/default/meaning.
- Cleanup: ensure references match canonical doc names.

#### S1.T2 — Specify contradiction rules (ES-C02, ES-C06) with R0 precedence

- **Outcome**: the contradiction rules are explicit, fail-closed, and correctly ordered relative to
  R0 capability support gating.
- **Inputs/outputs**:
  - Input: `docs/project_management/packs/.../threading.md` (ES-C02, ES-C06)
  - Output: updates to `docs/specs/universal-agent-api/extensions-spec.md`
- **Implementation notes**:
  - Make it explicit that the contradiction rules only apply after all keys pass R0.
  - Include at least one example of `backend.<agent_kind>.exec.*` keys for clarity.
- **Acceptance criteria**:
  - Contradictions are pinned to `AgentWrapperError::InvalidRequest` and “before spawn”.
  - The `backend.<agent_kind>.exec.*` prohibition is clearly scoped to `external_sandbox=true`.
- **Test notes**:
  - Ensure the statements are testable and align with planned coverage in SEAM-5.
- **Risk/rollback notes**:
  - N/A (spec-only).

Checklist:
- Implement: add/update contradiction rules and examples.
- Test: N/A.
- Validate: re-read R0 precedence text + the key’s contradiction bullets end-to-end; ensure the R0 gate is described as applying before contradiction rules.
- Cleanup: keep the section concise and consistent with existing key registry formatting.
