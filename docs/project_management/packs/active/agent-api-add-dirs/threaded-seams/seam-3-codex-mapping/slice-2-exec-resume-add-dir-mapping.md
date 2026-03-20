### S2 — Exec/resume repeated `--add-dir` mapping

- Status: decomposed for single-session execution because the original slice spans multiple crates
  and mixes runtime plumbing with contract maintenance.
- Archived original: `archive/slice-2-exec-resume-add-dir-mapping.md`
- Sub-slice directory: `slice-2-exec-resume-add-dir-mapping/`

#### Why this was split

- The original slice combines runtime exec/resume plumbing with a separate builder dependency
  review, which exceeds the single-session budget for a planning slice.
- The runtime wiring work and the Normative contract update have different primary touch surfaces
  and different completion criteria.

#### Sub-slices

- `S2a` → `slice-2-exec-resume-add-dir-mapping/subslice-1-exec-resume-plumbing.md`
  - Covers original `S2.T1`: thread `policy.add_dirs` into `ExecFlowRequest` and hand the typed
    list to the existing Codex builder without changing ordering guarantees.
- `S2b` → `slice-2-exec-resume-add-dir-mapping/subslice-2-streaming-contract-alignment.md`
  - Covers original `S2.T2`: update `docs/specs/codex-streaming-exec-contract.md` to match the
    implemented exec/resume mapping exactly.
