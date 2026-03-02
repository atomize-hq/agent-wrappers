# Threading — `agent_api` backend harness (ADR-0013)

This section makes coupling explicit: contracts/interfaces, dependency edges, critical path, and workstreams that avoid conflicts.

## Contract registry

- **Contract ID**: `BH-C01 backend harness adapter interface`
  - **Type**: API (internal Rust interface)
  - **Owner seam**: SEAM-1
  - **Consumers (seams)**: SEAM-2, SEAM-3, SEAM-4, SEAM-5
  - **Definition**: The internal interface that a backend adapter must provide to the harness (identity, supported extension keys, spawn, typed event mapping, completion extraction).
  - **Versioning/compat**: Internal-only; changes should be coordinated with backends.

- **Contract ID**: `BH-C02 extension key allowlist + fail-closed validator`
  - **Type**: schema/policy
  - **Owner seam**: SEAM-2
  - **Consumers (seams)**: SEAM-5
  - **Definition**: Unknown extension keys are rejected pre-spawn as `UnsupportedCapability(agent_kind, key)`.

- **Contract ID**: `BH-C03 env merge + timeout derivation`
  - **Type**: policy
  - **Owner seam**: SEAM-2
  - **Consumers (seams)**: SEAM-5
  - **Definition**: Deterministic env + timeout normalization:
    - Env precedence: backend config env < request env.
    - Timeout precedence: backend default timeout < request timeout override (with “absent” preserved explicitly).
    - Effective timeout (normative):
      - Let `request_timeout = request.timeout: Option<Duration>`.
      - Let `default_timeout = backend_default_timeout: Option<Duration>`.
      - Then `effective_timeout` MUST be:
        - `Some(t)` when `request_timeout == Some(t)` (including `t == Duration::ZERO`, which is an explicit “no timeout” request), else
        - `default_timeout` when `request_timeout == None`.
      - Equivalently (Rust): `effective_timeout = request.timeout.or(default_timeout)`.
    - Explicit “no timeout” (`Duration::ZERO`) semantics:
      - If `effective_timeout == Some(Duration::ZERO)`, adapters MUST treat it as “disable timeout”
        (i.e., MUST NOT enforce an immediate timeout due to `0`).
      - This preserves Codex wrapper behavior where `Duration::ZERO` is “no timeout” and avoids
        accidental `tokio::time::timeout(Duration::ZERO, ...)` immediate-failure behavior.

- **Contract ID**: `BH-C04 stream forwarding + drain-on-drop`
  - **Type**: API/policy
  - **Owner seam**: SEAM-3
  - **Consumers (seams)**: SEAM-5
  - **Definition**: Forward bounded events while receiver is alive; if receiver drops, stop forwarding but keep draining the backend stream to avoid deadlocks/cancellation.

- **Contract ID**: `BH-C05 completion gating integration`
  - **Type**: API/policy
  - **Owner seam**: SEAM-4
  - **Consumers (seams)**: SEAM-5
  - **Definition**: Run handle completion is gated per DR-0012 semantics via the canonical gate builder.

## Canonical internal lifecycle handshake (SEAM-3 ↔ SEAM-4)

This pack assumes a **split driver** so completion production is never double-driven and DR-0012’s
consumer-drop escape hatch can resolve completion while draining continues:

- **Pump/drainer** (SEAM-3 / `BH-C04`):
  - Owns the bounded `mpsc::Sender<AgentWrapperEvent>`.
  - Forwards bounded mapped events while the receiver is alive.
  - If the receiver drops, stops forwarding (forward-flag) but **keeps draining** the backend event stream until end.
  - Drops the `Sender` **only at stream finality** (sender drop is the finality signal consumed by `run_handle_gate`).
- **Completion sender** (SEAM-4 / `BH-C05`):
  - Owns and awaits the backend completion future.
  - Sends the completion outcome on a `oneshot` as soon as it is ready (**independent of draining**).
  - Does **not** decide stream finality and does **not** drop the event sender.
- **Gate** (`run_handle_gate::build_gated_run_handle`):
  - Completion observability is gated on (a) sender drop (“finality”) or (b) consumer drop of the events stream.
  - This gating is the only “blessed” path for harness-driven backends.

## Canonical test placement (harness-owned invariants)

- **Unit tests** for pure harness helpers (normalization, env/timeout derivation, allowlist checks, pump semantics):
  - Co-locate as `#[cfg(test)]` in `crates/agent_api/src/backend_harness.rs` (or a sibling internal module).
- **Integration/regression tests** for DR/protocol behaviors that must not drift (e.g., DR-0012):
  - Place in `crates/agent_api/tests/*` (see existing `dr0012_completion_gating.rs`).

## Dependency graph (text)

- `SEAM-1 blocks SEAM-3` because: the streaming pump needs a pinned “stream + completion + mapping” contract shape.
- `SEAM-1 blocks SEAM-4` because: completion gating wiring depends on where the harness constructs the run handle.
- `SEAM-1 blocks SEAM-5` because: backend adoption requires the harness interface to exist.
- `SEAM-2 blocks SEAM-5` because: migrated backends should not re-implement extension/env/timeout invariants.
- `SEAM-3 blocks SEAM-5` because: backend adoption should reuse a shared pump rather than per-backend drain loops.
- `SEAM-4 blocks SEAM-5` because: backend adoption must use the canonical gating path (no per-backend variation).

## Critical path

`SEAM-1 (contract)` → `SEAM-2 (normalization)` → `SEAM-3 (pump)` → `SEAM-4 (gating)` → `SEAM-5 (adoption + tests)`

## Parallelization notes / conflict-safe workstreams

- **WS-A (Harness primitives)**: SEAM-1..SEAM-4; touch surface: `crates/agent_api/src/backend_harness.rs` (+ any small shared helpers).
- **WS-B (Backend adoption)**: SEAM-5; touch surface: `crates/agent_api/src/backends/codex.rs`, `.../claude_code.rs` plus backends’ mapping modules if needed.
- **WS-INT (Integration)**: lands WS-A then WS-B; reconciles behavior to ADR-0013 invariants and runs full test suite.
