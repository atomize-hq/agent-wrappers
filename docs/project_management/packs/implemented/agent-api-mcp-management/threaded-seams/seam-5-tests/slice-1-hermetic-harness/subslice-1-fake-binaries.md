# S1a — Hermetic fake MCP binaries (`codex` + `claude`)

- **User/system value**: Provides deterministic, offline “upstream CLIs” for MCP management tests by recording invocations
  and simulating isolated-home state mutation via sentinel files.
- **Scope (in/out)**:
  - In:
    - Fake `codex` MCP management binary with JSONL invocation recording + sentinel write behavior.
    - Fake `claude` MCP management binary with JSONL invocation recording + sentinel write behavior.
    - Env-driven deterministic scenarios for bounds/timeout/drift simulations (used by later test slices).
  - Out:
    - Shared Rust test harness for per-test tempdirs + record parsing (S1b).
    - Capability advertising + non-run boundary regressions (S1c).
    - Backend-specific argv mapping assertions (owned by S2/S3).
- **Acceptance criteria**:
  - Each invocation appends exactly one JSONL record (stable fields; deterministic ordering).
  - `args` are captured excluding argv[0]; `cwd` is captured; `env` captures only an explicit allowlist of keys.
  - Write-ops (`add/remove`) create sentinel files *only* under the injected isolated home root.
  - Scenario knobs exist to deterministically emit:
    - stdout/stderr > 65,536 bytes (bounds coverage),
    - non-zero exit,
    - sleep-for-timeout,
    - drift-style stderr (`unknown flag/subcommand`).
- **Dependencies**:
  - SEAM-2 isolated-home wiring: fake binaries derive the “home root” from injected env (Codex: `CODEX_HOME`; Claude:
    `HOME`/`XDG_*` as wired by the backend config).
- **Verification**:
  - `cargo test -p agent_api --all-features --test c5_mcp_management_v1 -- --nocapture`
- **Rollout/safety**:
  - Test-only binaries; invoked only by integration tests; no network access; no user-state mutation (temp isolated homes).

## Atomic Tasks (moved from S1)

#### S1.T1 — Add hermetic fake MCP binaries (`codex` + `claude`) with record + sentinel support

- **Outcome**: Two compiled fake binaries that behave like minimal upstream CLIs for MCP management tests:
  - accept `mcp {list,get,add,remove}` argv shapes,
  - record argv/env/cwd to a JSONL record file,
  - write sentinel files under the injected isolated home root for `add/remove`,
  - support deterministic scenarios for bounds, timeouts, and drift.
- **Files** (suggested):
  - `crates/agent_api/src/bin/fake_codex_mcp_agent_api.rs`
  - `crates/agent_api/src/bin/fake_claude_mcp_agent_api.rs`

Checklist:
- Implement:
  - Append one JSONL record per invocation with stable keys: `args`, `cwd`, `env` (allowlisted).
  - Implement sentinel writes beneath injected home root for `add/remove` (and no other ops).
  - Add env-driven scenario knobs for: big output, non-zero exit, sleep, drift-stderr.
- Test:
  - Run `cargo test -p agent_api --all-features --test c5_mcp_management_v1 -- --nocapture` (even if assertions land in later sub-slices).
- Validate:
  - Confirm fake binaries never read/write outside the injected isolated home root + record path.
  - Keep scenario knobs minimal and documented in the binary sources.
