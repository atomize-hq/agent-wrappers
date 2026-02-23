# S2 — Adoption conformance: update backend tests + add wiring guards

- **User/system value**: Ensures the migrations do not silently bypass the harness (re-introducing duplicated invariants), and keeps test coverage aligned with the threading ownership model (harness invariants tested in the harness; backend mapping tested in backend modules).
- **Scope (in/out)**:
  - In:
    - Update backend-local tests to reflect the “harness owns invariants” architecture.
    - Add small “wiring guards” that fail loudly if a backend regresses into re-implementing harness-owned invariants.
  - Out:
    - Re-testing invariant semantics already covered by upstream seams:
      - `BH-C02` / `BH-C03` tests live in SEAM-2.
      - `BH-C04` drain-on-drop tests live in SEAM-3.
      - `BH-C05` completion gating tests live in SEAM-4.
- **Acceptance criteria**:
  - Backend tests remain focused on backend-owned behavior (mapping, capability reporting).
  - There is a lightweight regression check that both backends route through the harness entrypoint (without requiring spawning real CLIs in unit tests).
- **Dependencies**:
  - SEAM-1..SEAM-4 landed (so adapter and harness entrypoints exist).
- **Verification**:
  - `cargo test -p agent_api --features codex`
  - `cargo test -p agent_api --features claude_code`
  - `cargo test -p agent_api --features codex,claude_code`

## Atomic Tasks

#### S2.T1 — Add compile-time “adapter implements harness contract” checks for Codex and Claude

- **Outcome**: A minimal test that fails if the backend-specific adapter types drift away from the `BH-C01` contract shape.
- **Inputs/outputs**:
  - Output: `crates/agent_api/src/backends/codex/tests.rs`
  - Output: `crates/agent_api/src/backends/claude_code/tests.rs`
- **Implementation notes**:
  - Prefer a compile-time-only trait bound assertion (no spawning).
  - Keep these tests intentionally tiny so they do not become another “toy adapter” (SEAM-1 owns the contract smoke adapter).
- **Acceptance criteria**:
  - A backend adapter type is referenced in each backend’s test module with a trait bound that must compile.

Checklist:
- Add a `fn assert_impl<T: BackendHarnessAdapter>() {}`-style guard (symbol names per `BH-C01`).
- Ensure the tests compile under feature-flag combinations.

#### S2.T2 — Ensure backend-local tests no longer cover harness-owned pump/gating semantics

- **Outcome**: Test ownership is aligned with threading to avoid duplicated assertions and contradictory fixtures.
- **Inputs/outputs**:
  - Output: `crates/agent_api/src/backends/codex/tests.rs` (remove/adjust any pump/drain helper tests)
  - Output: `crates/agent_api/src/backends/claude_code/tests.rs` (if any such tests exist in the future)
- **Implementation notes**:
  - If a backend needs to assert that “bounds are enforced” or “completion is gated”, do so by asserting it routes through the harness (T1) and rely on harness tests for semantics.
- **Acceptance criteria**:
  - Pump/gating semantics are tested once (harness-layer), not per backend.

Checklist:
- Confirm no backend-local tests require the old drain-loop helpers.
- Run the full `agent_api` test matrix with both backends enabled.

#### S2.T3 — Add a small “no duplicated invariants” smoke (optional, if low-cost)

- **Outcome**: A low-effort guardrail that makes it obvious when a backend starts re-introducing copies of harness invariants.
- **Inputs/outputs**:
  - Output: documentation or a small test assertion local to `crates/agent_api/src/backends/*` tests.
- **Implementation notes**:
  - Keep this lightweight (e.g., assert the backend module does not contain a legacy helper behind `#[cfg(test)]` via symbol reachability, or assert the backend run path calls a single “harness entrypoint” function that can be unit-tested without spawning).
  - If this becomes intrusive, omit it and rely on code review + the adapter compile guards (T1).
- **Acceptance criteria**:
  - Guard is non-flaky and does not spawn external processes.

Checklist:
- Decide whether a lightweight guard is feasible given the final harness API surface.
- If feasible, implement and validate it does not require spawning real CLIs.

