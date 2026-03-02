# SEAM-4 — DR-0012 completion gating wiring

- **Name**: Canonical completion gating integration
- **Type**: integration
- **Goal / user value**: Ensure completion resolution obeys DR-0012 finality semantics consistently across backends (completion should not resolve “early” relative to stream finality).
- **Scope**
  - In:
    - Centralize the wiring to `run_handle_gate` so adapter implementations cannot drift.
    - Ensure completion resolution is coupled to stream finality (or consumer drop) in the same way for all harness-driven backends.
  - Out:
    - Changing DR-0012 semantics or the public `AgentWrapperRunHandle` surface.
- **Primary interfaces (contracts)**
  - Inputs:
    - Events receiver (`mpsc::Receiver<AgentWrapperEvent>`)
    - A completion oneshot/future that resolves when the backend completion outcome is ready
  - Outputs:
    - `AgentWrapperRunHandle` built via the canonical gate builder
- **Key invariants / rules**:
  - Completion MUST NOT resolve until stream finality is observed or the consumer has dropped the events stream (as defined by DR-0012 / `run_handle_gate` expectations).
  - The harness must ensure its own internal tasks cannot be prematurely dropped in a way that violates gating.
- **Dependencies**
  - Blocks:
    - `SEAM-5` — backend adoption must use the canonical gating path.
  - Blocked by:
    - `SEAM-1` — harness contract defines where the gate is applied.
    - `SEAM-3` — the pump determines the stream finality signal (sender drop) that gating consumes.
- **Touch surface**:
  - `crates/agent_api/src/run_handle_gate.rs`
  - New harness module integration point(s)
- **Verification**:
  - Harness unit test: completion remains pending until:
    - stream finality is observed (events sender dropped), or
    - the consumer drops the events stream (DR-0012 escape hatch),
    matching the semantics enforced by `run_handle_gate`.
- **Risks / unknowns**
  - Risk: subtly different interpretations between backends today; harness must pick the correct universal interpretation.
  - De-risk plan: codify the intended behavior as a harness test and run it against both migrated backends.
- **Rollout / safety**:
  - Internal-only; enforce via tests and by making the harness the only “blessed” path for new backends.

## Downstream decomposition prompt

Decompose into slices that (1) explicitly document the gating behavior being enforced, (2) centralize the handle construction, and (3) add a regression test that would fail if completion resolves early.
