### S2b — Pack Restatement And Seam Metadata Sync

- **User/system value**: Keep the pack's planning artifacts aligned with the final SEAM-1 contract so downstream workstreams can plan against one consistent blocker story and contract restatement set.
- **Scope (in/out)**:
  - In: update `README.md`, `scope_brief.md`, `threading.md`, and `seam-1-core-extension-contract.md` only where their SEAM-1 restatements drift from the final S1 verdict; preserve the existing blocker graph and contract ownership.
  - Out: patching canonical specs; editing ADR-0020; rewriting downstream seam plans that are unrelated to SEAM-1 contract drift.
- **Acceptance criteria**:
  - All pack restatements of MS-C01 through MS-C04 align with the final canonical wording and blocker posture.
  - The pack remains planning-oriented and does not duplicate large normative paragraphs when a precise cross-reference is enough.
  - No pack file claims semantic ownership that belongs in `docs/specs/**`.
- **Dependencies**:
  - Prior slice: S1 must provide the final verification verdict and compared-source package.
  - Contracts: republishes MS-C01 through MS-C04 for planning and dependency threading only.
- **Verification**:
  - Diff each changed pack paragraph against the S1 comparison matrix and final verdict package.
  - Re-read the blocker statements and contract-owner labels to confirm they still match `threading.md` and the seam brief.
- **Rollout/safety**:
  - Keep changes scoped to SEAM-1-facing planning text.
  - Remove obsolete provisional notes or TODOs once the synced restatement is in place.

#### S2b.T1 — Synchronize pack restatements and seam metadata

- **Outcome**: The pack files restate SEAM-1 accurately enough for downstream execution planning without conflicting with the canonical docs.
- **Files**:
  - `docs/project_management/packs/active/agent-api-model-selection/README.md`
  - `docs/project_management/packs/active/agent-api-model-selection/scope_brief.md`
  - `docs/project_management/packs/active/agent-api-model-selection/threading.md`
  - `docs/project_management/packs/active/agent-api-model-selection/seam-1-core-extension-contract.md`

Checklist:
- Implement:
  - Update only the pack files whose SEAM-1 restatements drifted from the final canonical contract.
  - Preserve the existing dependency edges, blocker graph, and contract ownership from `threading.md`.
  - Prefer concise restatements and explicit cross-references over copying large normative paragraphs into the pack.
- Test:
  - Compare each changed paragraph against the S1 verdict package and comparison matrix.
  - Confirm the same blocker posture still holds unless S1 explicitly changed the real gate state.
- Validate:
  - Ensure no pack file now claims ownership of semantics that belong in `docs/specs/**`.
  - Re-read the touched pack docs together and confirm they tell one coherent SEAM-1 story for downstream seams.
