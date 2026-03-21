# S1c — Manifest/runtime conflict classifier (fail closed; pinned)

- **User/system value**: Detect “manifest/runtime drift” (the upstream CLI rejects pinned argv as unsupported on an advertised
  target) and fail closed as `Err(Backend)` without mutating advertised capabilities.
- **Scope (in/out)**:
  - In:
    - A narrow classifier for “unknown command/subcommand/flag”-style failures for `mcp` / `get`.
    - Runner integration: when classified as drift, return `Err(Backend)` instead of `Ok(output)`.
    - Pure unit tests for the classifier behavior.
  - Out:
    - Any capability mutation or runtime probing to “fix” drift (explicitly forbidden; fail closed).
- **Acceptance criteria**:
  - Synthetic “unknown command/subcommand” outputs map to `Err(Backend)` in unit tests.
  - “Normal” non-zero exits remain `Ok(output)` (i.e., drift classification is conservative).
  - Error messages are safe and non-echoing (do not include raw stdout/stderr).
- **Dependencies**:
  - `docs/specs/universal-agent-api/mcp-management-spec.md` (target availability + fail-closed behavior).
  - S1b runner.
- **Verification**:
  - `cargo test -p agent_api --features claude_code`
- **Rollout/safety**:
  - Conservative classification only; prefer false negatives (treat as normal `Ok(output)`) over false positives.

## Atomic Tasks (moved from S1)

#### S1.T4 — Implement manifest/runtime conflict classification (fail as `Err(Backend)`, pinned)

- **Outcome**: When a Claude MCP op is invoked but the runtime CLI rejects the pinned argv shape as unsupported on this
  target (manifest drift), return `Err(Backend)` and do not mutate advertised capabilities.
- **Files**:
  - `crates/agent_api/src/backends/claude_code/mcp_management.rs` (or adjacent Claude-only module)

Checklist:
- Implement:
  - Add a small drift classifier used by the runner before returning `Ok(output)`.
  - Keep messages safe and non-echoing (no raw stderr/stdout in the error).
  - Prefer conservative classification (“unknown command/subcommand/flag” patterns only).
- Test:
  - Add pure unit tests for the classifier (no subprocesses).
  - Run `cargo test -p agent_api --features claude_code`.
- Validate:
  - Ensure drift cases do not leak partial output.

