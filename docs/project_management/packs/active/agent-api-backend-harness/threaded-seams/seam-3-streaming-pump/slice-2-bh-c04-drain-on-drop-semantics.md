# S2 — Pin drain-on-drop semantics (forward flag) + finality signaling rule

- **User/system value**: Pins the highest-risk semantics explicitly: “stop forwarding when receiver drops, but keep draining,” and makes the finality signal (sender drop) deterministic so SEAM-4 gating has a reliable input.
- **Scope (in/out)**:
  - In:
    - Fully implement `BH-C04 stream forwarding + drain-on-drop` semantics in the harness pump:
      - forward while receiver is alive,
      - detect receiver drop deterministically,
      - disable forwarding after drop (forward-flag),
      - keep draining the backend stream until end regardless.
    - Define the pump’s **finality signaling** rule in one place and document it (when the event `Sender` is dropped relative to stream end).
  - Out:
    - The canonical `run_handle_gate` wiring (SEAM-4).
    - Backend adoption/migration (SEAM-5).
- **Acceptance criteria**:
  - Once the downstream `mpsc::Receiver<AgentWrapperEvent>` is dropped, the pump:
    - stops attempting to send,
    - continues draining the backend stream to completion (no cancellation),
    - does not treat receiver drop as stream finality (sender drop remains reserved for true finality).
  - The pump defines and implements a deterministic **finality signaling** rule that is compatible with SEAM-4’s gating expectations (DR-0012), and this rule is pinned by S3 tests.
  - `crate::bounds` is applied to every forwarded event (and forwarding is never attempted without bounds).
  - The behavior is deterministic and does not depend on hash iteration order or timing races for correctness.
- **Dependencies**:
  - Slice S1: pump scaffold and basic loop.
  - Upstream contract: `BH-C01 backend harness adapter interface` (SEAM-1).
- **Verification**:
  - `cargo test -p agent_api --features codex,claude_code`
  - Harness regression tests from S3 must pass.
- **Rollout/safety**:
  - Treat S2 as the “semantics pin”: avoid changing semantics after S3 lands without updating tests and re-validating against both backends during SEAM-5 adoption.

## Pinned pump semantics (BH-C04) (normative)

This slice pins the exact backpressure + receiver-drop behavior so it is testable and stable.

### Backpressure algorithm while receiver is alive (pinned)

The pump MUST:

- preserve event ordering,
- drop **no** events while the receiver is alive (except as required by universal bounds enforcement),
- apply per-event bounds enforcement before forwarding, and
- apply backpressure by awaiting bounded sends.

Pseudo-code (exact behavior):

```rust
let mut forward = true;
while let Some(outcome) = backend_events.next().await {
    if !forward {
        // receiver is gone; drain without mapping/sending
        continue;
    }

    let mapped: Vec<AgentWrapperEvent> = match outcome {
        Ok(ev) => adapter.map_event(ev),
        Err(err) => vec![error_event(adapter.redact_error(Stream, &err))],
    };

    for e in mapped {
        for bounded in crate::bounds::enforce_event_bounds(e) {
            // IMPORTANT: await send (bounded backpressure).
            if tx.send(bounded).await.is_err() {
                forward = false;
                break;
            }
        }
        if !forward {
            break;
        }
    }
}
// Finality signal: drop tx only after backend stream ends.
drop(tx);
```

Receiver-drop transition (pinned):

- Receiver drop MUST be detected solely via `tx.send(...).await` returning `Err(_)`.
- After the first send failure:
  - set `forward=false`,
  - do not attempt any further sends,
  - continue draining the backend stream to end.

### Bounds interaction (pinned)

- Bounds are enforced only on forwarded events (`forward == true`).
- Once `forward == false`, mapping/bounds work that exists only for forwarding MUST stop; the pump
  drains the typed backend stream without producing any additional universal events.

### Pinned regression tests required by this slice

Add harness-level tests (co-located with the pump in `crates/agent_api/src/backend_harness.rs`):

- `pump_blocks_under_backpressure_until_receiver_polls`:
  - Use an events channel capacity of `1`.
  - Feed a backend stream that produces two mapped events quickly.
  - Assert the pump cannot complete while the receiver is alive but not polling.
  - Then poll the receiver once and assert the pump completes and ordering is preserved.
- `pump_stops_forwarding_after_receiver_drop_but_drains_to_end`:
  - Drop the receiver mid-stream and assert the backend stream is still fully consumed
    (using a counter inside the fake stream), even though no further sends occur.
- `pump_enforces_bounds_before_forwarding`:
  - Use a backend mapping that produces an `AgentWrapperEvent` with `message` larger than the
    universal bound (4096 bytes).
  - Assert the received forwarded event is truncated per `crate::bounds` (ends with `…(truncated)`),
    proving `map_event → enforce_event_bounds → send` ordering.

## Atomic Tasks

#### S2.T1 — Implement receiver-drop detection + forward-flag behavior

- **Outcome**: A pump implementation that deterministically flips from “forwarding” to “draining-only” when the receiver is dropped.
- **Inputs/outputs**:
  - Output: pump implementation updates in `crates/agent_api/src/backend_harness.rs`
- **Implementation notes**:
  - Receiver drop should be detected via bounded sender behavior (send returns closed) rather than out-of-band signaling.
  - Once the forward flag flips to “off,” avoid doing extra work that only exists for forwarding (e.g., mapping/bounds) unless required for drain correctness.
  - Keep draining the backend stream even after forward is off; do not exit early.
- **Acceptance criteria**:
  - Dropping the receiver never stops draining the backend stream.
  - Forwarding attempts stop after receiver drop is detected (no repeated send failures in a tight loop).
- **Test notes**: pinned by S3 receiver-drop regression test(s).
- **Risk/rollback notes**: this is the behavior most likely to regress; keep it simple and test-driven.

Checklist:
- Implement: forward flag + receiver-drop detection path.
- Test: receiver-drop regression tests (S3).
- Validate: `cargo test -p agent_api --features codex,claude_code`.
- Cleanup: document the forward flag semantics next to the code.

#### S2.T2 — Define and document the pump’s finality signaling rule

- **Outcome**: A single, explicit rule for when the pump drops the event `Sender` (the finality signal consumed by `run_handle_gate`) relative to backend stream end and receiver drop.
- **Inputs/outputs**:
  - Output: doc comment + implementation in `crates/agent_api/src/backend_harness.rs`
- **Implementation notes**:
  - The rule must be consistent with the canonical lifecycle handshake in `threading.md`:
    - receiver drop stops forwarding, but is **not** stream finality,
    - sender drop happens **only** when the backend event stream has ended.
  - Choose one deterministic rule and pin it with tests (S3) so SEAM-4 can treat it as an upstream input contract (no re-definition in gating code).
- **Acceptance criteria**:
  - The chosen rule is written down next to the pump implementation and referenced by S3 tests.
  - The rule does not rely on timing accidents (e.g., “usually the stream ends soon”).
- **Test notes**: S3 must include at least one test that would fail if the finality signaling rule changes accidentally.
- **Risk/rollback notes**: if SEAM-4 requires a different rule, change this task first (definition), then update S3 tests, then update SEAM-4 wiring.

Checklist:
- Implement: explicit finality signal rule + code path.
- Test: finality signal regression test(s) (S3).
- Validate: `make clippy`.
- Cleanup: keep the rule local and auditable (no distributed gating logic).

#### S2.T3 — Ensure bounded forwarding cannot deadlock draining

- **Outcome**: Backpressure on the bounded event channel does not prevent the pump from continuing to drain and complete (including after receiver drop).
- **Inputs/outputs**:
  - Output: pump implementation adjustments in `crates/agent_api/src/backend_harness.rs`
- **Implementation notes**:
  - Avoid designs where a full channel stalls the pump forever while the backend stream continues producing (deadlock risk).
  - Define what happens under backpressure (e.g., await send while receiver is alive; once dropped, stop forwarding and drain).
  - Keep behavior explicit and testable; avoid “best effort” that silently drops events while receiver is alive unless explicitly justified.
- **Acceptance criteria**:
  - With a slow receiver, the pump behavior is well-defined and does not violate drain-on-drop semantics.
  - Receiver drop always allows draining to proceed (forward flag off).
- **Test notes**: can be covered with a small “bounded channel backpressure” test in S3 (optional if too costly).
- **Risk/rollback notes**: if this becomes complex, capture the intended behavior in a comment and a targeted test before optimizing.

Checklist:
- Implement: backpressure behavior that preserves drain correctness.
- Test: optional backpressure regression test (S3).
- Validate: `cargo test -p agent_api --features codex,claude_code`.
- Cleanup: keep the behavior minimal; revisit tuning during SEAM-5 adoption if needed.
