# S2 — Write ops (`add/remove`) mapping + typed transports + write gating

- **User/system value**: Enables universal MCP management write operations for the Codex backend when explicitly enabled,
  with pinned typed transport mapping (MM-C05), isolation support (MM-C07), and bounded outputs (MM-C04), while remaining
  safe-by-default (MM-C06).
- **Scope (in/out)**:
  - In:
    - Implement `CodexBackend::{mcp_add,mcp_remove}` mapping to pinned argv:
      - `remove` → `codex mcp remove <name>`
      - `add`:
        - `Stdio` → `codex mcp add <name> [--env KEY=VALUE]* -- <argv...>`
        - `Url` → `codex mcp add <name> --url <url> [--bearer-token-env-var <ENV_VAR>]`
    - Enforce fail-closed gating:
      - if `agent_api.tools.mcp.{add,remove}.v1` is not advertised (including `allow_mcp_write=false`), return `UnsupportedCapability`.
    - Reuse S1 runner for context precedence + bounded output + timeout behavior.
  - Out:
    - Capability advertising + `allow_mcp_write` enablement (SEAM-2).
    - Cross-backend hermetic fake-binary integration tests (SEAM-5).
- **Acceptance criteria**:
  - `mcp_remove` invokes `codex mcp remove <name>` and returns bounded `AgentWrapperMcpCommandOutput`.
  - `mcp_add` maps typed transports exactly (no argv pass-through):
    - `Stdio`: repeats `--env KEY=VALUE` deterministically (map iteration order), uses `--` separator, and uses
      `argv = command + args` (concatenation).
    - `Url`: `--url <url>` and optional `--bearer-token-env-var <ENV_VAR>`.
  - Both operations:
    - fail closed with `UnsupportedCapability` when write capability ids are unadvertised / disabled (MM-C06),
    - honor isolated homes via backend config, but allow request env overrides to win (MM-C07/MM-C03),
    - return `Ok(output)` even on non-zero exit status (pinned),
    - return `Err(Backend)` only on spawn/wait/timeout/capture failures or manifest/runtime conflicts (pinned).
- **Dependencies**:
  - SEAM-1: typed add transport + transport validation helper + output enforcement helper (MM-C05/MM-C04).
  - SEAM-2: `allow_mcp_write` gating + isolated homes (`codex_home`) + write capability advertising (MM-C06/MM-C07).
  - S1: bounded runner and capture primitive.
- **Verification**:
  - `cargo test -p agent_api --features codex` (unit tests for argv composition + gating)
- **Rollout/safety**:
  - Safe-by-default: write ops remain unreachable unless enabled via backend config (SEAM-2).
  - Isolated homes reduce user-state mutation risk; request env overrides can defeat isolation by design (pinned).

## Atomic Tasks

#### S2.T1 — Add pure argv builders for `add/remove` (Codex, pinned)

- **Outcome**: Deterministic argv composition functions for Codex write ops (no subprocess spawning).
- **Inputs/outputs**:
  - Input: `docs/specs/universal-agent-api/mcp-management-spec.md` (“Codex backend mapping (pinned)”)
  - Output (suggested):
    - `crates/agent_api/src/backends/codex/mcp_management.rs`: `fn codex_mcp_remove_argv(name: &str) -> Vec<OsString>`
    - `crates/agent_api/src/backends/codex/mcp_management.rs`: `fn codex_mcp_add_argv(name: &str, transport: ...) -> Vec<OsString>`
    - unit tests that pin argv order and `--` separator placement
- **Implementation notes**:
  - Treat `name` + transport fields as already validated/trimmed by SEAM-1’s helper (avoid duplicating validation).
  - Ensure deterministic `--env KEY=VALUE` ordering (iterate the map in key order).
- **Acceptance criteria**:
  - Unit tests cover:
    - `Stdio` with multiple env vars + command+args concatenation,
    - `Url` with and without `bearer_token_env_var`.
- **Test notes**: pure unit tests only.
- **Risk/rollback notes**: low; internal helpers.

Checklist:
- Implement: argv builders + tests.
- Test: `cargo test -p agent_api --features codex`.
- Validate: confirm mapping matches pinned spec exactly.
- Cleanup: rustfmt.

#### S2.T2 — Implement `mcp_add` and `mcp_remove` hooks (write-gated, fail closed)

- **Outcome**: Codex backend supports MCP write operations with pinned gating + execution semantics.
- **Inputs/outputs**:
  - Input: SEAM-2 write advertising (`allow_mcp_write`) + SEAM-1 hook signatures + validation helper
  - Output:
    - `crates/agent_api/src/backends/codex.rs`: add MCP hook methods and forward to Codex MCP helper module
    - `crates/agent_api/src/backends/codex/mcp_management.rs`: hook implementations using the S1 runner
- **Implementation notes**:
  - Enforce fail-closed gating inside the hook:
    - if `self.capabilities()` does not contain the op capability id, return `UnsupportedCapability`.
  - Reuse S1 runner (bounded capture + timeout + context precedence).
  - Keep `AgentWrapperMcpAddTransport::Stdio.env` distinct from `request.context.env`:
    - `Stdio.env` maps to repeated `--env KEY=VALUE` args,
    - `request.context.env` maps to the *CLI process environment* only.
- **Acceptance criteria**:
  - When `allow_mcp_write=false` (default), `mcp_add/remove` are fail-closed (`UnsupportedCapability`) even if invoked directly.
  - When enabled + advertised, hooks spawn the pinned argv and return bounded `Ok(output)` regardless of exit status.
- **Test notes**:
  - Unit tests can assert gating + argv composition; hermetic fake binary execution is owned by SEAM-5.
- **Risk/rollback notes**:
  - Behavior is gated behind explicit enablement; rollback is removing/disable the hooks.

Checklist:
- Implement: hook wiring + runner reuse.
- Test: `cargo test -p agent_api --features codex`.
- Validate: ensure no output is emitted as run events.
- Cleanup: keep code localized to Codex backend modules.

#### S2.T3 — Add unit tests pinning write-op gating and argv mapping (Codex)

- **Outcome**: Deterministic regression tests that prevent drift in `add/remove` argv mapping and write gating behavior.
- **Inputs/outputs**:
  - Input: `docs/specs/universal-agent-api/mcp-management-spec.md` (mapping + gating rules)
  - Output: unit tests under `crates/agent_api/src/backends/codex/mcp_management.rs` (or adjacent test module).
- **Implementation notes**:
  - Pin that `mcp_add/remove` return `UnsupportedCapability` when write capability ids are absent.
  - Pin representative argv for both `Stdio` and `Url`.
- **Acceptance criteria**:
  - Tests fail on any argv drift, missing `--` separator, env ordering changes, or gating regressions.
- **Test notes**:
  - Run: `cargo test -p agent_api --features codex`.
- **Risk/rollback notes**: tests-only; safe.

Checklist:
- Implement: gating + argv assertions.
- Test: `cargo test -p agent_api --features codex`.
- Validate: confirm assertions match spec, not current upstream behavior guesses.
- Cleanup: keep tests small; avoid invoking subprocesses here.

## Notes for downstream seams (non-tasking)

- SEAM-5 will pin end-to-end behavior with a fake `codex` binary that records argv + env and writes state beneath the
  injected isolated home directory to assert writes are localized.

