# S1 — Hermetic fake-binary harness + capability/non-run regressions

- **User/system value**: Provides a deterministic, offline test harness for MCP management and pins the “safety posture”
  invariants (capability gating, safe advertising defaults, isolated homes, and non-run boundary) so backend mapping tests
  are stable and do not depend on installed upstream CLIs.
- **Scope (in/out)**:
  - In:
    - Hermetic fake `codex` / `claude` binaries suitable for integration tests:
      - record received argv + selected env keys + cwd,
      - perform “state mutation” only via sentinel files beneath the injected isolated home root,
      - support deterministic scenarios (large stdout/stderr, non-zero exit, sleep-for-timeout, drift-style stderr).
    - Shared integration-test support for:
      - per-test tempdirs (record path + isolated homes),
      - copying/hardlinking the compiled fake binaries into a temp dir (per-test-run “fake executable”),
      - parsing invocation records from the fake binaries.
    - Cross-backend regression tests that do **not** require backend-specific argv details:
      - default capability advertising posture (`list/get` vs write gating),
      - Claude target gating (`win32-x64` only for `get/add/remove` per pinned manifest),
      - **non-run boundary**: MCP capability ids MUST NOT be accepted as run extension keys.
  - Out:
    - Backend-specific argv mapping assertions (owned by S2/S3).
    - End-to-end networked MCP server tests.
- **Acceptance criteria**:
  - A hermetic fake binary exists for each backend (`codex` and `claude`) and can:
    - append a JSONL invocation record to a per-test record file, and
    - create sentinel files under the injected home root for write operations.
  - Test support can create per-test isolated homes and per-test fake executables, so parallel `cargo test` runs have no
    shared mutable state.
  - Capability tests pin the advertising table in `docs/specs/universal-agent-api/mcp-management-spec.md`
    (“Default capability advertising posture (built-in backends, pinned)”).
  - Non-run boundary test pins:
    - passing `agent_api.tools.mcp.list.v1` as a run extension key fails as `UnsupportedCapability` (fail closed),
      proving MCP management is not modeled as run extensions.
- **Dependencies**:
  - SEAM-1: MCP management capability ids + gateway entrypoints + error types (MM-C01/MM-C02).
  - SEAM-2: safe default advertising posture + isolated home config fields (MM-C06/MM-C07).
- **Verification**:
  - `cargo test -p agent_api --all-features --test c5_mcp_management_v1`
- **Rollout/safety**:
  - Offline by construction; no real upstream binaries invoked.
  - Default paths are isolated tempdirs; no user-state mutation.

## Atomic Tasks

#### S1.T1 — Add hermetic fake MCP binaries (`codex` + `claude`) with record + sentinel support

- **Outcome**: Two compiled fake binaries that behave like minimal upstream CLIs for MCP management tests:
  - accept `mcp {list,get,add,remove}` argv shapes,
  - record argv/env/cwd to a JSONL record file,
  - write sentinel files under the injected isolated home root for `add/remove`,
  - support deterministic scenarios for bounds, timeouts, and drift.
- **Inputs/outputs**:
  - Output:
    - `crates/agent_api/src/bin/fake_codex_mcp_agent_api.rs`
    - `crates/agent_api/src/bin/fake_claude_mcp_agent_api.rs`
- **Implementation notes**:
  - Record format (suggested JSONL per invocation):
    - `args`: argv excluding argv[0] (stable for assertions),
    - `cwd`: `std::env::current_dir()` (stringified),
    - `env`: only a whitelisted subset of env keys (provided via an env var like `FAKE_*_CAPTURE_ENV_KEYS`),
    - `timestamp` is optional (avoid nondeterminism unless needed).
  - Scenario selection:
    - support env-driven knobs like:
      - “emit > 65,536 bytes to stdout/stderr”,
      - “exit with code 1”,
      - “sleep N ms”,
      - “emit drift-style ‘unknown flag/subcommand’ stderr”.
  - Sentinel behavior:
    - derive the home root from the injected env (Codex: `CODEX_HOME`; Claude: `HOME`/`XDG_*` per SEAM-2 wiring),
      and write sentinel files beneath that root.
- **Acceptance criteria**:
  - Each invocation appends exactly one record line; record parsing is deterministic.
  - Sentinel files are created only for simulated write ops.
- **Test notes**:
  - Validate via S1.T3 capability/non-run tests plus S2/S3 integration tests.
- **Risk/rollback notes**: test-only binaries; low risk.

Checklist:
- Implement: fake binaries + JSONL record + sentinel writes.
- Test: `cargo test -p agent_api --all-features --test c5_mcp_management_v1`.
- Validate: ensure no real binaries are ever invoked by default tests.
- Cleanup: keep env knobs minimal and documented in comments.

#### S1.T2 — Add shared test support for isolated homes + per-test fake executables

- **Outcome**: A small, reusable harness that each integration test can use to:
  - create tempdir isolated homes,
  - create a tempdir “bin” with fake `codex` / `claude` executables (copy/hardlink compiled fakes per test),
  - read and parse invocation records.
- **Inputs/outputs**:
  - Output (suggested):
    - `crates/agent_api/tests/mcp_management_v1/support.rs`
  - Inputs:
    - compiled fake binary paths:
      - `CARGO_BIN_EXE_fake_codex_mcp_agent_api`
      - `CARGO_BIN_EXE_fake_claude_mcp_agent_api`
- **Implementation notes**:
  - Prefer per-test record paths to avoid cross-test interference under default Rust test parallelism.
  - Ensure Windows compatibility:
    - choose the correct executable filename (`codex.exe` / `claude.exe`) when copying into the temp “bin” dir.
  - Keep helpers “test-only”; do not add new non-test dependencies.
- **Acceptance criteria**:
  - Two tests running in parallel never share a record file or isolated home root.
  - Helpers can run with `--all-features` on macOS/Linux/Windows.
- **Test notes**:
  - Exercise via S2/S3 integration tests (process spawn + record parsing).
- **Risk/rollback notes**: tests-only; low risk.

Checklist:
- Implement: tempdir helpers + copy/hardlink helpers + record parser.
- Test: targeted `cargo test -p agent_api --all-features --test c5_mcp_management_v1`.
- Validate: no global env mutation (set env per backend config / request only).
- Cleanup: keep helper API small and obvious.

#### S1.T3 — Add capability advertising + non-run boundary regression tests (cross-backend)

- **Outcome**: A small set of deterministic regressions that pin the safe posture and non-run boundary without depending on
  backend-specific argv.
- **Inputs/outputs**:
  - Output (suggested):
    - `crates/agent_api/tests/c5_mcp_management_v1.rs`
    - `crates/agent_api/tests/mcp_management_v1/capabilities.rs`
- **Implementation notes**:
  - Capability advertising assertions (pinned):
    - Codex: `list/get` advertised by default; `add/remove` only with `allow_mcp_write=true`.
    - Claude:
      - `list` advertised by default,
      - `get` only advertised on `win32-x64`,
      - `add/remove` only advertised on `win32-x64` **and** with `allow_mcp_write=true`.
  - Non-run boundary assertion:
    - invoking `run(...)` / `run_control(...)` with `extensions["agent_api.tools.mcp.list.v1"]` must fail closed as
      `UnsupportedCapability` (do not spawn; proves it is not modeled as a run extension key).
- **Acceptance criteria**:
  - Tests are deterministic and do not spawn real upstream binaries.
  - All assertions are consistent with `docs/specs/universal-agent-api/mcp-management-spec.md`.
- **Test notes**:
  - Run: `cargo test -p agent_api --all-features --test c5_mcp_management_v1 -- --nocapture`
- **Risk/rollback notes**: tests-only; low risk.

Checklist:
- Implement: capability matrix tests + non-run boundary test.
- Test: `cargo test -p agent_api --all-features --test c5_mcp_management_v1`.
- Validate: ensure target gating uses `cfg!(target_os = \"windows\") && cfg!(target_arch = \"x86_64\")` (pinned `win32-x64`).
- Cleanup: keep assertions minimal and pinned to the spec table.

