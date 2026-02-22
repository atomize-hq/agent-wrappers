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

- **Contract ID**: `BH-C03 env merge precedence`
  - **Type**: policy
  - **Owner seam**: SEAM-2
  - **Consumers (seams)**: SEAM-5
  - **Definition**: Deterministic env precedence: backend config env < request env.

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

