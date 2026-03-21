### S2c — Canonical conformance doc and minimal drift guards

- **User/system value**: Claude-owned add-dir ordering and runtime-rejection truth lives in one
  canonical place, with only the smallest local regression hooks needed to catch drift.
- **Scope (in/out)**:
  - In:
    - Final canonical doc clauses for selector-branch placement and runtime-rejection posture.
    - Minimal Claude-only regression hooks guarding ordering or safe-message drift.
    - Seam-local closeout limited to Claude-owned conformance surfaces.
  - Out:
    - New selector-ordering logic.
    - New runtime rejection classification logic.
    - Capability-matrix regeneration and exhaustive fake-runtime coverage owned by SEAM-5.
- **Acceptance criteria**:
  - The canonical Claude mapping doc is sufficient for SEAM-5 to derive branch-specific and
    runtime-rejection tests.
  - Backend-local regression hooks fail if add-dir ordering or runtime error posture drifts.
  - No duplicate source of truth is introduced outside the canonical doc and minimal local
    assertions.
- **Dependencies**:
  - Blocked by: `S2a`, `S2b`, `AD-C06`
  - Unblocks: SEAM-5 seam-level integration closeout
- **Verification**:
  - Review doc text against emitted argv and runtime error behavior.
  - `cargo test -p agent_api claude_code`
- **Rollout/safety**:
  - Keep this subslice limited to Claude-owned conformance surfaces.
  - Do not regenerate `docs/specs/universal-agent-api/capability-matrix.md` here.

#### S2.T3 — Finalize Claude-owned conformance docs and seam-local regression hooks

- **Outcome**: the Claude backend’s normative mapping doc and local regression hooks reflect the
  final AD-C06 truth without absorbing SEAM-5’s shared integration work.
- **Files**:
  - `docs/specs/claude-code-session-mapping-contract.md`
  - `crates/agent_api/src/backends/claude_code/tests/backend_contract.rs`

Checklist:
- Implement:
  - Finalize doc text for branch-specific add-dir placement and runtime rejection parity.
  - Add only the minimal backend-contract assertions needed for a Claude-local drift guard.
- Test:
  - Run the Claude backend test slice after the doc-backed changes land together.
- Validate:
  - Ensure SEAM-5 still owns capability-matrix regeneration and exhaustive fake-runtime coverage.
  - Remove stale planning notes once the canonical doc is updated.
