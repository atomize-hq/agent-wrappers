# S1b — Claude MCP command runner (context precedence + timeout + bounded output)

- **User/system value**: Provide a single, reusable execution path for pinned Claude MCP argv with correct MM-C03/MM-C04
  semantics, reused by read ops now (S1) and write ops later (S2).
- **Scope (in/out)**:
  - In:
    - `run_claude_mcp(...)` runner that:
      - resolves the `claude` binary deterministically,
      - applies pinned context precedence (MM-C03),
      - captures stdout/stderr with bounded streaming capture and applies SEAM-1’s bounds enforcement helper (MM-C04),
      - returns `Ok(output)` even on non-zero exit status (pinned),
      - returns `Err(Backend)` only for spawn/wait/timeout/capture failures.
    - Safe errors: `Err(Backend)` messages are redacted and do not echo stdout/stderr.
  - Out:
    - Drift classification integration (S1c).
    - Backend hook wiring + gating tests (S1d).
- **Acceptance criteria**:
  - Runner returns bounded outputs with truncation flags and never buffers unbounded output.
  - Timeout failures return `Err(Backend)` and must not include partial stdout/stderr.
  - Non-zero exit status returns `Ok(output)` (until S1c adds the drift classifier exception).
- **Dependencies**:
  - S1a: `capture_bounded(...)` helper.
  - SEAM-1: output-bounds enforcement helper (MM-C04) + process context contract (MM-C03).
  - SEAM-2: isolated home injection field (`claude_home`) if present (MM-C07).
- **Verification**:
  - `cargo test -p agent_api --features claude_code`
- **Rollout/safety**:
  - Non-run boundary preserved: MCP stdout/stderr never enters the run event pipeline (MM-C02).

## Atomic Tasks (moved from S1)

#### S1.T3 — Implement Claude MCP command runner (context precedence + timeout + bounded output)

- **Outcome**: A single runner used by `mcp_list`/`mcp_get` (and reused by S2) that enforces MM-C03/MM-C04 and pinned
  execution semantics.
- **Files** (suggested):
  - `crates/agent_api/src/backends/claude_code/mcp_management.rs`

Checklist:
- Implement:
  - Resolve binary path deterministically:
    - `ClaudeCodeBackendConfig.binary` (if set) → `CLAUDE_BINARY` env var (if set) → `"claude"`.
  - Apply pinned context precedence:
    - working_dir: `request.context.working_dir` → `config.default_working_dir` → backend default.
    - timeout: `request.context.timeout` → `config.default_timeout` → backend default (or none).
    - env: backend config env + isolated-home injection (SEAM-2) + `request.context.env` overrides win.
  - Ensure `Err(Backend)` for spawn/wait/timeout/capture failures, with safe/redacted error messages.
  - Ensure non-zero exit status returns `Ok(output)` (pinned), subject to S1c’s drift classifier.
- Test:
  - Keep tests pure where possible (env merge + bounds enforcement). Hermetic fake-binary end-to-end tests are owned by SEAM-5.
  - Run `cargo test -p agent_api --features claude_code`.
- Validate:
  - Confirm bounded capture is used and the SEAM-1 enforcement helper applies suffix + flags deterministically.

