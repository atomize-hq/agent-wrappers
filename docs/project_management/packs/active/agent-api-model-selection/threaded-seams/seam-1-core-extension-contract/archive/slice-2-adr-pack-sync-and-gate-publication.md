### S2 — ADR/Pack Sync And Gate Publication

- **User/system value**: Publish one downstream-citable SEAM-1 gate by synchronizing the ADR and pack restatements to the final canonical contract.
- **Scope (in/out)**:
  - In:
    - sync ADR-0020 and pack files after S1 finishes
    - update the seam file's verification record with the latest verdict and synchronization reference
    - ensure the pack restatements of MS-C01 through MS-C04 match the canonical docs exactly enough for downstream seams to cite them safely
  - Out:
    - inventing new semantics that were not settled in S1
    - backend code, matrix generation, or runtime test implementation
- **Acceptance criteria**:
  - ADR-0020, `README.md`, `scope_brief.md`, `threading.md`, and `seam-1-core-extension-contract.md` reflect the final canonical wording and blocker posture
  - `seam-1-core-extension-contract.md` contains the latest verification-record entry with verdict, compared sources, verifier, and synchronization reference
  - downstream seams can cite that record without needing to infer missing context
- **Dependencies**:
  - Cross-seam: none
  - Prior slice: S1 must deliver a final verification verdict
  - Contracts: republishes owned contracts MS-C01 through MS-C04 for planning purposes; does not redefine them
- **Verification**:
  - compare every updated ADR/pack restatement against the final S1 verdict package
  - run `make adr-fix ADR=docs/adr/0020-universal-agent-api-model-selection.md` if ADR text changed
  - confirm the published verification record matches the exact synchronization reference currently in effect
- **Rollout/safety**:
  - only publish a pass when S1 produced a clean pass
  - if S1 ends in failure, S2 records the blocking state instead of papering over it

#### S2.T1 — Synchronize ADR-0020 to the canonical contract

- **Outcome**: ADR-0020 reflects the final canonical truth and continues to serve as rationale rather than a competing contract source.
- **Inputs/outputs**:
  - Inputs: S1 final verdict, canonical docs, current ADR text
  - Outputs: updated `docs/adr/0020-universal-agent-api-model-selection.md` plus any required ADR drift-fix output
- **Implementation notes**:
  - sync only the sections named in the seam brief verification scope
  - preserve the ADR's rationale/rollout role; do not move normative ownership out of `docs/specs/**`
- **Acceptance criteria**:
  - every ADR section that restates model-selection semantics matches the canonical docs after S1
  - any ADR drift guard or formatting requirements are satisfied
- **Test notes**:
  - run `make adr-fix ADR=docs/adr/0020-universal-agent-api-model-selection.md` when the ADR changes
- **Risk/rollback notes**:
  - if the ADR contains rationale that no longer fits the canonical contract, rewrite the rationale to point at the spec instead of softening the spec text

Checklist:
- Implement: update only the ADR sections covered by the seam verification scope to match the final canonical wording.
- Test: run `make adr-fix ADR=docs/adr/0020-universal-agent-api-model-selection.md` when needed and inspect the resulting diff.
- Validate: confirm the ADR now reads as rationale and rollout guidance, not as a second source of normative truth.
- Cleanup: remove any stale wording that could reopen ambiguity about ownership, validation, or advertising posture.

#### S2.T2 — Synchronize pack restatements and seam metadata

- **Outcome**: The pack files restate SEAM-1 accurately enough for downstream execution planning without conflicting with canonical docs.
- **Inputs/outputs**:
  - Inputs: S1 final verdict, canonical docs, current pack files
  - Outputs: updates to `README.md`, `scope_brief.md`, `threading.md`, and `seam-1-core-extension-contract.md` as needed
- **Implementation notes**:
  - keep the pack planning-oriented; avoid duplicating large normative paragraphs when a precise cross-reference is enough
  - preserve the existing blocker graph and contract ownership from `threading.md`
- **Acceptance criteria**:
  - all pack restatements of MS-C01 through MS-C04 align with the final canonical wording
  - the pack still makes clear that SEAM-1 blocks SEAM-2 through SEAM-5 until the published gate says otherwise
- **Test notes**:
  - diff the updated pack text against the S1 comparison matrix to ensure no stale language remains
- **Risk/rollback notes**:
  - do not "fix" downstream seam plans here; only update the SEAM-1-facing planning statements that changed because of S1

Checklist:
- Implement: update only the pack files whose SEAM-1 restatements drifted from the final canonical contract.
- Test: compare each changed paragraph against the S1 verdict package and confirm the same blocker posture still holds.
- Validate: ensure no pack file now claims ownership of semantics that belong in `docs/specs/**`.
- Cleanup: remove obsolete provisional notes or TODOs once the synced restatement is in place.

#### S2.T3 — Publish the verification record and unblock signal

- **Outcome**: `seam-1-core-extension-contract.md` carries the latest official pass/fail entry that downstream seams must cite.
- **Inputs/outputs**:
  - Inputs: S1 final verdict package and any synced ADR/pack changes
  - Outputs: appended verification-record entry in `seam-1-core-extension-contract.md`
- **Implementation notes**:
  - include verification date, verifier, compared sources, result string, and synchronization reference exactly as required by the seam brief
  - if the work is still at a provisional working-tree reference, note that explicitly and schedule replacement once a commit or PR exists
- **Acceptance criteria**:
  - the newest verification-record entry is sufficient for SEAM-2 through SEAM-5 to cite verbatim
  - there is exactly one current unblock signal, and it matches the real state of S1
- **Test notes**:
  - verify the recorded compared-source list and result string match the S1 handoff exactly
- **Risk/rollback notes**:
  - if later changes reopen drift, append a newer failing entry immediately rather than editing history to look clean

Checklist:
- Implement: append the latest verification-record entry with the exact verdict and synchronization reference from S1.
- Test: re-read the new entry in context and confirm it satisfies every recording rule in the seam brief.
- Validate: ensure downstream seams can cite the entry without consulting out-of-band notes.
- Cleanup: replace stale provisional synchronization references once a commit or PR reference exists.
