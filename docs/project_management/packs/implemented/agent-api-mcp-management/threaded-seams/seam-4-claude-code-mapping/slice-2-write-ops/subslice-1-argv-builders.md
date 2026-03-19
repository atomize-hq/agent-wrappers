# S2a — Claude write-op argv builders

- **User/system value**: Establish deterministic, pinned argv composition for Claude MCP write operations before backend wiring or subprocess execution.
- **Scope (in/out)**:
  - In:
    - Pure argv builders for `claude mcp remove <name>` and typed `claude mcp add ...` mappings.
    - Deterministic `--env KEY=VALUE` ordering for `Stdio`.
    - Early `InvalidRequest` rejection for `Url { bearer_token_env_var: Some(_) }`.
  - Out:
    - Backend hook wiring and capability gating (S2b).
    - Hermetic fake-binary execution tests (SEAM-5).
- **Acceptance criteria**:
  - `claude_mcp_remove_argv(name)` produces byte-for-byte pinned argv for `claude mcp remove <name>`.
  - `claude_mcp_add_argv(name, transport)` maps:
    - `Stdio` to `claude mcp add --transport stdio [--env KEY=VALUE]* <name> <command> [args...]`
    - `Url { bearer_token_env_var: None }` to `claude mcp add --transport http <name> <url>`
    - `Url { bearer_token_env_var: Some(_) }` to `Err(InvalidRequest)` before any subprocess work.
  - `Stdio.env` iteration order is deterministic by key, not map insertion order.
- **Dependencies**:
  - `docs/specs/universal-agent-api/mcp-management-spec.md` (MM-C05/MM-C09 pinned mapping).
  - SEAM-1 validation helper for normalized server names and typed add transport inputs.
- **Verification**:
  - `cargo test -p agent_api --features claude_code`
- **Rollout/safety**:
  - Internal-only helpers; runtime behavior does not change until S2b wires them into the backend hooks.

## Atomic Tasks (moved from S2)

#### S2.T1 — Add pure argv builders for `add/remove` (Claude, pinned)

- **Outcome**: Deterministic argv composition functions for Claude write ops (no subprocess spawning), including pinned rejection behavior for `Url.bearer_token_env_var`.
- **Files**:
  - `crates/agent_api/src/backends/claude_code/mcp_management.rs`

Checklist:
- Implement:
  - Add `claude_mcp_remove_argv(name: &str) -> Vec<OsString>` pinned to `claude mcp remove <name>`.
  - Add `claude_mcp_add_argv(name: &str, transport: ...) -> Result<Vec<OsString>, AgentWrapperError>`.
  - Sort `Stdio.env` keys before emitting repeated `--env KEY=VALUE` args.
  - Keep `Url.bearer_token_env_var` handling local to Claude mapping:
    - `None` -> supported `http` transport mapping.
    - `Some(_)` -> `InvalidRequest` (fail closed).
- Test:
  - Add pure unit tests covering `Stdio`, `Url(None)`, and `Url(Some(_))`.
  - Run `cargo test -p agent_api --features claude_code`.
- Validate:
  - Confirm argv matches the pinned spec exactly and does not add passthrough flags or headers.
