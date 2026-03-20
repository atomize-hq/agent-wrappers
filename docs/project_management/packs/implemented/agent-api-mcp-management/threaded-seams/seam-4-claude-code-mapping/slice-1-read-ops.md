# S1 — Read ops (`list/get`) mapping + bounded exec runner (decomposed)

- Archived original: `archive/slice-1-read-ops.md`
- Sub-slices live in: `slice-1-read-ops/`
- Recommended order: S1a → S1b → S1c → S1d

#### Sub-slices

- `slice-1-read-ops/subslice-1-argv-and-capture.md` — S1a: pinned argv builders + bounded capture primitive
- `slice-1-read-ops/subslice-2-runner.md` — S1b: command runner (context precedence + timeout + bounded output)
- `slice-1-read-ops/subslice-3-drift-classifier.md` — S1c: manifest/runtime drift classifier (fail closed; pinned)
- `slice-1-read-ops/subslice-4-hooks-and-tests.md` — S1d: hook wiring + fail-closed gating + unit tests
