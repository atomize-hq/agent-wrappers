### S1 — External sandbox mapping (dangerously-skip-permissions) + pre-spawn validation + warning

- **User/system value**: makes Claude Code deterministic and non-interactive in external sandbox
  mode, unblocking SEAM-5 mapping tests and preventing accidental interactive hangs or ambiguous
  policy precedence.
- **Scope (in/out)**:
  - In:
    - Parse `extensions["agent_api.exec.external_sandbox.v1"]` as boolean (default `false`) and
      validate before spawn.
    - Enforce contradictions when `external_sandbox=true` (pre-spawn):
      - reject `agent_api.exec.non_interactive=false` (ES-C02),
      - reject any `backend.claude_code.exec.*` keys (ES-C06).
    - Emit the pinned warning `Status` event when `external_sandbox=true` is accepted (exact
      message + ordering per `docs/specs/universal-agent-api/extensions-spec.md`).
    - Apply the pinned Claude mapping contract (ES-C05):
      - include `--dangerously-skip-permissions` in argv when `external_sandbox=true`.
    - Apply deterministic allow-flag preflight behavior (ES-C07):
      - run `claude --help` once per backend instance, parse for
        `--allow-dangerously-skip-permissions`, and cache the boolean result,
      - include `--allow-dangerously-skip-permissions` **iff** supported,
      - fail before spawn as `AgentWrapperError::Backend { .. }` when the preflight cannot be
        performed deterministically and the key is requested,
      - do not use a spawn+retry loop.
  - Out:
    - Capability advertising / opt-in gating (SEAM-2).
    - Regression tests (SEAM-5).
- **Acceptance criteria**:
  - When `external_sandbox` is absent/`false`, behavior is unchanged.
  - When `external_sandbox=true` is accepted:
    - argv includes `--dangerously-skip-permissions`,
    - argv includes `--allow-dangerously-skip-permissions` **iff** allow-flag preflight indicates
      support,
    - the backend remains non-interactive (no prompt hangs).
  - `external_sandbox=true` + `agent_api.exec.non_interactive=false` fails before spawn as
    `AgentWrapperError::InvalidRequest` (ES-C02).
  - `external_sandbox=true` + any `backend.claude_code.exec.*` key present fails before spawn as
    `AgentWrapperError::InvalidRequest` (ES-C06).
  - When `external_sandbox=true` is accepted, exactly one warning `Status` event is emitted with:
    - `channel="status"`,
    - `message="DANGEROUS: external sandbox exec policy enabled (agent_api.exec.external_sandbox.v1=true)"`,
    - `data=None`,
    - and emission ordering pinned by `docs/specs/universal-agent-api/extensions-spec.md`.
  - Deterministic behavior:
    - no spawn+retry loops to discover allow-flag support,
    - allow-flag preflight is cached (per backend instance) and re-used across runs.
- **Dependencies**:
  - `SEAM-2` opt-in gating: the key must be supported/advertised only when enabled (ES-C03).
  - Canonical mapping contract: `docs/specs/claude-code-session-mapping-contract.md` (ES-C05/ES-C07).
  - Core key semantics + warning contract: `docs/specs/universal-agent-api/extensions-spec.md`.
- **Verification**:
  - Compile + existing tests: `cargo test -p agent_api claude_code`
  - SEAM-5 adds pinned tests for argv shape, contradictions, warning ordering, and allow-flag
    behavior.
- **Rollout/safety**:
  - Safe-by-default: capability is unreachable unless the host opts in (SEAM-2) and the run
    explicitly requests the key.
  - Deterministic: no spawn+retry loops and no interactive prompts in external sandbox mode.

#### S1.T1 — Add external sandbox policy extraction and contradiction validation

- **Outcome**: the Claude backend extracts `agent_api.exec.external_sandbox.v1` into the run policy
  and enforces ES-C02/ES-C06 contradictions before spawn.
- **Inputs/outputs**:
  - Input: `docs/specs/universal-agent-api/extensions-spec.md` validation + contradiction rules.
  - Output: code changes in `crates/agent_api/src/backends/claude_code.rs`:
    - add `EXT_EXTERNAL_SANDBOX_V1: &str = "agent_api.exec.external_sandbox.v1"`,
    - extend `ClaudeExecPolicy` with `external_sandbox: bool` (default `false`),
    - parse `external_sandbox` as boolean when present,
    - when `external_sandbox=true`:
      - reject explicit `agent_api.exec.non_interactive=false`,
      - reject presence of any supported `backend.claude_code.exec.*` key.
- **Implementation notes**:
  - Apply contradiction checks only after R0 allowlisting (already enforced by the harness).
  - Prefer checking explicit presence of `agent_api.exec.non_interactive` to distinguish
    “absent (default true)” from “explicit false”.
  - For backend exec-policy keys, use a prefix scan (`backend.claude_code.exec.`) rather than
    enumerating specific key tokens.
- **Acceptance criteria**:
  - Meets the slice contradiction-related acceptance criteria.
  - Validation happens in `validate_and_extract_policy(...)` (i.e., pre-spawn).
- **Test notes**:
  - SEAM-5 will pin behavior; for this task, run `cargo test -p agent_api claude_code` for
    regressions.
- **Risk/rollback notes**:
  - Low risk: only affects requests that include the new key (and only when supported via SEAM-2).

Checklist:
- Implement: `external_sandbox` parsing + contradiction checks in `crates/agent_api/src/backends/claude_code.rs`.
- Test: `cargo test -p agent_api claude_code`.
- Validate: ensure R0 ordering is preserved (unsupported keys still fail as `UnsupportedCapability`).

#### S1.T2 — Emit the pinned external sandbox warning `Status` event (ordering-sensitive)

- **Outcome**: when `external_sandbox=true` is accepted, the backend emits exactly one pinned
  warning `Status` event before any other user-visible events, satisfying the observability/audit
  requirement in `docs/specs/universal-agent-api/extensions-spec.md`.
- **Inputs/outputs**:
  - Input: `docs/specs/universal-agent-api/extensions-spec.md` (“Observability / audit signal”).
  - Output: code changes in `crates/agent_api/src/backends/claude_code.rs` (and/or
    `crates/agent_api/src/backends/claude_code/mapping.rs`) that synthesize the warning event.
- **Implementation notes**:
  - Use the existing `mapping::status_event(...)` helper so the event has `channel="status"` and
    `data=None`.
  - Ensure the warning is emitted before any backend-originated events:
    - Option A: in `ClaudeHarnessAdapter.map_event(...)`, if `external_sandbox==true` and
      `warning_emitted==false`, prepend the warning `Status` event to the returned vec and flip the
      `warning_emitted` latch.
    - Option B: in `spawn(...)`, wrap the typed backend event stream with a one-shot synthetic
      warning event before forwarding the real stream.
  - Ordering with session handle facet:
    - The Claude backend attaches the session handle facet to the first `Status` event; therefore,
      emitting the warning before the first mapped backend event ensures the warning precedes the
      session handle facet `Status` event.
  - Non-emission cases:
    - do not emit when the key is absent/`false`,
    - do not emit when R0 gating fails or validation fails (achieved by only enabling the warning
      latch after successful value validation).
- **Acceptance criteria**:
  - Exactly one warning `Status` event is emitted with the pinned message and pinned ordering.
- **Test notes**:
  - SEAM-5 will pin the warning message and ordering (including ordering relative to the session
    handle facet `Status` event).
- **Risk/rollback notes**:
  - Low risk: additive `Status` event emission in a gated, explicitly dangerous mode.

Checklist:
- Implement: a once-per-run warning latch tied to validated `external_sandbox=true`.
- Validate: confirm the warning comes before any `TextOutput` / `ToolCall` / `ToolResult` events and before the session handle facet event.

#### S1.T3 — Deterministic allow-flag preflight (cached) + argv mapping for dangerous permission bypass

- **Outcome**: Claude external sandbox mode deterministically maps to the dangerous permission
  bypass flags, including the version-dependent allow flag, without spawn+retry loops.
- **Inputs/outputs**:
  - Input: `docs/specs/claude-code-session-mapping-contract.md` (external sandbox mapping + preflight).
  - Output: code changes in `crates/agent_api/src/backends/claude_code.rs`:
    - implement a cached `claude --help` preflight (once per backend instance) that returns a
      boolean: whether `--allow-dangerously-skip-permissions` is supported,
    - in `spawn(...)`, when `policy.external_sandbox == true`:
      - set `ClaudePrintRequest::dangerously_skip_permissions(true)`,
      - set `ClaudePrintRequest::allow_dangerously_skip_permissions(true)` **iff** preflight says
        the flag is supported,
      - on preflight failure, fail before spawning the print session as
        `AgentWrapperError::Backend { message }` with a safe/redacted message.
- **Implementation notes**:
  - Prefer `claude_code::ClaudeClient::help()` for the preflight; parse stdout for the literal
    token `--allow-dangerously-skip-permissions`.
  - Cache the preflight outcome (success or failure) to guarantee at-most-once execution per
    backend instance.
  - Keep the help parser a pure function so unit tests can pin behavior without spawning Claude.
  - Ensure surfaced error messages are safe/redacted:
    - do not include stdout/stderr,
    - include exit code when available (e.g., `code=<n> (output redacted)`).
- **Acceptance criteria**:
  - Allow-flag supported → argv includes `--allow-dangerously-skip-permissions`.
  - Allow-flag not supported → argv excludes `--allow-dangerously-skip-permissions`.
  - Preflight failure → run fails before spawn as `AgentWrapperError::Backend { .. }`.
  - No spawn+retry loops for allow-flag discovery.
- **Test notes**:
  - SEAM-5 will unit-test allow-flag included/excluded and preflight-failure behavior.
- **Risk/rollback notes**:
  - Medium risk: introduces a new process preflight; mitigate with caching and fail-closed behavior.

Checklist:
- Implement: cached help preflight + pure `help_supports_allow_flag(&str) -> bool`.
- Implement: map `policy.external_sandbox` to `ClaudePrintRequest::{dangerously_skip_permissions, allow_dangerously_skip_permissions}`.
- Test: `cargo test -p agent_api claude_code`.
- Validate: ensure the preflight runs before spawning the print session and is cached per backend instance.

