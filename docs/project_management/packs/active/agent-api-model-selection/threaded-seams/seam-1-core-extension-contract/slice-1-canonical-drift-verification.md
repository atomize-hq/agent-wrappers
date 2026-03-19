### S1 — Canonical Drift Verification

- **User/system value**: Prove that the authoritative universal docs still pin one coherent model-selection contract before backend seams build on it.
- **Scope (in/out)**:
  - In:
    - compare canonical docs and inherited baselines against MS-C01 through MS-C04
    - resolve any mismatch in canonical spec files before touching ADR or pack restatements
    - rerun the comparison until the result is either a clean pass or an explicitly documented blocking delta
  - Out:
    - updating ADR-0020 or pack files except as evidence inputs
    - backend code or test changes
- **Acceptance criteria**:
  - the comparison covers `extensions-spec.md`, `capabilities-schema-spec.md`, `contract.md`, `run-protocol-spec.md`, ADR-0020 sections named by the seam brief, and the pack restatements
  - MS-C01 through MS-C04 each resolve to exactly one canonical meaning with no unresolved cross-doc mismatch
  - if canonical-doc drift exists, the fix lands in the canonical doc set first and the comparison is rerun against the updated text
- **Dependencies**:
  - Cross-seam: none
  - Contracts: owns MS-C01, MS-C02, MS-C03, MS-C04
- **Verification**:
  - manual drift checklist covering capability id/bucket placement, trim-before-validate semantics, trimmed byte bound, absence semantics, exact `invalid agent_api.config.model.v1` template, and backend-owned runtime rejection posture
  - rerun the checklist after any canonical patch and confirm the final result is ready for publication by S2
- **Rollout/safety**:
  - fail closed: do not let downstream seams cite SEAM-1 as satisfied until this slice reaches a clean pass
  - keep changes doc-scoped inside canonical specs; do not introduce backend implementation decisions here

#### S1.T1 — Execute the canonical drift checklist

- **Outcome**: A complete comparison matrix for MS-C01 through MS-C04 across the canonical universal docs, ADR, and pack restatements.
- **Inputs/outputs**:
  - Inputs: `threading.md`, `seam-1-core-extension-contract.md`, `scope_brief.md`, `README.md`, ADR-0020, and the canonical universal spec files
  - Outputs: a written verdict for each contract subject stating either "aligned" or the exact mismatch that must be reconciled
- **Implementation notes**:
  - treat `docs/specs/**` as authoritative and use ADR/pack text only to detect drift
  - check the inherited runtime-error/event language in `contract.md` and `run-protocol-spec.md`, not just the extension owner doc
- **Acceptance criteria**:
  - every required comparison point from the seam brief verification section has an explicit result
  - any mismatch is concrete enough to patch without reopening interpretation debates
- **Test notes**:
  - verify the comparison includes the exact `InvalidRequest` template and the post-spawn terminal `Error` event rule
- **Risk/rollback notes**:
  - if a mismatch cannot be resolved from existing canonical docs, stop this slice as blocked rather than guessing

Checklist:
- Implement: enumerate the comparison points for MS-C01 through MS-C04 and record the alignment result for each source.
- Test: re-read every cited section to confirm the checklist did not miss capability bucket placement, byte bounds, or runtime rejection wording.
- Validate: confirm the checklist produces either "no unresolved canonical-doc delta" or a concrete patch list limited to canonical docs.
- Cleanup: remove any provisional notes that duplicate authoritative text once the final verdict is captured.

#### S1.T2 — Reconcile canonical universal docs if drift is found

- **Outcome**: Canonical universal spec files reflect one consistent truth for model-selection semantics before any non-normative sync work proceeds.
- **Inputs/outputs**:
  - Inputs: mismatch list from S1.T1
  - Outputs: patches to the affected files in `docs/specs/universal-agent-api/` and, only if required by the mismatch, the inherited baseline docs that define the error/run-lifecycle behavior
- **Implementation notes**:
  - patch the owner spec first, then the inherited baseline doc only when the mismatch actually lives there
  - do not edit ADR-0020 or pack files in this task; those belong to S2 after canonical truth is settled
- **Acceptance criteria**:
  - every mismatch from S1.T1 is either resolved in canonical docs or explicitly marked as a blocker with rationale
  - no patch introduces backend-specific mapping commitments outside the existing seam scope
- **Test notes**:
  - rerun the same drift checklist after patching and confirm the mismatched items now read as aligned
- **Risk/rollback notes**:
  - if a proposed patch would expand scope beyond MS-C01 through MS-C04, defer it instead of bundling unrelated policy changes

Checklist:
- Implement: update only the canonical files needed to eliminate the concrete mismatch list from S1.T1.
- Test: rerun the full comparison against the patched canonical text and verify the prior deltas are closed.
- Validate: confirm the resulting wording still matches SEAM-1 scope and does not invent wrapper-owned model behavior.
- Cleanup: trim provisional commentary from the patch once the canonical wording is self-sufficient.

#### S1.T3 — Produce the final verification verdict for handoff to S2

- **Outcome**: SEAM-1 has a final pass/fail result with enough evidence for S2 to publish or to stop the seam as blocked.
- **Inputs/outputs**:
  - Inputs: final comparison result from S1.T1 and any canonical patches from S1.T2
  - Outputs: a concise verification verdict, the compared-source list, and the synchronization reference S2 will record
- **Implementation notes**:
  - when no patch was needed, still produce an explicit final verdict anchored to the exact compared sources
  - when the work is still uncommitted, capture the current `git HEAD` plus working-tree delta note; replace it later once a commit or PR exists
- **Acceptance criteria**:
  - the verdict is specific enough that S2 can append it without reinterpretation
  - the verdict clearly states whether downstream seams remain blocked or may cite the pass
- **Test notes**:
  - confirm the compared-source list matches the seam brief verification requirements exactly
- **Risk/rollback notes**:
  - do not let a stale provisional synchronization reference survive once a commit or PR reference exists

Checklist:
- Implement: prepare the final compared-source list, verdict string, and synchronization reference for publication.
- Test: cross-check the verdict inputs against the seam brief to ensure no required source or rule was omitted.
- Validate: confirm the handoff package is sufficient for S2 to update the verification record without re-running discovery.
- Cleanup: discard superseded provisional verdicts so downstream seams see one current gate state.
