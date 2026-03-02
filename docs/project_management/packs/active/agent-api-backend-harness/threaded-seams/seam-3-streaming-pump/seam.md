# Threaded Seam Decomposition — SEAM-3 Streaming pump + drain-on-drop

Pack: `docs/project_management/packs/active/agent-api-backend-harness/`

Inputs:
- Seam brief: `docs/project_management/packs/active/agent-api-backend-harness/seam-3-streaming-pump.md`
- Threading (authoritative): `docs/project_management/packs/active/agent-api-backend-harness/threading.md`

## Seam Brief (Restated)

- **Seam ID**: SEAM-3
- **Name**: Shared stream forwarding and draining orchestration
- **Goal / value**: Make “live events + safe completion” behavior consistent across backends, including the critical invariant: if a consumer drops the universal events stream, the backend stream is still drained (to avoid deadlocks/cancellation).
- **Type**: risk
- **Scope**
  - In:
    - A shared orchestration loop that:
      - forwards mapped/bounded events while the receiver is alive, and
      - continues draining backend events after receiver drop without forwarding.
    - A shared internal driver split where:
      - the **pump/drainer** (this seam) owns the event sender and drains the backend stream to end, and
      - a separate **completion sender** (SEAM-4) awaits the backend completion future and publishes the completion outcome.
    - Canonical bounded channel sizing guidance and behavior (at minimum: no unbounded buffering).
  - Out:
    - Backend-specific mapping logic (still backend-owned) beyond a hook.
    - Changing DR-0012 finality rules (SEAM-4 owns the canonical gating integration).
- **Primary interfaces (contracts)**
  - Produced (owned): `BH-C04 stream forwarding + drain-on-drop`
  - Consumed (required upstream): `BH-C01 backend harness adapter interface` (SEAM-1)
- **Key invariants / rules**
  - MUST NOT cancel the backend process/stream just because the universal receiver is dropped.
  - MUST keep draining until the backend stream ends.
    - No early-stop (“give up”) escape hatch is permitted under `BH-C04`. If an escape hatch is needed in the future, introduce it as an explicit contract change with pinned tests.
  - MUST apply `crate::bounds` to every forwarded event.
  - MUST NOT introduce unbounded buffering (bounded channel + explicit backpressure behavior).
- **Touch surface (code)**
  - `crates/agent_api/src/backend_harness.rs` (shared pump implementation)
  - Existing exemplars to unify (reference-only in this seam; adoption is SEAM-5):
    - `crates/agent_api/src/backends/codex.rs` (`drain_events_while_polling_completion`)
    - `crates/agent_api/src/backends/claude_code.rs` (inline drain/forward loop)
- **Verification**
  - Harness-level tests using a fake stream that:
    - forces receiver drop mid-stream and asserts the backend stream is still fully drained, and
    - asserts at least one event can be forwarded before stream finality (“live” behavior).

## Slicing Strategy

**Risk-first / dependency-first**: SEAM-3 is the highest-risk behavioral seam in WS-A. Deliver a harness-owned pump in small increments, pin the receiver-drop semantics explicitly, and require harness-level regression tests before any backend adoption work (SEAM-5).

## Vertical Slices

- **S1 — Extract shared “drain + forward” pump primitive (scaffold `BH-C04`)**
  - File: `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-3-streaming-pump/slice-1-bh-c04-drain-while-polling-completion.md`
- **S2 — Pin drain-on-drop semantics (forward flag) + finality signaling rule**
  - File: `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-3-streaming-pump/slice-2-bh-c04-drain-on-drop-semantics.md`
- **S3 — Harness-layer pump tests (fake stream + receiver drop regression)**
  - File: `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-3-streaming-pump/slice-3-streaming-pump-unit-tests.md`

## Threading Alignment (mandatory)

- **Contracts produced (owned)**:
  - `BH-C04 stream forwarding + drain-on-drop`: implemented in `crates/agent_api/src/backend_harness.rs` as the harness-owned stream pump (introduced in Slice S1; semantics pinned in Slice S2; regression tests in Slice S3).
- **Contracts consumed**:
  - `BH-C01 backend harness adapter interface` (SEAM-1): provides the pinned “typed backend stream + mapping hook” shape that the pump consumes (the backend completion future is owned/awaited by SEAM-4’s completion sender).
- **Dependency edges honored**:
  - `SEAM-1 blocks SEAM-3`: all slices assume the `BH-C01` contract has pinned the typed stream + completion + mapping touch surface.
  - `SEAM-3 blocks SEAM-5`: S1/S2 deliver the shared pump so backend adoption can reuse it (no per-backend drain loops).
- **Parallelization notes**:
  - What can proceed now: SEAM-3 work in WS-A after SEAM-1 is landed (and preferably after SEAM-2 to avoid merge conflicts per `threading.md` critical path).
  - What must wait: SEAM-5 backend adoption should wait for S2 + S3 to land, so semantics are pinned and tested before migration.

## Integration suggestions (explicitly out-of-scope for SEAM-3 tasking)

- In SEAM-5, delete backend-local drain loops and route through the harness pump (treat any semantic diffs as bugs, not “compat exceptions”).
- In SEAM-4, treat the pump’s **finality signal** and drain-on-drop semantics as upstream input contracts (no re-definition) and pin gating behavior with a harness regression test.
