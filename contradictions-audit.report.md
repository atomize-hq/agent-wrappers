# Contradictions Audit Report

## Meta
- Generated at: 2026-02-23T00:52:31Z
- Files audited: 28 (under `docs/project_management/packs/active/agent-api-backend-harness/`)
- Scan used: yes (`/tmp/contradictions-audit.scan.XXXXXX.json`; 0 candidate keys)

## Summary
- Total issues: 2
- By severity: blocker=1, critical=1, major=0, minor=0
- High-confidence contradictions: 1

## Issue index
| ID | Severity | Confidence | Type | Subject | Files |
|---|---|---|---|---|---|
| CX-0001 | critical | high | behavior | BH-C04 drain-on-drop: early-stop condition allowed vs forbidden | `seam-3-streaming-pump.md`; `slice-2-bh-c04-drain-on-drop-semantics.md` |
| CX-0002 | blocker | medium | scope_mismatch | BH-C05 consumer-drop escape hatch vs harness driver ordering (pump/drain vs completion send) | `slice-2-bh-c05-canonical-handle-builder.md`; `slice-3-bh-c05-gating-regression-tests.md` |

## Issues

### CX-0001 — Drain early-stop condition is both allowed and forbidden
- Severity: critical
- Confidence: high
- Type: behavior
- Subject: BH-C04 drain-on-drop: early-stop condition allowed vs forbidden
- Scope: environment=all; version=unknown; feature_flag=unknown; timeline=planned
- Statement A: `docs/project_management/packs/active/agent-api-backend-harness/seam-3-streaming-pump.md:25-26`
  - Excerpt: “MUST keep draining until backend stream ends (or until a defined ‘give up’ condition …)”
- Statement B: `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-3-streaming-pump/slice-2-bh-c04-drain-on-drop-semantics.md:6-10`
  - Excerpt: “... keep draining the backend stream until end regardless.”
- Why this conflicts: One statement explicitly permits a justified early-stop (“give up”) draining condition; the other explicitly forbids it (“until end regardless”). Without an explicit scope qualifier that explains when early-stop is permitted, these requirements are mutually incompatible.
- What must be true:
  - Decide whether BH-C04 permits any early-stop condition at all.
  - If yes, define the stop condition(s) concretely (trigger + guarantees preserved + observability implications) and update all BH-C04 descriptions to match.
- Suggested evidence order:
  - codebase
  - tests
  - runtime-config
  - git-history
  - other-docs
  - external
  - decision

### CX-0002 — “Run pump then send completion” conflicts with “completion may resolve while draining continues”
- Severity: blocker
- Confidence: medium
- Type: scope_mismatch
- Subject: BH-C05 consumer-drop escape hatch vs harness driver ordering (pump/drain vs completion send)
- Scope: environment=all; version=unknown; feature_flag=unknown; timeline=planned
- Statement A: `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-4-completion-gating/slice-2-bh-c05-canonical-handle-builder.md:33-37`
  - Excerpt: “... a harness driver task that runs the SEAM-3 pump and then sends completion.”
- Statement B: `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-4-completion-gating/slice-3-bh-c05-gating-regression-tests.md:6-9`
  - Excerpt: “Consumer-drop path: ... completion may resolve once the backend completion is ready (while draining continues in the background).”
- Why this conflicts: If “the SEAM-3 pump” is the unit that performs full drain-to-end behavior, then a driver that “runs the pump and then sends completion” can only send completion after draining finishes. That contradicts the consumer-drop requirement where completion may resolve while draining is still ongoing. The missing piece is a precise definition of whether “pump” includes the full drain-to-end lifecycle or whether draining can be detached as background work that outlives completion sending.
- What must be true:
  - Clarify whether the SEAM-3 “pump” returns only after backend stream end, or can return earlier while a background drainer continues.
  - Specify when the completion oneshot is sent relative to (a) backend completion readiness, (b) event-sender finality (drop), and (c) continued draining after consumer drop.
  - Specify how the consumer-drop escape hatch is achieved without violating “keep draining” guarantees.
- Suggested evidence order:
  - codebase
  - tests
  - runtime-config
  - git-history
  - other-docs
  - external
  - decision

