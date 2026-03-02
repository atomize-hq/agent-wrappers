# SEAM-2 — Harness cancellation propagation (CA-C02)

## Seam Brief (Restated)

- **Seam ID**: SEAM-2
- **Name**: Harness cancellation propagation (CA-C02)
- **Goal / value**: Wire explicit cancellation into the backend harness run driver so cancellation is orthogonal to receiver drop and does not regress drain-on-drop ([BH-C04](../../seam-2-harness-cancel-propagation.md#bh-c04-drain-on-drop-posture)) or completion gating/finality (DR-0012 / [BH-C05](../../seam-2-harness-cancel-propagation.md#bh-c05-completion-gating-consumer-opt-out-dr-0012)).
- **Type**: risk / integration (runtime driver correctness + safety posture)
- **Scope**
  - In:
    - Introduce a harness-internal cancellation signal observed by both:
      - the pump/drainer task, and
      - the completion sender task.
    - On cancellation:
      - stop forwarding universal events to the consumer,
      - continue draining the typed backend event stream to completion (BH-C04 posture; see [BH-C04](../../seam-2-harness-cancel-propagation.md#bh-c04-drain-on-drop-posture)),
      - request backend termination best-effort (via a hook owned by SEAM-3 / CA-C03),
      - select the pinned cancellation error if cancellation is requested before backend completion,
        while still obeying DR-0012 completion gating (completion timing is not accelerated by cancellation).
    - Provide a harness entrypoint that returns `AgentWrapperRunControl` for backends implementing `run_control(...)`.
  - Out:
    - Public API surface and normative semantics (SEAM-1 / CA-C01).
    - Backend-specific termination implementation (SEAM-3 / CA-C03).
    - Harness-level integration tests involving real/fake blocking processes (SEAM-4).
- **Touch surface**:
  - `crates/agent_api/src/backend_harness/runtime.rs` (pump/drainer + completion sender tasks)
  - `crates/agent_api/src/backend_harness/contract.rs` (crate-private harness contract types if needed)
  - `crates/agent_api/src/run_handle_gate.rs` (completion gating/finality interplay)
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-2-harness-cancel-propagation.md` (pack-local pinned driver model)
- **Verification**:
  - Unit tests local to the harness driver (no external processes) that prove:
    - cancellation stops forwarding and closes the universal event stream,
    - completion resolves to `Err(AgentWrapperError::Backend { message: "cancelled" })` when cancellation is requested before backend completion (value selection; timing still gated by DR-0012),
    - draining continues even after cancellation and even after consumer receiver drop.
  - `make check` + `make clippy` for compilation and lint.
- **Threading constraints**
  - Upstream blockers:
    - `CA-C01` (SEAM-1): public cancel handle types + pinned strings/signatures.
  - Downstream blocked seams:
    - `SEAM-3` (CA-C03): needs the harness cancellation signal to trigger backend termination.
    - `SEAM-4` (tests): depends on SEAM-2/3 runtime wiring.
  - Contracts produced (owned):
    - `CA-C02` — Cancellation driver semantics
  - Contracts consumed:
    - `CA-C01` (SEAM-1): pinned cancellation completion error shape + capability id meaning.
    - `CA-C03` (SEAM-3): “request backend termination” hook implementation for built-in backends.

## Slice index

- `S1` → `slice-1-driver-semantics.md`: Implement CA-C02 cancellation driver semantics (pump + completion sender + drain/finality invariants).
- `S2` → `slice-2-harness-control-entrypoint.md`: Add a harness entrypoint returning `AgentWrapperRunControl` for `run_control(...)` backends (no backend adoption here).

## Threading Alignment (mandatory)

- **Contracts produced (owned)**:
  - `CA-C02` (SEAM-2): cancellation driver semantics
    - Lives in:
      - Pack: `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-2-harness-cancel-propagation.md`
      - Code: `crates/agent_api/src/backend_harness/runtime.rs`
    - Produced by: `S1` (runtime semantics) and `S2` (control entrypoint for backends)
- **Contracts consumed**:
  - `CA-C01` (SEAM-1): cancellation surface + pinned semantics (`"cancelled"`, idempotence)
    - Consumed by: `S1` (pinned completion error) and `S2` (constructing/returning cancel handle)
  - `CA-C03` (SEAM-3): backend termination behavior on cancellation
    - Consumed by: `S1` (invoking a termination request hook), implemented later by SEAM-3.
- **Dependency edges honored**:
  - `SEAM-1 (contract)` → `SEAM-2 (harness wiring)` → `SEAM-3 (backend hooks)` → `SEAM-4 (tests)`
    - This plan explicitly depends on SEAM-1 outputs and does not include SEAM-3 adoption tasks.
- **Parallelization notes**:
  - What can proceed now:
    - `S1` and `S2` can proceed immediately after SEAM-1 lands the required public types + crate-private constructors for cancellation wiring.
  - What must wait:
    - Built-in backend adoption of cancellation (SEAM-3) and the pack’s required integration tests (SEAM-4).
