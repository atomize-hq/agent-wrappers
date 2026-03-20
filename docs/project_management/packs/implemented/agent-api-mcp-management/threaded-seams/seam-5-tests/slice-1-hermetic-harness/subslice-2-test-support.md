# S1b — Shared test support (isolated homes + per-test fake executables)

- **User/system value**: Enables parallel, hermetic integration tests by providing per-test tempdirs for isolated homes,
  per-test “bin dirs” containing fake `codex`/`claude` executables, and deterministic record parsing.
- **Scope (in/out)**:
  - In:
    - Support module for creating per-test:
      - isolated home roots (Codex + Claude),
      - record file paths,
      - temp “bin dir” with `codex`/`claude` fake executables (copy or hardlink).
    - JSONL record parsing helpers (stable, minimal schema).
  - Out:
    - Backend-specific mapping assertions (S2/S3).
    - Capability advertising + non-run boundary regressions (S1c).
- **Acceptance criteria**:
  - Two tests running in parallel never share:
    - record file path,
    - isolated home root,
    - temp “bin dir”.
  - Windows is supported (`codex.exe` / `claude.exe` naming when copying into the bin dir).
  - Helpers remain test-only (no new non-test dependencies; no production surface changes).
- **Dependencies**:
  - S1a fake binaries: `CARGO_BIN_EXE_fake_codex_mcp_agent_api` and `CARGO_BIN_EXE_fake_claude_mcp_agent_api`.
  - SEAM-2 isolated-home env semantics (what keys to inject for each backend).
- **Verification**:
  - `cargo test -p agent_api --all-features --test c5_mcp_management_v1 -- --nocapture`
- **Rollout/safety**:
  - Test-only support code; uses tempdirs; does not mutate host env globally (set env per test/backend).

## Atomic Tasks (moved from S1)

#### S1.T2 — Add shared test support for isolated homes + per-test fake executables

- **Outcome**: A small, reusable harness that each integration test can use to:
  - create tempdir isolated homes,
  - create a tempdir “bin” with fake `codex` / `claude` executables (copy/hardlink compiled fakes per test),
  - read and parse invocation records.
- **Files** (suggested):
  - `crates/agent_api/tests/mcp_management_v1/support.rs`

Checklist:
- Implement:
  - Tempdir helpers for per-test record path + isolated home roots.
  - Copy/hardlink helpers that place `codex`/`claude` binaries into a per-test bin dir (with `.exe` on Windows).
  - Minimal JSONL record reader + parser (tolerant of multiple lines; one invocation == one line).
- Test:
  - Run targeted tests under `cargo test -p agent_api --all-features --test c5_mcp_management_v1 -- --nocapture`.
- Validate:
  - Ensure no helper sets global env for the whole test process (prefer backend config/request-scoped env).
  - Ensure default Rust test parallelism cannot cause cross-test record collisions.
