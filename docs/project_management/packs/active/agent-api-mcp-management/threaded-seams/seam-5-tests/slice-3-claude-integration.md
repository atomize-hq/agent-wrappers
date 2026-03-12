# S3 — Claude Code MCP integration tests (target-aware + bearer-token rule)

- **User/system value**: Pins Claude Code MCP management behavior end-to-end (including target availability) while keeping
  CI deterministic and offline. Prevents regressions in mapping, gating, and safety posture as the upstream CLI varies by
  platform.
- **Scope (in/out)**:
  - In:
    - Target-aware capability + invocation regressions for Claude MCP operations, pinned to `cli_manifests/claude_code/current.json`:
      - `list` is available broadly,
      - `get/add/remove` are `win32-x64` only (pinned for v1).
    - Hermetic fake-binary integration tests for Claude `mcp_list` on all targets, plus `mcp_get/add/remove` on
      `win32-x64` only.
    - Tests pinning Claude-specific `Url.bearer_token_env_var` behavior:
      - `bearer_token_env_var: Some(_)` MUST be rejected as `InvalidRequest` (fail closed; no spawn).
    - Tests pinning safe-by-default write posture
      (`ClaudeCodeBackendConfig.allow_mcp_write`, default `false`) and isolated home semantics
      (`claude_home`).
  - Out:
    - Backend mapping implementation itself (SEAM-4).
    - Real-network or non-hermetic Claude MCP tests (must remain opt-in).
- **Acceptance criteria**:
  - `mcp_list` invokes: `claude mcp list`.
  - `Url.bearer_token_env_var: Some(_)` is rejected as `InvalidRequest` for Claude (no spawn; pinned).
  - Capability advertising is target-aware and safe-by-default:
    - on non-`win32-x64`: `get/add/remove` are not advertised, and invoking them yields `UnsupportedCapability`.
    - on `win32-x64`: `get` may be advertised; `add/remove` require
      `ClaudeCodeBackendConfig.allow_mcp_write=true` (default remains `false`).
  - Isolated home behavior is pinned:
    - with `claude_home: Some(path)`, fake binary writes sentinels under that root;
    - request env overrides win over injected isolation env (MM-C03/MM-C07).
  - Over-bound stdout/stderr are truncated to 65,536 bytes with suffix `…(truncated)` and flags set (MM-C04).
  - Drift simulation returns `Err(Backend)` and does not mutate capability advertising (pinned).
- **Dependencies**:
  - SEAM-1: validation + output bounds helper (MM-C03/MM-C04/MM-C05).
  - SEAM-2: advertising + `ClaudeCodeBackendConfig.allow_mcp_write` + isolated home config
    (MM-C06/MM-C07).
  - SEAM-4: Claude MCP mapping + drift behavior + bearer-token rejection (MM-C09).
- **Verification**:
  - `cargo test -p agent_api --features codex,claude_code --test c5_mcp_management_v1 -- --nocapture`
- **Rollout/safety**:
  - Fake binaries + isolated homes only; optional live smoke tests must remain `#[ignore]`.

## Atomic Tasks

#### S3.T1 — Add Claude capability + target-gating regression tests (no spawn)

- **Outcome**: Deterministic regressions for Claude capability advertising + invocation gating that:
  - do not spawn any subprocesses, and
  - remain correct across targets.
- **Inputs/outputs**:
  - Output (suggested):
    - `crates/agent_api/tests/mcp_management_v1/claude_capabilities.rs`
  - Inputs:
    - spec: `docs/specs/universal-agent-api/mcp-management-spec.md` (advertising posture table, pinned)
- **Implementation notes**:
  - Pin the `win32-x64` check as:
    - `cfg!(target_os = "windows") && cfg!(target_arch = "x86_64")`
  - Assert:
    - always: `list` advertised,
    - non-`win32-x64`: `get/add/remove` not advertised and gateway returns `UnsupportedCapability` without spawning,
    - `win32-x64`: `get` advertised by default; `add/remove` require
      `ClaudeCodeBackendConfig.allow_mcp_write=true`.
  - Split write assertions into the same two cases used by Codex:
    1) `ClaudeCodeBackendConfig.allow_mcp_write=false` (default) -> `add/remove` stay unadvertised
       and fail closed.
    2) `ClaudeCodeBackendConfig.allow_mcp_write=true` -> `add/remove` can be advertised and
       invoked only on pinned `win32-x64`.
  - Assert capability presence/absence via backend `capabilities().ids`, not the generated
    capability matrix.
- **Acceptance criteria**:
  - Tests pass under `--all-features` on all supported targets.
- **Test notes**:
  - Run: `cargo test -p agent_api --features claude_code --test c5_mcp_management_v1 claude_*`
- **Risk/rollback notes**: low; no subprocess.

Checklist:
- Implement: capability assertions + invocation gating assertions.
- Test: targeted and all-features.
- Validate: ensure the tests do not assume write ops are present on non-`win32-x64`.
- Cleanup: keep the matrix minimal and pinned to the spec table.

#### S3.T2 — Add hermetic Claude integration tests for mapping + isolation + bounds (fake binary)

- **Outcome**: End-to-end tests that spawn the fake `claude` binary and assert:
  - argv mapping for supported operations,
  - isolated home injection and request env override precedence,
  - bounded stdout/stderr + suffix/flags behavior.
- **Inputs/outputs**:
  - Output (suggested):
    - `crates/agent_api/tests/mcp_management_v1/claude_mapping.rs`
  - Inputs:
    - fake binary: `CARGO_BIN_EXE_fake_claude_mcp_agent_api`
    - spec: `docs/specs/universal-agent-api/mcp-management-spec.md` (“Claude Code backend mapping (pinned)”)
- **Implementation notes**:
  - Always run `mcp_list` mapping assertion (all targets).
  - Gate `mcp_get/add/remove` mapping assertions behind the pinned target check (`win32-x64` only).
  - Use per-test isolated `claude_home` tempdir and validate sentinel writes remain beneath it.
  - Include output-bounds coverage by having the fake binary emit > 65,536 bytes to stdout/stderr and asserting truncation.
- **Acceptance criteria**:
  - Mapping assertions are byte-for-byte pinned to the canonical spec.
  - Isolation is observable via records + sentinel writes, and request env overrides are honored.
- **Test notes**:
  - Run: `cargo test -p agent_api --features claude_code --test c5_mcp_management_v1 claude_* -- --nocapture`
- **Risk/rollback notes**: low; deterministic fake process + tempdirs.

Checklist:
- Implement: list mapping test + win32-x64 gated mapping tests.
- Test: all-features and claude-only.
- Validate: ensure no test invokes a real installed `claude` binary by default.
- Cleanup: keep record assertions narrow (args/env/cwd + sentinel presence).

#### S3.T3 — Pin Claude-specific rejection + failure modes (`bearer_token_env_var`, drift, timeout) with “no leaks”

- **Outcome**: Regressions that pin Claude-specific semantics and hard edges:
  - `Url.bearer_token_env_var: Some(_)` rejected as `InvalidRequest` (no spawn),
  - drift-style stderr causes `Err(Backend)` (pinned),
  - timeout causes `Err(Backend)` with no partial-output leakage in the error message.
- **Inputs/outputs**:
  - Output (suggested):
    - `crates/agent_api/tests/mcp_management_v1/claude_failures.rs`
- **Implementation notes**:
  - For bearer-token rejection:
    - configure the backend such that if a subprocess were spawned it would write a record, then assert the record file is
      absent after the call (proves validate-before-spawn).
  - For drift/timeout:
    - use fake-binary scenarios to emit a unique sentinel to stdout/stderr, then assert the returned `Backend.message`
      does not contain the sentinel (no partial-output leakage).
- **Acceptance criteria**:
  - Tests are deterministic and target-aware.
- **Test notes**:
  - Run: `cargo test -p agent_api --features claude_code --test c5_mcp_management_v1 claude_* -- --nocapture`
- **Risk/rollback notes**: medium risk (timeout flake); mitigate with conservative timeouts + short sleeps.

Checklist:
- Implement: bearer-token rejection regression + drift regression + timeout regression.
- Test: repeated targeted runs to detect flake early.
- Validate: error messages never echo raw stdout/stderr.
- Cleanup: keep sentinel strings unique and asserted absent from errors.

#### S3.T4 — (Optional) Add opt-in live smoke tests (`#[ignore]` + `AGENT_API_MCP_LIVE=1`)

- **Outcome**: A minimal set of `#[ignore]` tests that can run against a real installed `claude` binary (and optionally
  `codex`) under isolated homes, without requiring network access.
- **Inputs/outputs**:
  - Output (suggested):
    - `crates/agent_api/tests/mcp_management_v1/live_smoke.rs`
  - Input: `docs/specs/universal-agent-api/mcp-management-spec.md` (“Integration coverage + gating (pinned)”)
- **Implementation notes**:
  - Keep smoke tests extremely small:
    - prefer `list/get/add/remove` only,
    - use isolated homes only,
    - no networked MCP servers.
  - Gate by:
    - `#[ignore]`, and
    - `AGENT_API_MCP_LIVE=1` plus configured binary path selection.
- **Acceptance criteria**:
  - Smoke tests never run in CI by default and do not require installed binaries unless explicitly opted in.
- **Test notes**:
  - Run (opt-in): `AGENT_API_MCP_LIVE=1 cargo test -p agent_api --all-features --test c5_mcp_management_v1 -- --ignored`
- **Risk/rollback notes**: optional; keep separate from default hermetic coverage.

Checklist:
- Implement: ignored tests + env gating + isolated home tempdirs.
- Test: local opt-in only.
- Validate: no network required; no user-state mutation.
- Cleanup: keep separate from default test module paths.
