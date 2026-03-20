# S1 — Hermetic fake-binary harness + capability/non-run regressions (decomposed)

- Archived original: `archive/slice-1-hermetic-harness.md`
- Sub-slices live in: `slice-1-hermetic-harness/`
- Recommended order: S1a → S1b → S1c

#### Sub-slices

- `slice-1-hermetic-harness/subslice-1-fake-binaries.md` — S1a: fake `codex`/`claude` MCP binaries (records + sentinels + scenarios)
- `slice-1-hermetic-harness/subslice-2-test-support.md` — S1b: shared test support (isolated homes + per-test fake executables + record parsing)
- `slice-1-hermetic-harness/subslice-3-capability-non-run-regressions.md` — S1c: cross-backend capability posture + non-run boundary regressions (no spawn)
