# S2 — Codex MCP integration tests (argv/env/isolation/bounds/drift)

- **User/system value**: Pins Codex MCP management behavior end-to-end against the canonical spec by running operations
  through `AgentWrapperGateway::{mcp_list,mcp_get,mcp_add,mcp_remove}` with hermetic fake binaries. This prevents “silent”
  drift in argv mapping, environment precedence, output bounds, and error shaping.
- **Scope (in/out)**:
  - In:
    - Hermetic integration tests for Codex `list/get/add/remove` that:
      - invoke the Codex backend with `binary = <fake codex>` and `codex_home = <tempdir>`,
      - assert argv mapping matches `docs/specs/universal-agent-api/mcp-management-spec.md` exactly,
      - assert request context precedence (working_dir/timeout/env) is applied to the spawned management command,
      - assert output bounds + truncation marker + flags end-to-end,
      - assert command execution semantics:
        - `Ok(output)` even when exit status is non-zero,
        - `Err(Backend)` for spawn/wait/timeout/capture failures (no partial stdout/stderr leakage),
      - assert manifest drift behavior: runtime “unknown flag/subcommand” conflicts fail as `Err(Backend)` and do not mutate
        advertised capabilities.
    - Write-op gating tests (`allow_mcp_write`) for `add/remove`.
  - Out:
    - Backend mapping implementation itself (SEAM-3).
    - Claude Code mapping tests (S3).
- **Acceptance criteria**:
  - `mcp_list` invokes: `codex mcp list --json`.
  - `mcp_get` invokes: `codex mcp get --json <name>`.
  - `mcp_remove` invokes: `codex mcp remove <name>`.
  - `mcp_add` mapping is pinned:
    - `Stdio` → `codex mcp add <name> [--env KEY=VALUE]* -- <argv...>`
    - `Url` → `codex mcp add <name> --url <url> [--bearer-token-env-var <ENV_VAR>]`
  - `add/remove` are not advertised by default, and are only invocable when `allow_mcp_write=true` (MM-C06).
  - `request.context.env` keys win over isolated-home injection (MM-C03/MM-C07), and transport env (`Stdio.env`) is passed
    only via `--env KEY=VALUE` argv (not as CLI process env).
  - Over-bound stdout/stderr are truncated to 65,536 bytes with suffix `…(truncated)` and flags set (MM-C04).
  - Drift simulation (`--json` rejected / `mcp` unknown) returns `Err(Backend)` and does not silently change capability
    advertising (pinned).
- **Dependencies**:
  - SEAM-1: validation + output-bounds helper (MM-C03/MM-C04/MM-C05).
  - SEAM-2: advertising + `allow_mcp_write` + isolated home injection (MM-C06/MM-C07).
  - SEAM-3: Codex MCP argv mapping + drift classifier (MM-C08).
- **Verification**:
  - `cargo test -p agent_api --features codex,claude_code --test c5_mcp_management_v1 -- --nocapture`
- **Rollout/safety**:
  - Fake binaries only; no network; isolated homes only.

## Atomic Tasks

#### S2.T1 — Add Codex read-op integration tests (`list/get`) with argv + context assertions

- **Outcome**: End-to-end tests that spawn the fake `codex` binary and assert:
  - argv mapping for `list/get`,
  - working_dir override behavior,
  - request env injection and precedence.
- **Inputs/outputs**:
  - Output (suggested):
    - `crates/agent_api/tests/mcp_management_v1/codex_read_ops.rs`
  - Inputs:
    - fake binary: `CARGO_BIN_EXE_fake_codex_mcp_agent_api`
    - spec: `docs/specs/universal-agent-api/mcp-management-spec.md` (“Codex backend mapping (pinned)” + “Process context”)
- **Implementation notes**:
  - Use per-test:
    - temp record file (JSONL),
    - isolated `codex_home`,
    - a temp “bin dir” with `codex` fake executable (copy/hardlink) to satisfy “generate fake binaries per test run”.
  - Capture and assert only whitelisted env keys (avoid leaking host env into records).
- **Acceptance criteria**:
  - Record `args` equals the pinned mapping for `list/get`.
  - Record `cwd` equals `request.context.working_dir` when set.
  - Record includes request env keys (and shows they win over config defaults on collisions).
- **Test notes**:
  - Run: `cargo test -p agent_api --features codex --test c5_mcp_management_v1 codex_* -- --nocapture`
- **Risk/rollback notes**: low; deterministic fake process.

Checklist:
- Implement: tests for `mcp_list` and `mcp_get` record assertions.
- Test: targeted run under `--features codex`.
- Validate: ensure no test depends on real `codex` being installed.
- Cleanup: keep assertions byte-for-byte pinned to the spec.

#### S2.T2 — Add Codex write-op integration tests (`add/remove`) + write gating (`allow_mcp_write`)

- **Outcome**: Tests that pin:
  - safe-by-default advertising (write ops off),
  - typed transport mapping for `mcp_add` (stdio + url),
  - isolated-home state mutation (sentinel files under `codex_home`),
  - and `mcp_remove` argv mapping.
- **Inputs/outputs**:
  - Output (suggested):
    - `crates/agent_api/tests/mcp_management_v1/codex_write_ops.rs`
  - Inputs:
    - spec: `docs/specs/universal-agent-api/mcp-management-spec.md` (“Add transport typing” + “Codex mapping (pinned)”)
    - contract: `docs/specs/universal-agent-api/contract.md` (“allow_mcp_write”)
- **Implementation notes**:
  - Split into two cases:
    1) `allow_mcp_write=false` → `mcp_add/remove` return `UnsupportedCapability` without spawning.
    2) `allow_mcp_write=true` → operations spawn fake binary and produce record + sentinel writes.
  - Pin the “transport env vs CLI env” distinction:
    - include a `Stdio.env` pair like `("MCP_SERVER_ENV", "1")` and assert:
      - record `args` contains `--env MCP_SERVER_ENV=1`,
      - record `env` does **not** contain `MCP_SERVER_ENV`.
  - Pin bearer-token env var semantics:
    - set `bearer_token_env_var=Some("MY_TOKEN")` with `request.context.env["MY_TOKEN"]="SECRET"` and assert argv contains
      `--bearer-token-env-var MY_TOKEN` and does not contain `SECRET`.
- **Acceptance criteria**:
  - `mcp_add`/`mcp_remove` are gated by `allow_mcp_write`.
  - Sentinels are written beneath the injected isolated home root for write ops.
  - No argv pass-through exists: only typed transports appear in argv.
- **Test notes**:
  - Run: `cargo test -p agent_api --features codex --test c5_mcp_management_v1 codex_* -- --nocapture`
- **Risk/rollback notes**: low; deterministic fake process + tempdirs.

Checklist:
- Implement: write gating tests + stdio/url mapping assertions.
- Test: targeted run under `--features codex`.
- Validate: ensure sentinel writes are confined to temp `codex_home`.
- Cleanup: avoid duplicating argv builders (assert record only).

#### S2.T3 — Add Codex failure-mode regressions (non-zero exit, timeout, drift classifier, no leaks)

- **Outcome**: Pin the “hard edges” that tend to regress:
  - non-zero exit returns `Ok(output)` with captured bounded stdout/stderr,
  - timeouts become `Err(Backend)` without leaking partial stdout/stderr in the error message,
  - drift-style stderr (`unknown flag/subcommand`) becomes `Err(Backend)` and does not change advertised capabilities.
- **Inputs/outputs**:
  - Output (suggested):
    - `crates/agent_api/tests/mcp_management_v1/codex_failures.rs`
- **Implementation notes**:
  - Use fake-binary scenarios controlled by env to produce:
    - bounded overage output,
    - non-zero exit,
    - sleep longer than `request.context.timeout`,
    - drift-like stderr messages.
  - For timeout and drift, assert the returned `AgentWrapperError::Backend.message` does not include any sentinel output
    emitted by the fake binary (no partial-output leakage).
  - “No capability mutation” regression:
    - capture `backend.capabilities()` before the call and compare after the call; they must match.
- **Acceptance criteria**:
  - All three regressions are pinned and deterministic under parallel `cargo test`.
- **Test notes**:
  - Run: `cargo test -p agent_api --features codex --test c5_mcp_management_v1 codex_* -- --nocapture`
- **Risk/rollback notes**: medium risk (timeout flake); mitigate with small sleeps and conservative timeouts.

Checklist:
- Implement: failure-mode scenarios + tests.
- Test: repeated targeted runs locally to catch flake.
- Validate: ensure timeout tests cannot hang the test runner (kill + bounded wait).
- Cleanup: keep sentinel strings unique and asserted absent from error messages.

