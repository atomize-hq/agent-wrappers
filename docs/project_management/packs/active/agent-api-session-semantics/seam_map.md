# Seam Map — Universal Agent API session semantics (ADR-0015 + ADR-0017)

Primary extraction axis: **integration-first (risk-first)** — the initiative spans multiple crates and two built-in backends; the value is unlocked by pinning explicit contracts (extension keys + facets) and then mapping them safely into each backend without violating Universal Agent API bounds/redaction rules.

## Seams (pruned)

- **SEAM-1 — Typed session/thread id accessors (integration seam)**: Add tiny accessor helpers on typed backend event models (Codex + Claude Code) so downstream crates can extract ids without duplicating match logic.
  - File: `seam-1-typed-session-thread-id-accessors.md`
- **SEAM-2 — Session handle facet emission (integration seam)**: Implement `agent_api.session.handle.v1` facet emission (early `Status` + completion attachment) in `crates/agent_api` built-in backends, with bounds enforcement and tests.
  - File: `seam-2-session-handle-facet-emission.md`
- **SEAM-3 — Session resume extension key (capability seam)**: Implement `agent_api.session.resume.v1` (selectors `"last"` and `"id"`) in built-in backends with closed validation + deterministic CLI mapping + tests.
  - File: `seam-3-session-resume-extension-key.md`
- **SEAM-4 — Session fork extension key (risk/capability seam)**: Implement `agent_api.session.fork.v1` (selectors `"last"` and `"id"`) in built-in backends; Codex requires a headless fork surface (ADR-0015 recommends app-server `thread/fork` + `turn/start`) and may need protocol/client work.
  - File: `seam-4-session-fork-extension-key.md`

## Quick “what ships” view

- After **SEAM-3**, orchestrators can resume “last” or “by id” using a universal extension key (per-backend capability-gated).
- After **SEAM-2**, orchestrators can *discover* the backend-defined id to persist/round-trip for resume-by-id flows.
- After **SEAM-4**, orchestrators can fork sessions via a universal extension key (likely shipping Claude first; Codex fork is riskier).

