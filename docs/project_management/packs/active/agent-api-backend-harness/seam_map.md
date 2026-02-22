# Seam Map — `agent_api` backend harness (ADR-0013)

Primary extraction axis: **integration-first (risk-first)** — the value is enabling consistent, safe onboarding across many CLI backends via explicit internal contracts and shared invariants.

## Seams (pruned)

- **SEAM-1 — Harness contract-definition (integration seam)**: Define the internal harness interface between `agent_api` and per-backend adapters (what a backend must provide; what the harness guarantees).
  - File: `seam-1-harness-contract.md`
- **SEAM-2 — Canonical request normalization + validation (integration seam)**: Centralize fail-closed extension validation, env merge precedence, timeout wrapping rules, and other shared request invariants.
  - File: `seam-2-request-normalization.md`
- **SEAM-3 — Streaming pump + drain-on-drop semantics (risk seam)**: Provide the shared “forward while receiver is alive; keep draining when dropped” orchestration for backend event streams and completion polling.
  - File: `seam-3-streaming-pump.md`
- **SEAM-4 — DR-0012 completion gating wiring (integration seam)**: Centralize the completion gating integration so backends cannot drift on finality ordering semantics.
  - File: `seam-4-completion-gating.md`
- **SEAM-5 — Adopt harness in existing backends + harness tests (capability seam)**: Refactor `codex` and `claude_code` backends to use the harness and add harness-level tests proving invariants.
  - File: `seam-5-backend-adoption-and-tests.md`

## Quick “what ships” view

When all seams land, adding a new backend adapter should look like:

1) Implement wrapper crate spawn + typed stream parsing.
2) Implement a thin `agent_api` backend adapter that:
   - advertises capabilities (including supported extension keys), and
   - supplies spawn + event mapping to the harness.
3) Reuse harness-provided validation/env/timeout/bounds/drain/gating behavior.
