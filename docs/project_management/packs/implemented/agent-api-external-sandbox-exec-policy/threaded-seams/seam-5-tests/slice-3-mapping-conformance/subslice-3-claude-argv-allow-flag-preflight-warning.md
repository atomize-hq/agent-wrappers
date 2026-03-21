### S3c — Claude argv mapping + allow-flag preflight + warning ordering

- **User/system value**: locks in deterministic Claude Code external-sandbox argv mapping (danger
  flags + cached allow-flag preflight) and ensures the required warning is emitted before other
  user-visible events.
- **Scope (in/out)**:
  - In:
    - Claude argv mapping tests when `agent_api.exec.external_sandbox.v1=true` is accepted.
    - allow-flag supported/unsupported behavior driven by deterministic cached `claude --help`
      preflight output (ES-C07).
    - preflight failure behavior: fail as `AgentWrapperError::Backend { .. }` before spawning
      `--print` (ES-C07).
    - warning `Status` event message + ordering per
      `docs/specs/universal-agent-api/extensions-spec.md`.
  - Out:
    - Codex argv mapping (covered by `S3a`).
    - Codex fork/app-server JSON-RPC mapping (covered by `S3b`).
- **Acceptance criteria**:
  - When `external_sandbox=true`, argv includes `--dangerously-skip-permissions`.
  - When allow-flag is supported, argv includes `--allow-dangerously-skip-permissions`.
  - When allow-flag is not supported, argv excludes `--allow-dangerously-skip-permissions`.
  - When `claude --help` preflight fails, wrapper returns `AgentWrapperError::Backend { .. }`
    before attempting to spawn `--print`.
  - Exactly one warning `Status` event with the pinned message is emitted when
    `external_sandbox=true` is accepted.
  - Warning is emitted before the session handle facet `Status` event.
- **Dependencies**:
  - `docs/specs/claude-code-session-mapping-contract.md` (ES-C05/ES-C07).
  - `docs/specs/universal-agent-api/extensions-spec.md` (warning contract).
  - SEAM-2 opt-in gate (`allow_external_sandbox_exec`) + SEAM-4 Claude mapping implementation.
- **Verification**:
  - `cargo test -p agent_api claude_code`
- **Rollout/safety**:
  - Tests + fake binary only.

#### S3.T3 — Claude mapping tests (argv + allow-flag preflight + warning ordering)

- **Outcome**: Claude external sandbox mode is pinned to ES-C05/ES-C07 with deterministic argv,
  deterministic allow-flag behavior, and pinned warning ordering.
- **Files**:
  - `crates/agent_api/tests/` (new Claude exec-policy integration test module)
  - `crates/agent_api/src/bin/fake_claude_stream_json_agent_api.rs` (env-gated help/preflight +
    argv assertions)

Checklist:
- Implement:
  - Add integration tests for:
    - allow-flag supported (`--help` stdout contains token) → argv includes allow-flag
    - allow-flag unsupported (`--help` stdout lacks token) → argv excludes allow-flag
    - `--help` preflight failure → `AgentWrapperError::Backend { .. }` before spawning `--print`
  - Assert argv always includes `--dangerously-skip-permissions` when `external_sandbox=true`.
  - Assert the warning `Status` event is emitted exactly once, with pinned message + ordering.
  - Extend the fake Claude binary to:
    - return controlled `--help` output (include/exclude token),
    - fail `--help` deterministically for failure-path coverage,
    - assert `--print` argv contains/excludes required flags (scenario/ENV-gated).
- Test:
  - `cargo test -p agent_api claude_code`
- Validate:
  - In the preflight-failure test, include a secret sentinel in fake output and assert wrapper
    error redaction (does not contain the secret).
  - Keep fake-binary assertions scenario/ENV-gated so existing scenarios remain stable.

