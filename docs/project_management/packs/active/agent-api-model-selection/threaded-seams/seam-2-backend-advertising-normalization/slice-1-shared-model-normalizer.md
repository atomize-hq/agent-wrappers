### S1 — Shared model normalizer + normalized-request handoff

- This slice was decomposed because it spans shared helper implementation, `NormalizedRequest` contract plumbing, backend harness compile surfaces, and harness-level conformance tests across roughly six files/modules.
- Archived original: `archive/slice-1-shared-model-normalizer.md`
- Sub-slice directory: `slice-1-shared-model-normalizer/`

#### Sub-slices

- `subslice-1-helper-contract.md` — `S1a`, covering original `S1.T1` for the shared model-selection constant and harness-owned normalizer helper.
- `subslice-2-normalized-request-handoff.md` — `S1b`, covering original `S1.T2` for the typed `NormalizedRequest.model_selection` handoff and adapter compile-surface updates.
- `subslice-3-harness-conformance-tests.md` — `S1c`, covering original `S1.T3` for ordering, bounds, and typed-handoff regression coverage.

#### Task redistribution

- `S1.T1` moved to `S1a`.
- `S1.T2` moved to `S1b`.
- `S1.T3` moved to `S1c`.
