### S3 — Runtime rejection conformance and contract publication

- **Status**: decomposed into sub-slices sized for a single Codex session.
- **Why decomposed**:
  - touches runtime classification code plus a dedicated fake scenario binary
  - spans multiple verification layers across backend runtime tests and focused Codex contract tests
  - includes two normative spec updates in addition to code/test work
- **Archived original**: `archive/slice-3-runtime-rejection-conformance.md`
- **Sub-slice directory**: `slice-3-runtime-rejection-conformance/`

#### Sub-slices

- `subslice-1-runtime-rejection-translation.md`
  - Covers the implementation core of original `S3.T1`: fake-scenario support plus the narrowed runtime-rejection translation path and event/completion parity.
- `subslice-2-runtime-rejection-backend-tests.md`
  - Covers the backend regression layer extracted from original `S3` verification work: focused Codex tests that pin runtime translation, mapping, and fork posture without changing normative docs.
- `subslice-3-codex-contract-publication.md`
  - Covers the normative publication portion extracted from original `S3.T2`: update the two Codex spec contracts after S1/S2/S3a behavior is stable.

#### Sequencing

- Start with `subslice-1-runtime-rejection-translation.md`.
- Follow with `subslice-2-runtime-rejection-backend-tests.md`.
- Finish with `subslice-3-codex-contract-publication.md` once the code and focused tests are settled.
