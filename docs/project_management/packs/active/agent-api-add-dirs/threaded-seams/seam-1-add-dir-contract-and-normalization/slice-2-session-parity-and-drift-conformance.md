### S2 — Session parity and drift conformance

- **User/system value**: Backend seams inherit one explicit rule for how accepted add-dir inputs behave across new-session, resume, and fork flows, and the ADR stays mechanically aligned with that owner-doc truth.
- **Scope (in/out)**:
  - In:
    - Pin `AD-C04` in the owner doc and mirror it in ADR-0021 without contradiction.
    - Preserve the ADR drift-guard workflow for this seam.
  - Out:
    - Editing backend-owned session mapping docs.
    - Writing or changing backend tests.
- **Acceptance criteria**:
  - `docs/specs/universal-agent-api/extensions-spec.md` states that accepted add-dir inputs are orthogonal to session selector keys and must survive new-session, resume, and fork decision-making unless a pinned backend rejection contract applies.
  - ADR-0021 reflects the same fork/resume truth, including the pinned Codex fork rejection boundary and Claude fork applicability.
  - `make adr-check ADR=docs/adr/0021-universal-agent-api-add-dirs.md` passes after any ADR edits.
- **Dependencies**:
  - Requires `S1` to establish the base add-dir contract.
  - Evidence-only references:
    - `docs/specs/codex-app-server-jsonrpc-contract.md`
    - `docs/specs/claude-code-session-mapping-contract.md`
    - `docs/project_management/packs/active/agent-api-add-dirs/threading.md`
- **Verification**:
  - Spec review against `AD-C04`.
  - `make adr-check ADR=docs/adr/0021-universal-agent-api-add-dirs.md`
- **Rollout/safety**:
  - Doc-only and additive.
  - Backend doc updates remain owned by later seams; use them here only as evidence.

#### S2.T1 — Publish the session-flow parity contract in the owner doc

- **Outcome**: `docs/specs/universal-agent-api/extensions-spec.md` unambiguously describes how accepted add-dir inputs interact with resume and fork flows.
- **Inputs/outputs**:
  - Inputs:
    - `docs/project_management/packs/active/agent-api-add-dirs/threading.md`
    - `docs/specs/codex-app-server-jsonrpc-contract.md`
    - `docs/specs/claude-code-session-mapping-contract.md`
  - Outputs:
    - Updated `docs/specs/universal-agent-api/extensions-spec.md`
- **Implementation notes**:
  - Keep the owner doc focused on the universal rule: accepted add-dir inputs must survive session selection unless a pinned backend-owned rejection contract applies.
  - Name the pinned Codex fork exception without trying to redefine the backend contract inside this seam.
  - Preserve the statement that session selector keys live in `AgentWrapperRunRequest.extensions`.
- **Acceptance criteria**:
  - `AD-C04` is represented in the owner doc without ambiguity.
  - The text distinguishes accepted-input fork rejection from invalid-input `InvalidRequest` failures.
- **Test notes**:
  - Manual review against the `AD-C04` definition in `threading.md`.
- **Risk/rollback notes**:
  - Main risk is leaking backend-owned implementation detail into the universal owner doc instead of referencing it as a pinned exception.

Checklist:
- Implement:
  - Tighten the session-compatibility paragraphs in `docs/specs/universal-agent-api/extensions-spec.md`.
  - Cross-check the language against the pinned Codex and Claude evidence docs without editing them.
- Test:
  - Confirm the resulting text covers new-session, resume selector behavior, and fork behavior.
- Validate:
  - Confirm the accepted-input vs invalid-input precedence remains coherent with the pack threading.
- Cleanup:
  - Remove vague phrases that could let SEAM-3 or SEAM-4 reinterpret session handling locally.

#### S2.T2 — Align ADR-0021 and refresh the drift guard

- **Outcome**: ADR-0021 remains a rationale/plan document that derives from the owner doc and passes the repo’s ADR drift check.
- **Inputs/outputs**:
  - Inputs:
    - Updated `docs/specs/universal-agent-api/extensions-spec.md`
    - `docs/adr/0021-universal-agent-api-add-dirs.md`
  - Outputs:
    - Updated `docs/adr/0021-universal-agent-api-add-dirs.md`
    - Refreshed ADR drift hash if the ADR body changed
- **Implementation notes**:
  - Keep the ADR non-normative.
  - If the ADR body changes, run the repo-prescribed fix/check sequence rather than hand-editing the guard hash.
- **Acceptance criteria**:
  - ADR session-flow text matches the owner doc on orthogonality, Claude fork applicability, and the pinned Codex fork rejection path.
  - `make adr-check ADR=docs/adr/0021-universal-agent-api-add-dirs.md` passes.
- **Test notes**:
  - Run `make adr-fix ADR=docs/adr/0021-universal-agent-api-add-dirs.md` only if the ADR changed.
  - Then run `make adr-check ADR=docs/adr/0021-universal-agent-api-add-dirs.md`.
- **Risk/rollback notes**:
  - Main risk is leaving the ADR_BODY_SHA256 stale after a content update.

Checklist:
- Implement:
  - Edit ADR-0021 so it mirrors the owner-doc session-flow truth and clearly defers normative ownership to `extensions-spec.md`.
- Test:
  - Run `make adr-fix ADR=docs/adr/0021-universal-agent-api-add-dirs.md` if needed.
  - Run `make adr-check ADR=docs/adr/0021-universal-agent-api-add-dirs.md`.
- Validate:
  - Confirm ADR-0021 still reads as rationale plus implementation plan, not as a competing contract.
- Cleanup:
  - Drop any stale wording that implies session-flow semantics are still open.
