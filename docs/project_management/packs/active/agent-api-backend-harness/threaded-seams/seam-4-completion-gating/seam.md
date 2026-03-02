# Threaded Seam Decomposition — SEAM-4 Completion gating wiring

Pack: `docs/project_management/packs/active/agent-api-backend-harness/`

Inputs:
- Seam brief: `docs/project_management/packs/active/agent-api-backend-harness/seam-4-completion-gating.md`
- Threading (authoritative): `docs/project_management/packs/active/agent-api-backend-harness/threading.md`

## Seam Brief (Restated)

- **Seam ID**: SEAM-4
- **Name**: Canonical completion gating integration
- **Goal / value**: Ensure completion resolution obeys DR-0012 finality semantics consistently across backends (completion should not resolve “early” relative to stream finality).
- **Type**: integration
- **Scope**
  - In:
    - Centralize the wiring to `run_handle_gate` so adapter implementations cannot drift.
    - Ensure completion resolution is coupled to stream finality (or consumer drop) in the same way for all harness-driven backends.
  - Out:
    - Changing DR-0012 semantics or the public `AgentWrapperRunHandle` surface.
- **Primary interfaces (contracts)**
  - Produced (owned): `BH-C05 completion gating integration`
  - Consumed (required upstream):
    - `BH-C01 backend harness adapter interface` (SEAM-1)
    - `BH-C04 stream forwarding + drain-on-drop` (SEAM-3) (defines the stream finality signal consumed by gating)
- **Key invariants / rules**
  - Completion MUST NOT resolve until stream finality is observed, unless the consumer has dropped the events stream (DR-0012 permitted escape hatch).
  - The harness must ensure its own internal tasks cannot be prematurely dropped in a way that violates gating (no accidental cancellation).
  - There must be exactly one “blessed” run-handle construction path for harness-driven backends.
- **Touch surface (code)**
  - `crates/agent_api/src/run_handle_gate.rs`
  - `crates/agent_api/src/backend_harness.rs` (harness-owned run orchestration; created in SEAM-1)
- **Verification**
  - Harness unit test(s) that prove completion remains pending until:
    - the event stream reaches finality (sender dropped), or
    - the consumer drops the events stream,
    and that this behavior is stable across harness-driven backends (SEAM-5 adoption runs the same assertions).

## Slicing Strategy

**Contract-first / dependency-first**: SEAM-4 is an integration seam that blocks SEAM-5, but is blocked by SEAM-1 (where the harness constructs the run handle) and SEAM-3 (which pins the stream finality signal semantics). First, document and pin the gating contract (`BH-C05`), then ensure the harness has a single canonical handle-construction path, and finally add a regression test that fails if completion resolves early.

## Vertical Slices

- **S1 — Document and pin `BH-C05` gating semantics (DR-0012)**
  - File: `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-4-completion-gating/slice-1-bh-c05-gating-semantics.md`
- **S2 — Centralize run-handle construction in the harness using `run_handle_gate`**
  - File: `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-4-completion-gating/slice-2-bh-c05-canonical-handle-builder.md`
- **S3 — Harness regression tests: completion does not resolve early**
  - File: `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-4-completion-gating/slice-3-bh-c05-gating-regression-tests.md`

## Threading Alignment (mandatory)

- **Contracts produced (owned)**:
  - `BH-C05 completion gating integration`: implemented by a single harness-owned `AgentWrapperRunHandle` construction path that uses `crates/agent_api/src/run_handle_gate.rs` (documented in S1; centralized in S2; regression-tested in S3).
- **Contracts consumed**:
  - `BH-C01 backend harness adapter interface` (SEAM-1): ensures adapters provide `(typed stream, completion future, mapping)` and do not construct `AgentWrapperRunHandle` directly.
  - `BH-C04 stream forwarding + drain-on-drop` (SEAM-3): defines when the event `Sender` is dropped (finality signal) and ensures draining continues even if the consumer drops the event stream.
- **Dependency edges honored**:
  - `SEAM-1 blocks SEAM-4`: all slices assume the harness owns run-handle construction and can enforce a canonical builder.
  - `SEAM-3 blocks SEAM-4`: S2/S3 treat the pump’s finality signal + drain-on-drop semantics as upstream input contracts (no re-definition here).
  - `SEAM-4 blocks SEAM-5`: S2 delivers the canonical gating path so backend adoption cannot drift.
- **Parallelization notes**:
  - What can proceed now: S1 can land early (docs + contract pinning) as soon as `run_handle_gate.rs` exists.
  - What must wait: S2/S3 should land after SEAM-1 has created `backend_harness.rs` and SEAM-3 has pinned finality signaling + drain-on-drop semantics (so tests match reality).

## Integration suggestions (explicitly out-of-scope for SEAM-4 tasking)

- In SEAM-5, delete backend-local handle construction and ensure all migrated backends go through the harness-owned canonical handle builder (and therefore `BH-C05`).
- In WS-INT, run the SEAM-4 regression test(s) against both migrated backends to detect semantic drift immediately (treat diffs as bugs, not acceptable “backend variance”).
