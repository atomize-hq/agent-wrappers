# SEAM-4 — Claude Code backend mapping

- **Name**: Claude Code mapping for `agent_api.exec.external_sandbox.v1`
- **Type**: capability (backend mapping) + integration (CLI version differences)
- **Goal / user value**: when enabled + requested, run Claude Code in a mode compatible with
  external sandboxing by relaxing internal permission guardrails without prompting.

## Scope

- In:
  - Validate the new key (boolean) before spawn.
  - Enforce the non-interactive invariant and contradiction rule with
    `agent_api.exec.non_interactive`.
  - Map `agent_api.exec.external_sandbox.v1 == true` to:
    - `claude --print --dangerously-skip-permissions ...`
  - Ensure the required "allow" flag behavior is deterministic pre-spawn:
    - enable `--allow-dangerously-skip-permissions` when supported by the installed CLI version.
    - pinned strategy: preflight by running `claude --help` once per backend instance, parse for the
      allow flag token, and cache the boolean result (no spawn+retry loop).
    - canonical mapping + preflight contract:
      - `docs/specs/claude-code-session-mapping-contract.md`
- Out:
  - Expanding Claude wrapper semantics beyond permission bypass (not requested).

## Primary interfaces (contracts)

- **Input**: `extensions["agent_api.exec.external_sandbox.v1"] == true` (when capability is enabled)
- **Output**: Claude CLI invocation includes the dangerous permission bypass flags and remains non-interactive.

## Key invariants / rules

- MUST NOT hang on prompts.
- MUST be validated before spawn.
- MUST fail before spawn with `AgentWrapperError::InvalidRequest` on explicit contradiction with
  `agent_api.exec.non_interactive == false`.
- Mapping must be deterministic across CLI versions (no "spawn then retry with different flags").

## Dependencies

- Blocks: SEAM-5 (tests).
- Blocked by: SEAM-1 (semantics) + SEAM-2 (enablement).

## Touch surface

- `crates/agent_api/src/backends/claude_code.rs`
- `crates/agent_api/src/backends/claude_code/tests.rs`
- v1 decision: keep allow-flag detection + caching **local** to the Claude backend (no shared helper).
- `crates/claude_code/src/commands/print.rs` already supports
  `dangerously_skip_permissions(...)` + `allow_dangerously_skip_permissions(...)`.

## Verification

- Unit tests that pin:
  - default capabilities do not advertise the key,
  - contradiction behavior (`external_sandbox=true` + `non_interactive=false`) fails pre-spawn, and
  - argv includes `--dangerously-skip-permissions` (and includes/excludes the allow flag per the
    pinned `claude --help` preflight strategy), including:
    - allow-flag supported → argv includes `--allow-dangerously-skip-permissions`
    - allow-flag not supported → argv excludes `--allow-dangerously-skip-permissions`
    - preflight failure (help cannot be run) → fail before spawn as `AgentWrapperError::Backend { .. }`

## Risks / unknowns

- None (pinned: `claude --help` preflight + cached allow-flag support check).

## Rollout / safety

- Only reachable behind explicit host opt-in (SEAM-2).
