# S1a — Claude read-op helpers (argv builders + bounded capture)

- **User/system value**: Establish deterministic pinned argv composition and bounded stdout/stderr capture primitives used by
  Claude MCP ops.
- **Scope (in/out)**:
  - In:
    - Pure argv builders for `claude mcp list` and `claude mcp get <name>` (pinned).
    - Bounded streaming capture helper for stdout/stderr (MM-C04 capture step).
    - Pure unit tests for argv equality + bounded-capture invariants.
  - Out:
    - Subprocess runner / timeout behavior (S1b).
    - Drift classification (S1c).
    - Backend hook wiring + gating tests (S1d).
- **Acceptance criteria**:
  - `claude_mcp_list_argv()` and `claude_mcp_get_argv(name)` produce byte-for-byte pinned argv (no extra flags).
  - `capture_bounded(...)` retains at most `bound_bytes` (plus small fixed overhead) and reports `saw_more_bytes` when
    truncation occurred.
- **Dependencies**:
  - `docs/specs/universal-agent-api/mcp-management-spec.md` (pinned mapping + MM-C04 algorithm).
  - SEAM-1 provides server-name validation; treat `name` as already validated/trimmed by the SEAM-1 helper.
- **Verification**:
  - `cargo test -p agent_api --features claude_code`
- **Rollout/safety**:
  - Internal-only helpers; no runtime behavior changes until wired by later sub-slices.

## Atomic Tasks (moved from S1)

#### S1.T1 — Add pure argv builders for `list/get` (Claude, pinned)

- **Outcome**: Deterministic argv composition functions for Claude read ops that do not spawn processes.
- **Files** (suggested):
  - `crates/agent_api/src/backends/claude_code/mcp_management.rs`

Checklist:
- Implement:
  - Add `claude_mcp_list_argv() -> Vec<OsString>` pinned to `claude mcp list`.
  - Add `claude_mcp_get_argv(name: &str) -> Vec<OsString>` pinned to `claude mcp get <name>`.
  - Keep argv construction byte-for-byte pinned (no extra flags; no `--json`).
- Test:
  - Add pure unit tests asserting argv equality for representative inputs.
  - Run `cargo test -p agent_api --features claude_code`.
- Validate:
  - Confirm tests reflect the spec (not current upstream behavior guesses).

#### S1.T2 — Implement bounded streaming capture primitive for subprocess stdout/stderr (MM-C04 capture step)

- **Outcome**: A bounded capture helper that retains at most `bound_bytes` bytes (plus small fixed overhead) per stream.
- **Files** (suggested):
  - `crates/agent_api/src/backends/claude_code/mcp_management.rs`

Checklist:
- Implement:
  - Implement `capture_bounded<R: AsyncRead + Unpin>(...) -> (Vec<u8>, bool)` (or equivalent) returning:
    - retained bytes (bounded),
    - `saw_more_bytes` (whether truncation occurred).
  - Do not buffer unbounded output; enforce the retained-bytes invariant while reading.
  - Keep this helper local to Claude mapping initially (avoid cross-backend refactors).
- Test:
  - Use in-memory streams (e.g., `tokio::io::duplex`) to generate over-bound output deterministically.
  - Run `cargo test -p agent_api --features claude_code`.
- Validate:
  - Confirm bounded memory posture (no unbounded `Vec` growth).

