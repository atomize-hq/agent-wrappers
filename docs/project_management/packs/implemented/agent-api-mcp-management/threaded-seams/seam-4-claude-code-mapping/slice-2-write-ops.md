# S2 — Write ops (`add/remove`) mapping + typed transports + write gating (decomposed)

- Archived original: `archive/slice-2-write-ops.md`
- Sub-slices live in: `slice-2-write-ops/`
- Recommended order: S2a -> S2b -> S2c

#### Sub-slices

- `slice-2-write-ops/subslice-1-argv-builders.md` — S2a: pinned `add/remove` argv builders + deterministic `--env` ordering + bearer-token rejection
- `slice-2-write-ops/subslice-2-hooks-and-gating.md` — S2b: Claude write-hook wiring + fail-closed capability gating + runner reuse
- `slice-2-write-ops/subslice-3-regression-tests.md` — S2c: unit tests pinning argv mapping, gating, and `InvalidRequest` behavior
