# S1 — Safe default advertising + write enablement gating

- **User/system value**: Ensures built-in backends present a safe, discoverable MCP management surface:
  - read operations are advertised when available on this target, and
  - write operations are never advertised unless explicitly enabled.
- **Scope (in/out)**:
  - In:
    - Add host-provided config knobs:
      - `CodexBackendConfig.allow_mcp_write: bool` (default `false`)
      - `ClaudeCodeBackendConfig.allow_mcp_write: bool` (default `false`)
    - Implement pinned advertising precedence for built-in backends:
      1) determine upstream subcommand availability from pinned CLI manifest snapshots,
      2) apply config enablement (`add/remove` require `allow_mcp_write == true`),
      3) advertise capability ids iff enabled.
    - Fail closed: if target availability cannot be established from the pinned snapshot, do not advertise.
  - Out:
    - MCP argv mapping + process execution for Codex/Claude (SEAM-3/4).
    - Cross-backend conformance tests/harness (SEAM-5).
- **Acceptance criteria**:
  - Defaults are safe:
    - both built-in backends advertise `mcp add/remove` capability ids only when `allow_mcp_write == true`.
  - Advertising follows the pinned default posture in `docs/specs/universal-agent-api/mcp-management-spec.md`:
    - Codex: `list/get` advertised by default when available on this target; `add/remove` gated by `allow_mcp_write`.
    - Claude: `list` advertised by default when available on this target; `get/add/remove` are `win32-x64` only and
      `add/remove` are additionally gated by `allow_mcp_write`.
  - Capability advertising is deterministic and does not depend on runtime probing of the upstream binary.
- **Dependencies**:
  - SEAM-1: capability id strings (MM-C01) and gateway capability gating surface.
- **Verification**:
  - `cargo test -p agent_api --features codex,claude_code` (unit tests pin advertising behavior)
- **Rollout/safety**:
  - Safe-by-default posture: write ops remain disabled unless explicitly enabled.
  - No run event surface changes (MCP management is non-run per MM-C02).

## Atomic Tasks

#### S1.T1 — Add `allow_mcp_write` to built-in backend configs (default false)

- **Outcome**: Both built-in backends expose an explicit, host-controlled enablement knob that defaults safe.
- **Inputs/outputs**:
  - Input: `docs/specs/universal-agent-api/mcp-management-spec.md` (“Safety posture” + “Default capability advertising posture”)
  - Output:
    - `crates/agent_api/src/backends/codex.rs`: `CodexBackendConfig.allow_mcp_write: bool`
    - `crates/agent_api/src/backends/claude_code.rs`: `ClaudeCodeBackendConfig.allow_mcp_write: bool`
- **Implementation notes**:
  - Keep defaults pinned (`false`).
  - Treat this as a host-provided knob only (no request-level override).
- **Acceptance criteria**:
  - Existing construction sites compile unchanged (defaulted field), and the default remains safe.
- **Test notes**:
  - No tests required for “field exists,” but later tasks assert the default advertising posture.
- **Risk/rollback notes**: additive config fields; safe.

Checklist:
- Implement: add config fields + update `Default` impls where present.
- Test: `cargo check -p agent_api --features codex,claude_code`.
- Validate: confirm defaults are `false`.
- Cleanup: rustfmt.

#### S1.T2 — Implement pinned target availability checks for MCP subcommands (built-in backends)

- **Outcome**: Advertising logic has a single source of truth for “is this op available on this target?” derived from the
  pinned CLI manifest snapshots (no runtime probing).
- **Inputs/outputs**:
  - Input: `cli_manifests/codex/current.json`, `cli_manifests/claude_code/current.json` (pinned availability)
  - Output: internal helper functions (location flexible) used by both backends’ advertising logic.
- **Implementation notes**:
  - Prefer small `cfg(...)`-based helpers that match the pinned manifest target identifiers:
    - Claude uses `linux-x64`, `darwin-arm64`, `win32-x64` with `get/add/remove` only on `win32-x64`.
    - Codex snapshot is currently incomplete; fail closed on non-covered targets (do not advertise).
  - Keep the helpers private to `agent_api` (not part of the public MCP API).
- **Acceptance criteria**:
  - Helpers encode the pinned availability table exactly and are used by advertising logic (S1.T3).
- **Test notes**:
  - Use `#[cfg(...)]`-gated unit tests to pin target-specific behavior (Windows vs non-Windows expectations for Claude).
- **Risk/rollback notes**: low; behavior is new + safe-by-default.

Checklist:
- Implement: add helper fns for “op available on this target?” per backend.
- Test: target-gated unit tests where appropriate.
- Validate: no runtime binary probing (no `--help` parsing, no version probing).
- Cleanup: keep helpers small and well-named.

#### S1.T3 — Wire MCP capability advertising into `capabilities()` for Codex + Claude backends

- **Outcome**: Built-in backends advertise MCP management capability ids iff enabled (pinned precedence).
- **Inputs/outputs**:
  - Input: MM-C01 capability ids (SEAM-1)
  - Output:
    - `crates/agent_api/src/backends/codex.rs`: include MCP capability ids in `capabilities().ids` based on availability + config
    - `crates/agent_api/src/backends/claude_code.rs`: include MCP capability ids in `capabilities().ids` based on availability + config
- **Implementation notes**:
  - Apply pinned precedence:
    - `list/get`: enabled iff available on this target.
    - `add/remove`: enabled iff available on this target **and** `allow_mcp_write == true`.
  - Avoid duplicating string literals: reuse SEAM-1 capability id constants if available.
  - Keep the “MCP block” in `capabilities()` tightly scoped to reduce conflict with SEAM-3/4 mapping edits.
- **Acceptance criteria**:
  - With default configs, `add/remove` capability ids are never advertised.
  - With `allow_mcp_write=true`, `add/remove` capability ids appear only when available on this target.
  - Claude `get/add/remove` capability ids are not advertised on non-`win32-x64` targets.
- **Test notes**:
  - Unit tests assert capability presence/absence for default and enabled configs.
- **Risk/rollback notes**:
  - Behavior is additive and safe-by-default; rollback is removing MCP capability ids from `capabilities()`.

Checklist:
- Implement: insert MCP capability ids per pinned table + precedence.
- Test: `cargo test -p agent_api --features codex,claude_code`.
- Validate: ensure write ops remain disabled unless enabled.
- Cleanup: rustfmt.

#### S1.T4 — Add unit tests pinning the default advertising table + enablement gating

- **Outcome**: Deterministic unit tests that fail if advertising posture regresses.
- **Inputs/outputs**:
  - Input: `docs/specs/universal-agent-api/mcp-management-spec.md` (“Default capability advertising posture (pinned)”)
  - Output: unit tests under `crates/agent_api/src/backends/*` or a shared test module.
- **Implementation notes**:
  - Pin at minimum:
    - default `allow_mcp_write=false` → no `add/remove` advertised,
    - `allow_mcp_write=true` → `add/remove` advertised only when target-available,
    - Claude `get/add/remove` only on `win32-x64` (target-gated assertions).
  - Keep tests pure (no subprocess spawning).
- **Acceptance criteria**:
  - Tests cover both backends and enforce the pinned table.
- **Test notes**:
  - Run: `cargo test -p agent_api --features codex,claude_code`.
- **Risk/rollback notes**: tests-only; safe.

Checklist:
- Implement: table-driven capability assertions.
- Test: `cargo test -p agent_api --features codex,claude_code`.
- Validate: confirm tests are target-aware (no false positives on non-Windows CI).
- Cleanup: keep tests narrowly scoped to advertising semantics.

## Notes for downstream seams (non-tasking)

- SEAM-3/4 hook implementations should still fail-closed when an op is unadvertised (gateway should prevent invocation),
  but SHOULD NOT introduce ad hoc extra advertising rules beyond the precedence pinned here.
