# S2c — Claude write-op regression tests

- **User/system value**: Lock the Claude write-op mapping and gating rules to the pinned spec so later refactors cannot silently broaden or drift the behavior.
- **Scope (in/out)**:
  - In:
    - Unit tests pinning `add/remove` argv mapping.
    - Unit tests pinning fail-closed write gating via backend capability ids.
    - Unit tests pinning `Url.bearer_token_env_var` rejection and deterministic `--env` ordering.
  - Out:
    - Fake-binary end-to-end execution tests owned by SEAM-5.
    - Capability advertisement implementation owned by SEAM-2.
- **Acceptance criteria**:
  - Tests fail on any drift in representative `Stdio` / `Url(None)` argv output.
  - Tests fail if write hooks stop returning `UnsupportedCapability` when write capability ids are absent.
  - Tests fail if `Url { bearer_token_env_var: Some(_) }` stops returning `InvalidRequest`.
  - Assertions are spec-driven and do not depend on real `claude` subprocess execution.
- **Dependencies**:
  - S2a argv builders.
  - S2b hook implementations and gating behavior.
  - `docs/specs/universal-agent-api/mcp-management-spec.md` and `threading.md` (MM-C05/MM-C06/MM-C09).
- **Verification**:
  - `cargo test -p agent_api --features claude_code`
- **Rollout/safety**:
  - Tests-only slice; safe to land after S2a/S2b without changing runtime behavior.

## Atomic Tasks (moved from S2)

#### S2.T3 — Add unit tests pinning write-op gating, bearer-token rejection, and argv mapping (Claude)

- **Outcome**: Deterministic regression tests that prevent drift in `add/remove` argv mapping, write gating, and the pinned bearer-token rejection rule.
- **Files**:
  - `crates/agent_api/src/backends/claude_code/mcp_management.rs`

Checklist:
- Implement:
  - Pin representative argv for `Stdio` and `Url { bearer_token_env_var: None }`.
  - Pin deterministic `--env` ordering for multi-entry `Stdio.env`.
  - Pin `UnsupportedCapability` results when write capability ids are absent from `capabilities().ids`.
  - Pin `InvalidRequest` for `Url { bearer_token_env_var: Some(_) }`.
- Test:
  - Run `cargo test -p agent_api --features claude_code`.
- Validate:
  - Keep tests pure/unit-level; do not introduce subprocess execution or SEAM-5 fake-binary responsibilities here.
