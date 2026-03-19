# S1c — Cross-backend regressions (capability posture + non-run boundary)

- **User/system value**: Pins the “safety posture” invariants (capability gating, safe default advertising, target gating,
  and non-run boundary) without relying on backend-specific argv mapping details.
- **Scope (in/out)**:
  - In:
    - Deterministic regression tests for default capability advertising posture across backends.
    - Claude target gating regression (pinned `win32-x64` only for `get/add/remove`).
    - Non-run boundary regression: MCP management capability ids are *not* run extension keys (fail closed; no spawn).
  - Out:
    - Backend-specific argv mapping tests (S2/S3).
    - Any tests requiring a real networked MCP server.
- **Acceptance criteria**:
  - Advertising assertions match `docs/specs/universal-agent-api/mcp-management-spec.md` table:
    - Codex: `list/get` by default; `add/remove` only with `CodexBackendConfig.allow_mcp_write=true`.
    - Claude: `list` by default; `get` only on `win32-x64`; `add/remove` only on `win32-x64`
      and with `ClaudeCodeBackendConfig.allow_mcp_write=true`.
  - Advertising assertions use backend instance `capabilities().ids`; the generated default
    capability matrix may omit `agent_api.tools.mcp.add.v1` / `.remove.v1` because default configs
    leave `allow_mcp_write=false`.
  - Non-run boundary: `extensions["agent_api.tools.mcp.list.v1"]` (and peers) must be rejected as
    `UnsupportedCapability` without spawning any subprocess.
- **Dependencies**:
  - SEAM-1 capability ids + gateway error type (`UnsupportedCapability`) (MM-C01/MM-C02).
  - SEAM-2 advertising + `allow_mcp_write` + isolated-home fields (MM-C06/MM-C07).
  - Canonical spec: `docs/specs/universal-agent-api/mcp-management-spec.md`.
- **Verification**:
  - `cargo test -p agent_api --all-features --test c5_mcp_management_v1 -- --nocapture`
- **Rollout/safety**:
  - Offline/deterministic; should not spawn fake binaries for gating failures (validate-before-spawn).

## Atomic Tasks (moved from S1)

#### S1.T3 — Add capability advertising + non-run boundary regression tests (cross-backend)

- **Outcome**: A small set of deterministic regressions that pin the safe posture and non-run boundary without depending
  on backend-specific argv.
- **Files** (suggested):
  - `crates/agent_api/tests/c5_mcp_management_v1.rs`
  - `crates/agent_api/tests/mcp_management_v1/capabilities.rs`

Checklist:
- Implement:
  - Capability matrix assertions (Codex + Claude; read vs write posture; pinned).
  - Target gating assertions for `win32-x64` as `cfg!(target_os = "windows") && cfg!(target_arch = "x86_64")`.
  - Non-run boundary test: run extensions must reject MCP management capability ids with `UnsupportedCapability` and no spawn.
- Test:
  - Run `cargo test -p agent_api --all-features --test c5_mcp_management_v1 -- --nocapture`.
- Validate:
  - Ensure tests remain independent of backend-specific argv mapping (owned by S2/S3).
  - Ensure “no spawn” is observable (e.g., record file remains absent when rejecting).
