# S1 — Read ops (`list/get`) mapping + bounded exec runner

- **User/system value**: Enables universal MCP management read operations for the Codex backend with deterministic,
  bounded output capture and pinned process-context semantics (MM-C03/MM-C04), while preserving the non-run boundary
  (MM-C02).
- **Scope (in/out)**:
  - In:
    - Implement `CodexBackend::{mcp_list,mcp_get}` mapping to pinned argv:
      - `codex mcp list --json`
      - `codex mcp get --json <name>`
    - Add a Codex-only “MCP command runner” that:
      - enforces capability gating (MM-C01),
      - applies context precedence rules (MM-C03),
      - captures stdout/stderr with bounded streaming capture and applies SEAM-1’s bounds enforcement helper (MM-C04),
      - returns `Ok(output)` even on non-zero exit status (pinned),
      - returns `Err(Backend)` only for spawn/wait/timeout/capture failures.
    - Detect manifest/runtime conflicts and fail as `Err(Backend)` (pinned) without mutating advertised capabilities.
  - Out:
    - Request validation helper + output enforcement helper (SEAM-1).
    - Capability advertising + isolated home config (SEAM-2).
    - Hermetic fake-binary integration tests (SEAM-5).
- **Acceptance criteria**:
  - `mcp_list` invokes `codex mcp list --json` and returns bounded `AgentWrapperMcpCommandOutput`.
  - `mcp_get` invokes `codex mcp get --json <name>` and returns bounded `AgentWrapperMcpCommandOutput`.
  - Both operations:
    - fail closed with `UnsupportedCapability` when the capability id is not advertised (MM-C01),
    - do not emit MCP stdout/stderr as run events (MM-C02),
    - honor context precedence + env collision rules (MM-C03),
    - bound stdout/stderr to 65,536 bytes and apply the pinned suffix + flags (MM-C04),
    - return `Ok(output)` for non-zero exit status (pinned),
    - return `Err(Backend)` for spawn/wait/timeout/capture failures (pinned) without leaking partial stdout/stderr.
  - If the runtime upstream CLI behavior conflicts with the pinned manifest snapshot (e.g., `mcp`/`--json` not recognized
    on a target where it is advertised), the operation fails as `Err(Backend)` (pinned) and does not mutate advertised capabilities.
- **Dependencies**:
  - SEAM-1: `agent_api::mcp` types + server name validation helper + output-bounds enforcement helper (MM-C01/03/04).
  - SEAM-2: capability advertising (read ops) + isolated home injection field (`codex_home`) (MM-C06/MM-C07).
- **Verification**:
  - `cargo test -p agent_api --features codex` (unit tests for argv + capture primitive + env precedence + gating)
- **Rollout/safety**:
  - Safe-by-default: read ops only; no persistent state mutation required.
  - Non-run boundary enforced (MCP outputs never enter the run event pipeline).

## Atomic Tasks

#### S1.T1 — Add pure argv builders for `list/get` (Codex, pinned)

- **Outcome**: Deterministic argv composition functions for Codex read ops that do not spawn processes.
- **Inputs/outputs**:
  - Input: `docs/specs/universal-agent-api/mcp-management-spec.md` (“Codex backend mapping (pinned)”)
  - Output (suggested):
    - `crates/agent_api/src/backends/codex/mcp_management.rs`: `fn codex_mcp_list_argv() -> Vec<OsString>`
    - `crates/agent_api/src/backends/codex/mcp_management.rs`: `fn codex_mcp_get_argv(name: &str) -> Vec<OsString>`
    - unit tests co-located with the module
- **Implementation notes**:
  - Treat `name` as already validated/trimmed by the SEAM-1 helper (avoid duplicating validation).
  - Keep argv construction byte-for-byte pinned (`--json` always present).
- **Acceptance criteria**:
  - Unit tests assert argv equality for representative inputs.
- **Test notes**:
  - Pure unit tests only; no subprocesses.
- **Risk/rollback notes**: low; internal-only helpers.

Checklist:
- Implement: add argv builders + tests.
- Test: `cargo test -p agent_api --features codex`.
- Validate: confirm argv matches spec exactly.
- Cleanup: rustfmt.

#### S1.T2 — Implement bounded streaming capture primitive for subprocess stdout/stderr (MM-C04 capture step)

- **Outcome**: A bounded capture helper that retains at most `bound_bytes + 1` bytes (or `bound_bytes` + `saw_more`) per stream.
- **Inputs/outputs**:
  - Input: `docs/specs/universal-agent-api/mcp-management-spec.md` (“Output capture + truncation algorithm (pinned)”)
  - Output (suggested):
    - `crates/agent_api/src/backends/codex/mcp_management.rs`: `async fn capture_bounded<R: AsyncRead + Unpin>(...) -> ...`
- **Implementation notes**:
  - Do not buffer unbounded output; enforce the retained-bytes invariant while reading.
  - Return `(captured_bytes, saw_more_bytes)` so SEAM-1’s enforcement helper can apply suffix + flags deterministically.
  - Prefer a small, locally-tested helper rather than reusing `codex::*` output capture (Codex wrapper capture is unbounded and
    treats non-zero exit as an error).
- **Acceptance criteria**:
  - The helper never grows memory usage beyond the bound (+ small fixed overhead).
  - `saw_more_bytes == true` when the stream emitted bytes past the retained limit.
- **Test notes**:
  - Use in-memory streams (e.g., `tokio::io::duplex`) to generate over-bound output deterministically.
- **Risk/rollback notes**: low; new internal helper.

Checklist:
- Implement: bounded capture helper + minimal tests.
- Test: `cargo test -p agent_api --features codex`.
- Validate: confirm bounded memory posture (no `Vec::extend` without a cap).
- Cleanup: keep helper private to Codex backend mapping.

#### S1.T3 — Implement Codex MCP command runner (context precedence + timeout + bounded output)

- **Outcome**: A single runner used by `mcp_list`/`mcp_get` that enforces MM-C03/MM-C04 and pinned execution semantics.
- **Inputs/outputs**:
  - Input: SEAM-1 request validation helper + output enforcement helper (MM-C03/MM-C04)
  - Output:
    - `crates/agent_api/src/backends/codex/mcp_management.rs`: `async fn run_codex_mcp(argv: Vec<OsString>, context: ...) -> Result<AgentWrapperMcpCommandOutput, AgentWrapperError>`
- **Implementation notes**:
  - Apply pinned context precedence:
    - working_dir: `request.context.working_dir` → `config.default_working_dir` → backend default.
    - timeout: `request.context.timeout` → `config.default_timeout` → backend default (or none).
    - env: `config.env` + isolated-home injection + `request.context.env` overrides win.
  - Timeout failures are `Err(Backend)` and must not include partial stdout/stderr.
  - Non-zero exit status returns `Ok(output)` (pinned) unless classified as a manifest/runtime conflict (S1.T4).
- **Acceptance criteria**:
  - Runner returns bounded outputs with truncation flags, and never emits run events.
  - Spawn/wait/timeout/capture failures return `Err(Backend)` with safe messages.
- **Test notes**:
  - Unit tests for env precedence and bounded output enforcement can exercise the runner with a small fake child process
    in SEAM-5; in SEAM-3 keep tests pure where possible (argv/context helpers).
- **Risk/rollback notes**:
  - Risk is limited to the new non-run surface; rollback is removing the MCP hooks.

Checklist:
- Implement: runner + context/env merge helper(s).
- Test: `cargo test -p agent_api --features codex`.
- Validate: confirm `Ok(output)` on non-zero exit (except drift classification).
- Cleanup: keep error messages pinned/safe.

#### S1.T4 — Implement manifest/runtime conflict classification (fail as `Err(Backend)`, pinned)

- **Outcome**: When a Codex MCP op is invoked but the runtime CLI rejects the pinned argv shape as unsupported on this target
  (manifest drift), return `Err(Backend)` and do not mutate advertised capabilities.
- **Inputs/outputs**:
  - Input: `docs/specs/universal-agent-api/mcp-management-spec.md` (“Target availability source of truth (pinned)”)
  - Output: a small classifier (location flexible) used by the runner before returning `Ok(output)`.
- **Implementation notes**:
  - Keep messages safe and non-echoing (do not include raw stderr/stdout in the error).
  - Prefer conservative classification:
    - detect “unknown subcommand/flag” style failures for `mcp` / `--json`,
    - avoid treating “normal” domain failures (e.g., “server not found”) as drift.
- **Acceptance criteria**:
  - A synthetic “unknown subcommand/flag” output is mapped to `Err(Backend)` in unit tests.
  - “Normal” non-zero exits remain `Ok(output)`.
- **Test notes**:
  - Pure unit tests for the classifier (no subprocesses).
- **Risk/rollback notes**: low; changes only the error shape for drift cases.

Checklist:
- Implement: classifier + tests.
- Test: `cargo test -p agent_api --features codex`.
- Validate: ensure error message does not leak partial output.
- Cleanup: keep classifier narrowly-scoped to “manifest drift” indicators.

## Notes for downstream seams (non-tasking)

- SEAM-5 will execute hermetic fake `codex` binaries to pin `mcp list/get` argv, env injection (including isolated homes),
  and bounded output behavior end-to-end.

