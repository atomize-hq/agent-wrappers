### S1a — Claude capability surface and supported-key alignment

- **User/system value**: Claude advertises `agent_api.exec.add_dirs.v1` through the same
  authoritative capability and allowlist surfaces that R0 gating already trusts.
- **Scope (in/out)**:
  - In:
    - Add `agent_api.exec.add_dirs.v1` to Claude `supported_extension_keys()`.
    - Add the same key to Claude `capabilities().ids`.
    - Add backend-local assertions that both surfaces stay in sync.
  - Out:
    - Policy extraction or normalization logic.
    - Fresh-run argv mapping.
    - Contract doc wording beyond references needed by the tests.
- **Acceptance criteria**:
  - Default Claude backend construction reports `agent_api.exec.add_dirs.v1` in capabilities.
  - Claude harness supported-key allowlists report the same key.
  - Capability tests fail if either surface drops or diverges on the key.
- **Dependencies**:
  - Consumes `AD-C01`.
  - Must stack with `S1b` before merge so Claude never advertises an unhandled key in isolation.
- **Verification**:
  - Backend capability assertions in `crates/agent_api/src/backends/claude_code/tests/capabilities.rs`
  - `cargo test -p agent_api claude_code`
- **Rollout/safety**:
  - Keep capability publication and allowlist changes as one atomic edit set.
  - If this work is developed separately, do not merge it ahead of `S1b`.

#### S1a.T1 — Publish add-dir support in Claude capability surfaces

- **Outcome**: the Claude backend exposes the add-dir extension key consistently anywhere
  unsupported-key gating inspects Claude support.
- **Files**:
  - `crates/agent_api/src/backends/claude_code/mod.rs`
  - `crates/agent_api/src/backends/claude_code/backend.rs`
  - `crates/agent_api/src/backends/claude_code/tests/capabilities.rs`

Checklist:
- Implement:
  - Add `agent_api.exec.add_dirs.v1` to the Claude-supported extension key list.
  - Add the same key to Claude capability reporting with no backend-local opt-in branch.
- Test:
  - Extend capability assertions to check for the new extension id.
  - Assert capability ids and supported-key surfaces stay aligned for the Claude backend.
- Validate:
  - Confirm the change does not introduce a second source of truth for supported extension keys.
  - Confirm this sub-slice remains merge-blocked on `S1b`.
