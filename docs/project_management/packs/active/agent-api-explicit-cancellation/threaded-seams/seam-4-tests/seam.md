# SEAM-4 — Tests

## Seam Brief (Restated)

- **Seam ID**: SEAM-4
- **Name**: Tests (explicit cancellation)
- **Goal / value**: Pin explicit cancellation behavior end-to-end so it cannot drift: cancellation triggers best-effort backend termination, completion resolves to the pinned cancellation error, and drain-on-drop / completion gating invariants do not regress (no deadlocks).
- **Type**: integration (behavior + regression safety net)
- **Scope**
  - In:
    - Harness-level integration test using a fake backend process that blocks until killed:
      - calling `cancel()` triggers best-effort termination,
      - `completion` resolves to `Err(AgentWrapperError::Backend { message: "cancelled" })`,
      - cancel-handle lifetime/orthogonality is exercised (drop `events`, then `cancel()`),
      - no raw backend output leaks into events/errors.
    - Regression test: drop events receiver without calling cancel:
      - draining continues,
      - completion gating semantics remain correct (no deadlocks).
  - Out:
    - Defining public API surface or semantics (SEAM-1).
    - Implementing harness cancellation driver semantics (SEAM-2).
    - Implementing built-in backend termination hooks / capability advertisement (SEAM-3).
- **Touch surface**:
  - Integration tests: `crates/agent_api/tests/*.rs`
  - Test binaries/fixtures (fake processes):
    - `crates/agent_api/src/bin/fake_codex_stream_exec_scenarios_agent_api.rs`
    - (optional) `crates/agent_api/src/bin/fake_claude_stream_json_agent_api.rs`
- **Verification**:
  - `cargo test -p agent_api` with relevant feature flags (at least `--features codex` for the fake Codex process path).
  - Tests assert pinned strings and do not rely on nondeterministic timing beyond the pinned timeouts
    defined in `S1` / `S2` (`slice-1-explicit-cancel-integration.md` and `slice-2-drop-regression.md`).
- **Threading constraints**
  - Upstream blockers:
    - `CA-C01` (SEAM-1): `run_control(...)` surface + pinned completion error `"cancelled"`.
    - `CA-C02` (SEAM-2): cancellation driver semantics (pump + completion sender + drain/finality invariants).
    - `CA-C03` (SEAM-3): built-in backend termination hooks + capability advertisement.
  - Downstream blocked seams: none (terminal seam)
  - Contracts produced (owned): none (tests only)
  - Contracts consumed:
    - `CA-C01`, `CA-C02`, `CA-C03`

## Slice index

- `S1` → `slice-1-explicit-cancel-integration.md`: End-to-end explicit cancellation integration test with a fake blocking process (best-effort termination + pinned completion error + no leaks).
- `S2` → `slice-2-drop-regression.md`: Regression test for drop-based cancellation/drain semantics (no deadlocks; completion gating correct).

## Threading Alignment (mandatory)

- **Contracts produced (owned)**:
  - (none — SEAM-4 pins behavior via tests)
- **Contracts consumed**:
  - `CA-C01` (SEAM-1): `run_control(...)` API + pinned `"cancelled"` completion error
    - Verified by: `S1` (completion error + cancel handle path)
  - `CA-C02` (SEAM-2): cancellation driver semantics + drain-on-drop invariants
    - Verified by: `S1` (cancel vs completion race + stream finality) and `S2` (drop receiver regression)
  - `CA-C03` (SEAM-3): built-in backend termination behavior on cancellation
    - Verified by: `S1` (fake blocking process is terminated best-effort after cancel)
- **Dependency edges honored**:
  - `SEAM-1 (contract)` → `SEAM-2 (harness wiring)` → `SEAM-3 (backend hooks)` → `SEAM-4 (tests)`
    - SEAM-4 does not introduce new behavior; it only asserts the cross-seam outcome.
- **Parallelization notes**:
  - What can proceed now:
    - Test scaffolding (helpers, fake process scenarios) can be prepared early, but assertions must match pinned CA-C01 semantics.
  - What must wait:
    - `S1` must wait until `run_control(...)` exists (SEAM-1/2) and built-in backends implement cancellation + termination hooks (SEAM-3).
