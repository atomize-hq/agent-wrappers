# Threading — Explicit cancellation (`agent_api`)

This section makes coupling explicit: contracts/interfaces, dependency edges, and sequencing.

## Contract registry

- **CA-C01 — Public cancellation surface**
  - **Type**: API (public `agent_api` Rust surface)
  - **Definition**: A caller can obtain an explicit cancellation handle alongside a run handle and
    call `cancel()` to request best-effort termination, yielding a pinned completion error outcome.
  - **Owner seam**: SEAM-1

- **CA-C02 — Cancellation driver semantics**
  - **Type**: policy
  - **Definition**: Cancellation must not violate drain-on-drop; cancellation is implemented as a
    separate signal observed by both the pump/drainer and the completion sender.
  - **Owner seam**: SEAM-2

- **CA-C03 — Backend termination contract**
  - **Type**: policy
  - **Definition**: Built-in backends must provide best-effort termination behavior when
    cancellation is requested (best-effort termination of the spawned CLI process).
  - **Owner seam**: SEAM-3

## Critical path

`SEAM-1 (contract)` → `SEAM-2 (harness wiring)` → `SEAM-3 (backend hooks)` → `SEAM-4 (tests)`
