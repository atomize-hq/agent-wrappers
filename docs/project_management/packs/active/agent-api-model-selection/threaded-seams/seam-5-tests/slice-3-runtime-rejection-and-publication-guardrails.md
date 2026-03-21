### S3 — Runtime rejection and capability-publication guardrails

- This slice was decomposed into sub-slices in this directory:
  - `slice-3-runtime-rejection-and-publication-guardrails/`
- Archived original: `archive/slice-3-runtime-rejection-and-publication-guardrails.md`
- Decomposition rationale:
  - The original slice coupled two backend-specific runtime-rejection fixtures/tests with a
    separate capability-publication validation path.
  - Splitting keeps each sub-slice within one main outcome, one primary touch surface, and one main
    verification layer.

#### Sub-slices

- `subslice-1-codex-runtime-rejection-guardrails.md`
  - Covers former `S3.T1`: Codex midstream rejection, safe error redaction, and exactly one
    terminal `AgentWrapperEventKind::Error`.
- `subslice-2-claude-runtime-rejection-guardrails.md`
  - Covers former `S3.T2`: Claude post-init rejection, safe error redaction, and exactly one
    terminal `AgentWrapperEventKind::Error`.
- `subslice-3-capability-matrix-freshness-guardrails.md`
  - Covers former `S3.T3`: generator-driven capability-matrix freshness checks tied to advertising
    state.
