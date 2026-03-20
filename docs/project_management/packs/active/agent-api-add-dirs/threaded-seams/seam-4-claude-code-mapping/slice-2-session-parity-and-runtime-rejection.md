### S2 — Session-branch parity and runtime rejection conformance

- This slice was decomposed because it bundled three distinct concern groups across
  `harness.rs`, `mapping.rs`, backend-local regression tests, and the canonical Claude mapping
  doc:
  - selector-branch add-dir ordering parity,
  - runtime rejection event/completion parity,
  - final doc and drift-guard closure.
- Archived original: `archive/slice-2-session-parity-and-runtime-rejection.md`
- Sub-slice directory: `slice-2-session-parity-and-runtime-rejection/`

#### Sub-slices

- `subslice-1-selector-branch-add-dir-ordering.md`
  - `S2a`; moves `S2.T1` into a single session focused on resume/fork branch ordering and local
    ordered-subsequence assertions.
- `subslice-2-runtime-rejection-parity.md`
  - `S2b`; moves `S2.T2` into a single session focused on add-dir runtime rejection
    classification and terminal error/completion parity.
- `subslice-3-conformance-doc-and-drift-guards.md`
  - `S2c`; moves `S2.T3` into a single session focused on final canonical doc text and minimal
    Claude-only drift guards.
