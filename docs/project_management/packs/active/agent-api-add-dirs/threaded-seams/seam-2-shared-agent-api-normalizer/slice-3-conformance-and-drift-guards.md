### S3 — Lock conformance with exhaustive edge-case and handoff regression coverage

- This slice was decomposed into sub-slices in this directory:
  - `slice-3-conformance-and-drift-guards/`
- Archived original: `archive/slice-3-conformance-and-drift-guards.md`

#### Sub-slices

- `subslice-1-helper-conformance-matrix.md` — expands the shared helper's exhaustive normalization, failure-surface, and safe-message redaction matrix in `backend_harness`.
- `subslice-2-codex-policy-regressions.md` — pins Codex-only policy attachment, empty-vector absence semantics, and effective-cwd precedence without touching spawn mapping.
- `subslice-3-claude-policy-regressions-and-drift-audit.md` — pins Claude-only policy attachment and fallback precedence, then closes with the repo-level drift check that raw add-dir parsing stays out of later backend paths.
