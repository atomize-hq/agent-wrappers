### S2 — Backend mapping, absence semantics, and shared-normalizer handoff

- **Status**: decomposed into sub-slices sized for one Codex session.
- **Why it was split**:
  - spans both built-in backends plus shared parser-guard coverage
  - touches many distinct test modules across Codex and Claude surfaces
  - mixes capability advertising, argv mapping, fork behavior, and source-contract guardrails
- **Archived original**: `archive/slice-2-backend-mapping-and-absence.md`
- **Sub-slice directory**: `slice-2-backend-mapping-and-absence/`

#### Sub-slices

- `S2a` → `slice-2-backend-mapping-and-absence/subslice-1-codex-mapping-and-fork-guardrails.md`
  - Codex-only capability advertising, exec/resume mapping, absence behavior, and pre-handle fork
    rejection.
- `S2b` → `slice-2-backend-mapping-and-absence/subslice-2-claude-mapping-and-fallback-guardrails.md`
  - Claude-only capability advertising, argv placement, absence behavior, and explicit
    `--fallback-model` exclusion.
- `S2c` → `slice-2-backend-mapping-and-absence/subslice-3-single-parser-contract-guards.md`
  - shared single-parser guardrails in backend contract tests so neither backend re-reads the raw
    extension payload.

#### Task reassignment

- Original `S2.T1` now lives in `S2a`.
- Original `S2.T2` now lives in `S2b`.
- Original `S2.T3` now lives in `S2c`.
