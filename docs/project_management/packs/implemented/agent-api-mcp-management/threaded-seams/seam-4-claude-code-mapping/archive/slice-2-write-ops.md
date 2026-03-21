# S2 — Write ops (`add/remove`) mapping + typed transports + write gating (decomposed)

<<<<<<<< HEAD:docs/project_management/packs/implemented/agent-api-mcp-management/threaded-seams/seam-4-claude-code-mapping/archive/slice-2-write-ops.md
- **User/system value**: Enables universal MCP management write operations for the Claude Code backend when explicitly enabled,
  with pinned typed transport mapping (MM-C05), isolation support (MM-C07), and bounded outputs (MM-C04), while remaining
  safe-by-default (MM-C06).
- **Scope (in/out)**:
  - In:
    - Implement `ClaudeCodeBackend::{mcp_add,mcp_remove}` mapping to pinned argv:
      - `remove` → `claude mcp remove <name>` (**win32-x64 only** per pinned manifest)
      - `add` (**win32-x64 only** per pinned manifest):
        - `Stdio` → `claude mcp add --transport stdio [--env KEY=VALUE]* <name> <command> [args...]`
        - `Url`:
          - when `bearer_token_env_var == None` → `claude mcp add --transport http <name> <url>`
          - when `bearer_token_env_var == Some(_)` → reject as `InvalidRequest` (pinned; fail closed).
    - Enforce fail-closed write gating:
      - if `agent_api.tools.mcp.{add,remove}.v1` is not advertised (including
        `ClaudeCodeBackendConfig.allow_mcp_write=false`), return `UnsupportedCapability`.
    - Reuse S1 runner for context precedence + bounded output + timeout behavior.
  - Out:
    - Capability advertising + `ClaudeCodeBackendConfig.allow_mcp_write` enablement (SEAM-2).
    - Cross-backend hermetic fake-binary integration tests (SEAM-5).
- **Acceptance criteria**:
  - `mcp_remove` invokes `claude mcp remove <name>` and returns bounded `AgentWrapperMcpCommandOutput` on supported targets.
  - `mcp_add` maps typed transports exactly (no argv pass-through):
    - `Stdio`: repeats `--env KEY=VALUE` deterministically (map iteration order) and maps `command` + `args` to the tail argv.
    - `Url`: uses `--transport http <name> <url>` only when `bearer_token_env_var == None`.
    - `Url` with `bearer_token_env_var == Some(_)` returns `InvalidRequest` (pinned).
  - Both operations:
    - fail closed with `UnsupportedCapability` when write capability ids are unadvertised / disabled (MM-C06),
    - honor isolated homes via backend config, but allow request env overrides to win (MM-C07/MM-C03),
    - return `Ok(output)` even on non-zero exit status (pinned),
    - return `Err(Backend)` only on spawn/wait/timeout/capture failures or manifest/runtime conflicts (pinned).
- **Dependencies**:
  - SEAM-1: typed add transport + transport validation helper + output enforcement helper (MM-C05/MM-C04).
  - SEAM-2: `ClaudeCodeBackendConfig.allow_mcp_write` gating + isolated homes (`claude_home`) +
    write capability advertising (MM-C06/MM-C07) and pinned `win32-x64` availability for
    `get/add/remove` (MM-C09).
  - S1: bounded runner and capture primitive.
- **Verification**:
  - `cargo test -p agent_api --features claude_code` (unit tests for argv composition + gating + bearer-token rejection)
- **Rollout/safety**:
  - Safe-by-default: write ops remain unreachable while
    `ClaudeCodeBackendConfig.allow_mcp_write == false` (the default) (SEAM-2).
  - Isolated homes reduce user-state mutation risk; request env overrides can defeat isolation by design (pinned).

## Atomic Tasks

#### S2.T1 — Add pure argv builders for `add/remove` (Claude, pinned)

- **Outcome**: Deterministic argv composition functions for Claude write ops (no subprocess spawning), including pinned
  rejection behavior for `Url.bearer_token_env_var`.
- **Inputs/outputs**:
  - Input: `docs/specs/universal-agent-api/mcp-management-spec.md` (“Claude Code backend mapping (pinned)”)
  - Output (suggested):
    - `crates/agent_api/src/backends/claude_code/mcp_management.rs`: `fn claude_mcp_remove_argv(name: &str) -> Vec<OsString>`
    - `crates/agent_api/src/backends/claude_code/mcp_management.rs`: `fn claude_mcp_add_argv(name: &str, transport: ...) -> Result<Vec<OsString>, AgentWrapperError>`
    - unit tests that pin argv order and `--env` ordering
- **Implementation notes**:
  - Treat `name` + transport fields as already validated/trimmed by SEAM-1’s helper (avoid duplicating validation).
  - Ensure deterministic `--env KEY=VALUE` ordering (iterate the map in key order).
  - Keep `Url.bearer_token_env_var` logic local to Claude mapping:
    - `None` → supported mapping,
    - `Some(_)` → `InvalidRequest` (pinned; fail closed).
- **Acceptance criteria**:
  - Unit tests cover:
    - `Stdio` with multiple env vars + command+args tail mapping,
    - `Url` with `bearer_token_env_var == None`,
    - `Url` with `bearer_token_env_var == Some(_)` → `InvalidRequest`.
- **Test notes**: pure unit tests only.
- **Risk/rollback notes**: low; internal helpers.

Checklist:
- Implement: argv builders + tests.
- Test: `cargo test -p agent_api --features claude_code`.
- Validate: confirm mapping matches pinned spec exactly.
- Cleanup: rustfmt.

#### S2.T2 — Implement `mcp_add` and `mcp_remove` hooks (write-gated, fail closed)

- **Outcome**: Claude backend supports MCP write operations with pinned gating + execution semantics.
- **Inputs/outputs**:
  - Input: SEAM-2 write advertising (`ClaudeCodeBackendConfig.allow_mcp_write`) + SEAM-1 hook
    signatures + validation helper
  - Output:
    - `crates/agent_api/src/backends/claude_code.rs`: add MCP hook methods and forward to Claude MCP helper module
    - `crates/agent_api/src/backends/claude_code/mcp_management.rs`: hook implementations using the S1 runner
- **Implementation notes**:
  - Enforce fail-closed gating inside the hook:
    - if `self.capabilities()` does not contain the op capability id, return `UnsupportedCapability`.
  - Reuse S1 runner (bounded capture + timeout + context precedence + drift classifier).
  - Keep `AgentWrapperMcpAddTransport::Stdio.env` distinct from `request.context.env`:
    - `Stdio.env` maps to repeated `--env KEY=VALUE` args (persisted by the upstream CLI),
    - `request.context.env` maps to the *CLI process environment* only.
- **Acceptance criteria**:
  - When `ClaudeCodeBackendConfig.allow_mcp_write=false` (default), `mcp_add/remove` are
    fail-closed (`UnsupportedCapability`) even if invoked directly.
  - When enabled + advertised on `win32-x64`, hooks spawn the pinned argv and return bounded `Ok(output)` regardless of exit status.
  - `Url.bearer_token_env_var == Some(_)` returns `InvalidRequest` before spawning any subprocess (pinned).
- **Test notes**:
  - Assert write-capability presence/absence via backend `capabilities().ids`, not the generated
    capability matrix.
  - Unit tests can assert gating + argv composition + invalid-request behavior; hermetic fake binary execution is owned by SEAM-5.
- **Risk/rollback notes**:
  - Behavior is gated behind explicit enablement; rollback is removing/disable the hooks.

Checklist:
- Implement: hook wiring + runner reuse + early `InvalidRequest`.
- Test: `cargo test -p agent_api --features claude_code`.
- Validate: ensure no output is emitted as run events.
- Cleanup: keep code localized to Claude backend modules.

#### S2.T3 — Add unit tests pinning write-op gating, bearer-token rejection, and argv mapping (Claude)

- **Outcome**: Deterministic regression tests that prevent drift in `add/remove` argv mapping, write gating, and the pinned
  bearer-token rejection rule.
- **Inputs/outputs**:
  - Input: `docs/specs/universal-agent-api/mcp-management-spec.md` (mapping + gating + bearer-token semantics)
  - Output: unit tests under `crates/agent_api/src/backends/claude_code/mcp_management.rs` (or adjacent test module).
- **Implementation notes**:
  - Pin that `mcp_add/remove` return `UnsupportedCapability` when write capability ids are absent.
  - Pin that `mcp_add(Url{ bearer_token_env_var: Some(_) })` returns `InvalidRequest`.
  - Pin representative argv for `Stdio` and `Url(None)`.
- **Acceptance criteria**:
  - Tests fail on any argv drift, env ordering changes, gating regressions, or bearer-token rejection regressions.
- **Test notes**:
  - Run: `cargo test -p agent_api --features claude_code`.
- **Risk/rollback notes**: tests-only; safe.

Checklist:
- Implement: gating + argv + invalid-request assertions.
- Test: `cargo test -p agent_api --features claude_code`.
- Validate: confirm assertions match spec, not current upstream behavior guesses.
- Cleanup: keep tests small; avoid invoking subprocesses here.

## Notes for downstream seams (non-tasking)

- SEAM-5 will pin end-to-end behavior with a fake `claude` binary that records argv + env and writes state beneath the
  injected isolated home directory to assert writes are localized and gating is respected.
========
- Archived original: `archive/slice-2-write-ops.md`
- Sub-slices live in: `slice-2-write-ops/`
- Recommended order: S2a -> S2b -> S2c

#### Sub-slices

- `slice-2-write-ops/subslice-1-argv-builders.md` — S2a: pinned `add/remove` argv builders + deterministic `--env` ordering + bearer-token rejection
- `slice-2-write-ops/subslice-2-hooks-and-gating.md` — S2b: Claude write-hook wiring + fail-closed capability gating + runner reuse
- `slice-2-write-ops/subslice-3-regression-tests.md` — S2c: unit tests pinning argv mapping, gating, and `InvalidRequest` behavior
>>>>>>>> origin/main:docs/project_management/packs/active/agent-api-mcp-management/threaded-seams/seam-4-claude-code-mapping/slice-2-write-ops.md
