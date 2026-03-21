### S2a — ADR-0020 Sync To Final Canonical Contract

- **User/system value**: Keep ADR-0020 aligned with the settled SEAM-1 contract so downstream readers get rationale and rollout context without encountering a competing contract source.
- **Scope (in/out)**:
  - In: sync ADR-0020 sections named by the seam brief after S1 delivers its final verdict; preserve the ADR's rationale and rollout role; run the ADR drift guard when the file changes.
  - Out: changing canonical ownership in `docs/specs/**`; editing pack files; publishing the verification record or downstream unblock signal.
- **Acceptance criteria**:
  - Every ADR section that restates MS-C01 through MS-C04 matches the final canonical docs and S1 verdict package.
  - `make adr-fix ADR=docs/adr/0020-universal-agent-api-model-selection.md` is run when the ADR changes, and any resulting drift-guard output is handled in the same change.
  - The ADR reads as rationale and rollout guidance, not as a second normative contract.
- **Dependencies**:
  - Prior slice: S1 must provide the final verification verdict and compared-source package.
  - Contracts: republishes MS-C01 through MS-C04 for rationale only; does not redefine them.
- **Verification**:
  - Compare each edited ADR section against the final S1 verdict package and the canonical docs named in the seam brief.
  - Run `make adr-fix ADR=docs/adr/0020-universal-agent-api-model-selection.md` if the ADR text changes and inspect the resulting diff.
- **Rollout/safety**:
  - Keep normative ownership pinned to `docs/specs/**`.
  - If rationale wording no longer fits the canonical contract, rewrite the rationale to point at the spec instead of weakening the spec restatement.

#### S2a.T1 — Synchronize ADR-0020 to the canonical contract

- **Outcome**: `docs/adr/0020-universal-agent-api-model-selection.md` reflects the final canonical truth while remaining a non-normative rationale document.
- **Files**:
  - `docs/adr/0020-universal-agent-api-model-selection.md`

Checklist:
- Implement:
  - Update only the ADR sections covered by the seam verification scope to match the final canonical wording from S1.
  - Preserve rationale, rollout guidance, and explicit references back to the canonical specs.
  - Remove stale wording that could reopen ambiguity about ownership, validation, mapping, or advertising posture.
- Test:
  - Run `make adr-fix ADR=docs/adr/0020-universal-agent-api-model-selection.md` when the ADR changes.
  - Inspect the resulting diff and confirm the drift guard matches the synchronized ADR text.
- Validate:
  - Re-read the updated ADR and confirm it does not claim normative ownership of model-selection semantics.
  - Confirm every restated MS-C01 through MS-C04 point matches the final S1 verdict package exactly enough to avoid re-interpretation.
