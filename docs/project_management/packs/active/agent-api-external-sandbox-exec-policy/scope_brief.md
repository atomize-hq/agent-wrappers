# Scope brief — External sandbox execution policy (dangerous)

## Goal

Introduce a new **core extension key**:

- `agent_api.exec.external_sandbox.v1` (boolean)

…to let **externally sandboxed hosts** (e.g., Substrate) explicitly request that a built-in backend
relax internal approvals/sandbox/permissions guardrails **without ever becoming interactive**.

## Why now

Some orchestrators already provide strong isolation externally and need the CLI backend to avoid
internal prompting/guardrails that conflict with unattended automation.

## Primary users + JTBD

- **Host integrators / orchestrators**: "Run the built-in backend in a mode compatible with external
  sandboxing (no prompts; explicit opt-in to dangerous execution policy)."

## In-scope

- Define `agent_api.exec.external_sandbox.v1` in `docs/specs/universal-agent-api/extensions-spec.md`
  (schema + defaults + validation + contradiction rules + mapping requirements).
- Implement capability-gated support in:
  - `crates/agent_api/src/backends/codex.rs`
  - `crates/agent_api/src/backends/claude_code.rs`
- Ensure **validated before spawn** (fail-closed) and **non-interactive** behavior.
- Ensure the capability is **not advertised by default** in built-in backends; only enabled for
  externally sandboxed hosts via explicit backend configuration.

## Out-of-scope

- Providing (or validating) the external sandbox itself.
- Changing the default safety posture of built-in backends when the key is absent.
- Advertising this dangerous capability in the generated capability matrix by default.
- Adding new universal exec-policy keys beyond this specific opt-in.

## Capability inventory (implied)

- New core extension key: `agent_api.exec.external_sandbox.v1` (boolean; validated pre-spawn).
- Capability-gated adoption in Codex + Claude Code built-in backends.
- Explicit host opt-in path so the capability is not advertised by default.
- Deterministic mapping to underlying CLI "danger bypass" flags (Codex + Claude).
- Contradiction handling with `agent_api.exec.non_interactive` (fail closed).
- Regression tests for default advertising + validation ordering + mapping.

## Required invariants (must not regress)

- **Fail-closed extension gating**: unknown extension keys continue to fail with
  `UnsupportedCapability` before any extension value validation (R0).
- **Validation before spawn**: the key is type-checked and contradiction-checked before any backend
  process is started.
- **No interactive hangs**: when `agent_api.exec.external_sandbox.v1 == true`, the backend MUST NOT
  hang on approvals/permissions prompts.
- **Contradictory intent fails closed**: when `agent_api.exec.external_sandbox.v1 == true` and
  `agent_api.exec.non_interactive == false` is explicitly requested, the backend MUST fail before
  spawn with `AgentWrapperError::InvalidRequest` as contradictory intent (exact error pinned in SEAM-1).

## Success criteria

- A request with `extensions["agent_api.exec.external_sandbox.v1"] = true`:
  - fails with `UnsupportedCapability` by default (built-in backends do not advertise it), and
  - succeeds when the host explicitly enables the capability on the backend instance.
- When enabled and requested:
  - Codex backend maps to `--dangerously-bypass-approvals-and-sandbox` (pinned; via
    `CodexClientBuilder::dangerously_bypass_approvals_and_sandbox(true)` for exec/resume; see
    `docs/specs/codex-external-sandbox-mapping-contract.md` for fork/app-server details).
  - Claude Code backend maps to `claude --print --dangerously-skip-permissions ...` (plus any
    required opt-in flag depending on CLI version).
- Contradiction with `agent_api.exec.non_interactive=false` fails before spawn.
- New tests pin the behavior and prevent accidental default advertising.

## Constraints

- Value MUST be boolean and validated before spawn.
- This key is explicitly dangerous and MUST NOT be advertised by default in built-in backends
  (host opt-in via `allow_external_sandbox_exec`).
- When requested, the backend MUST remain non-interactive (MUST NOT hang).

## External systems / dependencies

- Codex CLI behavior behind `--dangerously-bypass-approvals-and-sandbox`.
- Claude CLI behavior behind `--dangerously-skip-permissions` and any accompanying "allow" flag.

## Known unknowns / risks

- **Claude CLI allow-flag handling**: pinned to a deterministic `claude --help` preflight (cached),
  with failure before spawn when preflight cannot be performed. (Canonical:
  `docs/specs/claude-code-session-mapping-contract.md`.)
- **Exec-policy interactions / precedence**: pinned to “reject `backend.<agent_kind>.exec.*` keys
  when `agent_api.exec.external_sandbox.v1 == true`” to avoid ambiguous precedence in a dangerous
  surface. (Canonical: `docs/specs/universal-agent-api/extensions-spec.md`.)
- **Footgun risk**: ensuring the opt-in path is explicit enough that this does not become a default
  escape hatch for non-sandboxed hosts.

## Assumptions (explicit)

- Built-in backends will gate support via `allow_external_sandbox_exec: bool` (default `false`) on
  each backend config so capability advertising remains off by default.
- Hosts that need this behavior will:
  1) enable the backend capability explicitly, then
  2) set the per-run extension key explicitly.
