# S2b — Claude write hooks + fail-closed gating

- **User/system value**: Expose universal Claude MCP write operations only when explicitly enabled and advertised, while reusing the bounded runner from S1 and preserving fail-closed behavior.
- **Scope (in/out)**:
  - In:
    - `ClaudeCodeBackend::{mcp_add,mcp_remove}` hook implementations.
    - Fail-closed capability gating using backend `capabilities().ids`.
    - Reuse of the S1 runner for process context precedence, isolated-home injection, bounded output, and drift classification.
    - Early `InvalidRequest` return for unsupported Claude URL bearer-token configuration before any spawn occurs.
  - Out:
    - Public config/capability advertisement changes owned by SEAM-2.
    - Hermetic fake-binary integration tests owned by SEAM-5.
- **Acceptance criteria**:
  - When write capability ids are absent, `mcp_add` and `mcp_remove` return `UnsupportedCapability` without spawning a process.
  - When enabled and advertised on pinned supported targets, hooks spawn the exact argv from S2a and return bounded `Ok(output)` even on non-zero exit status.
  - `request.context.env` remains process-only and overrides backend-config env plus isolated-home injection.
  - MCP management stdout/stderr stays outside the run event pipeline (MM-C02).
- **Dependencies**:
  - S2a argv builders.
  - S1b runner and S1c drift classifier from `slice-1-read-ops/`.
  - MM-C01, MM-C03, MM-C04, MM-C06, MM-C07, MM-C09 from `threading.md`.
- **Verification**:
  - `cargo test -p agent_api --features claude_code`
- **Rollout/safety**:
  - Safe by default: hooks remain unreachable when `ClaudeCodeBackendConfig.allow_mcp_write == false` or the capability is unadvertised.

## Atomic Tasks (moved from S2)

#### S2.T2 — Implement `mcp_add` and `mcp_remove` hooks (write-gated, fail closed)

- **Outcome**: Claude backend supports MCP write operations with pinned gating and execution semantics.
- **Files**:
  - `crates/agent_api/src/backends/claude_code.rs`
  - `crates/agent_api/src/backends/claude_code/mcp_management.rs`

Checklist:
- Implement:
  - Add `mcp_add` / `mcp_remove` hook methods and forward to the Claude MCP helper module.
  - Enforce fail-closed gating inside each hook:
    - if `self.capabilities()` does not contain the op capability id, return `UnsupportedCapability`.
  - Reuse the S1 runner for bounded capture, timeout handling, process context precedence, isolated-home env injection, and drift classification.
  - Keep `AgentWrapperMcpAddTransport::Stdio.env` mapped to repeated `--env KEY=VALUE` argv, while `request.context.env` only affects the spawned process environment.
  - Return `InvalidRequest` for `Url { bearer_token_env_var: Some(_) }` before spawning any subprocess.
- Test:
  - Run `cargo test -p agent_api --features claude_code`.
- Validate:
  - Confirm write hooks do not emit run events and do not rely on capability-matrix generation for truth.
