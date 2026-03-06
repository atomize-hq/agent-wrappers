# S1d — Claude `mcp_list` / `mcp_get` hooks + fail-closed gating tests

- **User/system value**: Expose universal MCP read operations for Claude with pinned argv mapping, capability-driven
  fail-closed behavior, and regression tests to prevent drift.
- **Scope (in/out)**:
  - In:
    - `ClaudeCodeBackend::{mcp_list,mcp_get}` hook implementations wired to the S1 runner.
    - Fail-closed capability gating inside each hook (MM-C01/MM-C09).
    - Unit tests pinning argv mapping and gating behavior (Claude).
  - Out:
    - Hermetic fake-binary end-to-end tests (owned by SEAM-5).
    - Any “probe” behavior on unsupported targets (must remain advertising-driven fail closed).
- **Acceptance criteria**:
  - Hooks do not spawn processes when the capability is unadvertised (`UnsupportedCapability`).
  - On supported targets, hooks spawn pinned argv and return bounded `Ok(output)` regardless of exit status (except drift cases).
  - `mcp_get` remains naturally fail-closed off-target via advertising (pinned `win32-x64` availability).
- **Dependencies**:
  - SEAM-1: hook signatures + validation helper + output-bounds enforcement helper.
  - SEAM-2: capability advertising (read ops) + isolated-home config (`claude_home`).
  - S1a/S1b/S1c: argv helpers + runner + drift classifier.
- **Verification**:
  - `cargo test -p agent_api --features claude_code`
- **Rollout/safety**:
  - Read-only surface; no persistent state mutation required.

## Atomic Tasks (moved from S1)

#### S1.T5 — Implement `mcp_list` and `mcp_get` hooks (fail-closed gating + runner integration)

- **Outcome**: Claude backend supports MCP read operations with pinned gating + execution semantics.
- **Files** (suggested):
  - `crates/agent_api/src/backends/claude_code.rs`
  - `crates/agent_api/src/backends/claude_code/mcp_management.rs`

Checklist:
- Implement:
  - Add the MCP hook methods and forward to the Claude MCP helper module.
  - Enforce gating inside each hook:
    - if `self.capabilities()` does not contain the op capability id, return `UnsupportedCapability`.
  - Avoid upstream probing on unsupported targets; rely on advertising for pinned `win32-x64` availability.
- Test:
  - Run `cargo test -p agent_api --features claude_code`.
- Validate:
  - Confirm MCP stdout/stderr is not emitted as run events (MM-C02).

#### S1.T6 — Add unit tests pinning read-op gating and argv mapping (Claude)

- **Outcome**: Deterministic regression tests preventing drift in `list/get` argv mapping and fail-closed gating behavior.
- **Files** (suggested):
  - `crates/agent_api/src/backends/claude_code/mcp_management.rs` (or adjacent test module)

Checklist:
- Implement:
  - Pin representative argv for both `list` and `get`.
  - Pin fail-closed behavior: invoking `mcp_get` without the advertised capability returns `UnsupportedCapability`.
  - Include drift-classifier regression cases (unknown-command style → `Err(Backend)`; normal non-zero → `Ok(output)`).
- Test:
  - Run `cargo test -p agent_api --features claude_code`.
- Validate:
  - Ensure assertions match the spec, not current upstream behavior guesses.

