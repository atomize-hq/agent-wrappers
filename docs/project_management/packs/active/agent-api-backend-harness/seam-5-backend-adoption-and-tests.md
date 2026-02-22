# SEAM-5 — Adopt harness in existing backends + harness tests

- **Name**: Codex + Claude backend migration to harness, with harness-level test coverage
- **Type**: capability
- **Goal / user value**: Prove the harness is viable and reduces duplication by migrating the two existing built-in backends while preserving behavior, and by adding harness tests for shared invariants.
- **Scope**
  - In:
    - Refactor:
      - `crates/agent_api/src/backends/codex.rs`
      - `crates/agent_api/src/backends/claude_code.rs`
      to delegate glue/invariants to the harness.
    - Keep backend-specific mapping/adapter logic in backend-owned modules (e.g. Codex’s `backends/codex/mapping.rs`).
    - Add harness unit tests that are backend-agnostic (fake backend stream/completion).
  - Out:
    - Changing capability IDs or extension keys.
    - Large-scale reorganization of wrapper crates.
- **Primary interfaces (contracts)**
  - Inputs:
    - Harness contract and helpers from `SEAM-1`..`SEAM-4`
    - Existing backend config structs and wrapper clients
  - Outputs:
    - Behavior-equivalent `AgentWrapperRunHandle` implementations for Codex and Claude, now built via the harness.
    - Harness test suite that prevents future drift.
- **Key invariants / rules**:
  - “No behavior change” intent relative to ADR-0013’s user contract: only internal refactor.
  - Every forwarded event MUST pass through bounds enforcement and redaction rules.
- **Dependencies**
  - Blocks:
    - Future onboarding work: once these migrations land, new backends should be required to use the harness by convention.
  - Blocked by:
    - `SEAM-1`..`SEAM-4` — the harness primitives must exist first.
- **Touch surface**:
  - `crates/agent_api/src/backends/codex.rs`
  - `crates/agent_api/src/backends/claude_code.rs`
  - `crates/agent_api/src/backend_harness.rs` (new)
  - Harness tests (location TBD; likely `crates/agent_api/src/backend_harness/tests.rs` or `crates/agent_api/tests/*`)
- **Verification**:
  - Run existing backend tests and add new harness tests for:
    - env merge precedence
    - fail-closed unknown extension key behavior
    - drain-on-drop behavior
    - DR-0012 completion gating behavior
- **Risks / unknowns**
  - Risk: Migration breaks subtle semantics (e.g., what errors are emitted as events vs returned, ordering differences, or default behaviors).
  - De-risk plan: migrate one backend first (Codex has an explicit helper), lock in behavior with tests, then migrate the second backend.
- **Rollout / safety**:
  - Roll out as a refactor PR with focused review, using tests as the acceptance gate.

## Downstream decomposition prompt

Decompose into slices that (1) migrate Codex backend to the harness with zero behavior change, (2) migrate Claude backend, and (3) add harness-wide tests that both backends implicitly rely on.

