# Contradictions Remediator Log

Generated (local): 2026-02-23

Input report: `contradictions-audit.report.json`

Audit scope (from report): `docs/project_management/packs/active/agent-api-backend-harness/**`

## Triage

- `CX-0001` (critical): `BH-C04` drain-on-drop semantics — early-stop (“give up”) allowed vs forbidden.
  - Resolution type: **Single truth** (forbid early-stop under `BH-C04`).
- `CX-0002` (blocker): `BH-C05` consumer-drop escape hatch vs handle builder ordering (pump/drain vs completion send).
  - Resolution type: **Scope clarification + responsibility split** (make the lifecycle split explicit in the handle-builder slice).

## CX-0001 — Fixed

### Contradiction (restated)
`BH-C04` drain-on-drop semantics were described as both:
- “drain to stream end, regardless” and
- “drain to stream end, unless a justified explicit early-stop (‘give up’) condition exists”.

These cannot both be true without a pinned, concrete definition of the early-stop condition.

### Evidence used (truth-finding)

- Executable behavior: existing backend adapters fully drain the backend stream after the consumer drops the universal events stream (no early-stop path).
  - Codex adapter drains to stream end and flips a forward-flag off on send failure: `crates/agent_api/src/backends/codex.rs:429`–`469`.
  - Claude adapter drains to stream end and flips a forward-flag off on send failure: `crates/agent_api/src/backends/claude_code.rs:210`–`279`.
- ADR intent: the harness MUST “continue draining on receiver drop” to avoid cancellation/deadlocks: `docs/adr/0013-agent-api-backend-harness.md:71`–`75`.

### Doc changes applied

- Removed the unscoped “give up” allowance and pinned `BH-C04` to full drain-to-end.
  - `docs/project_management/packs/active/agent-api-backend-harness/seam-3-streaming-pump.md:24`
  - `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-3-streaming-pump/seam.md:28`

### Resulting single truth

- Under `BH-C04`, the pump/drainer **MUST** keep draining until the backend stream ends; no early-stop escape hatch exists in the contract.

## CX-0002 — Fixed

### Contradiction (restated)
`BH-C05` docs required a consumer-drop path where completion may resolve once backend completion is ready “while draining continues in the background”, but the canonical handle builder slice described a driver that “runs the pump and then sends completion” (implying completion only after draining completes).

### Evidence used (truth-finding)

- Executable truth: the canonical gate builder explicitly allows a “consumer drop” escape hatch by signaling “done” when the `events` stream is dropped:
  - `crates/agent_api/src/run_handle_gate.rs:12`–`35` (completion waits for `events_done_rx`)
  - `crates/agent_api/src/run_handle_gate.rs:62`–`65` (`Drop` signals done)
- Pack requirement: regression tests explicitly require the consumer-drop escape hatch while draining continues:
  - `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-4-completion-gating/slice-3-bh-c05-gating-regression-tests.md:6`–`9`

### Doc changes applied

- Clarified the canonical handle-builder lifecycle so it can satisfy the consumer-drop requirement without violating drain-to-end:
  - `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-4-completion-gating/slice-2-bh-c05-canonical-handle-builder.md:32`
  - Change: replaced “run pump then send completion” with an explicit split:
    - pump/drainer owns the event sender and drains to stream end (dropping sender only at finality),
    - completion sender publishes the completion outcome as soon as backend completion is ready (independent of draining), leaving DR-0012 gating to control observability.

### Resulting scoped truth

- Completion outcome production (oneshot send) is **independent** of draining.
- Completion *observability* remains gated (DR-0012) on stream finality, **unless** the consumer drops the events stream, in which case completion may resolve once the backend completion outcome is ready while draining continues.

## Decisions introduced

- None.

## Notes / follow-ups (out of scope for this remediation)

- The normative run-protocol spec in `docs/specs/universal-agent-api/run-protocol-spec.md` does not explicitly spell out the consumer-drop escape hatch that `run_handle_gate` implements. If you want, I can run a broader contradictions audit against `docs/specs/**` and `docs/adr/**` to verify DR-0012 is consistently specified.

