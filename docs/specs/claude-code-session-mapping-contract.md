# Claude Code Session Mapping Contract (v1)

Status: **Normative**  
Scope: concrete `claude --print` spawn/mapping rules for Universal Agent API session semantics
(`agent_api.session.resume.v1`, `agent_api.session.fork.v1`) and non-interactive policy
(`agent_api.exec.non_interactive`).

## Normative language

This document uses RFC 2119-style requirement keywords (`MUST`, `MUST NOT`, `SHOULD`).

## Purpose

The Universal Agent API specs define the **backend-neutral** semantics and validation rules for:

- `agent_api.exec.non_interactive` (boolean)
- `agent_api.session.resume.v1` (object)
- `agent_api.session.fork.v1` (object)

This document defines the **Claude Code backend-owned** and testable mapping from those extension
keys into a headless Claude CLI invocation, including the pinned non-interactive behavior and safe
error translation requirements.

Canonical schemas + selection-failure rules are owned by:
- `docs/specs/universal-agent-api/extensions-spec.md`

## CLI surface (pinned)

All session resume/fork mappings MUST use Claude Code headless print mode:

- base tokens: `claude --print --output-format stream-json`
- prompt placement: `PROMPT` is a single positional argv token and MUST be the **final** argv token.

Stream-json verbose requirement (pinned):
- When `--output-format stream-json` is used, the argv MUST include `--verbose`.
- Rationale/evidence: `crates/claude_code` enforces this upstream requirement (it injects `--verbose`
  for stream-json output to avoid a non-zero exit).

## `agent_api.exec.non_interactive` mapping (pinned)

Default policy is owned by `docs/specs/universal-agent-api/extensions-spec.md`:
`agent_api.exec.non_interactive` defaults to `true`.

When `agent_api.exec.non_interactive == true`, the Claude backend MUST:

- include `--permission-mode bypassPermissions` in argv, and
- MUST NOT fall back to an interactive invocation if the CLI rejects this flag/value.

If the CLI exits non-zero due to rejecting the pinned non-interactive flag/value, the backend MUST
fail the run as `AgentWrapperError::Backend { message }` with a safe/redacted `message` (MUST NOT
embed raw backend output).

## `agent_api.exec.external_sandbox.v1` mapping (dangerous; pinned)

The external sandbox extension key (`agent_api.exec.external_sandbox.v1`) is owned by the universal
extensions registry (`docs/specs/universal-agent-api/extensions-spec.md`). This section pins the
Claude Code backend-owned CLI mapping when the Claude backend advertises and accepts the key.

When `extensions["agent_api.exec.external_sandbox.v1"] == true`, the Claude backend MUST:

- remain non-interactive (so the `agent_api.exec.non_interactive` mapping still applies), and
- include `--dangerously-skip-permissions` in argv.

Additionally, the backend MUST include `--allow-dangerously-skip-permissions` **iff** the installed
Claude CLI supports that flag.

Deterministic allow-flag preflight requirement (pinned):

- The backend MUST determine allow-flag support pre-spawn and MUST NOT spawn a session and then
  retry with a different argv.
- Pinned strategy:
  - run `claude --help` once per backend instance,
  - parse stdout for the literal token `--allow-dangerously-skip-permissions`, and
  - cache the resulting boolean for subsequent runs.
- If the preflight cannot be performed deterministically (non-zero exit, timeout, missing binary,
  etc.) and the key is requested, the backend MUST fail the run **before spawning** as
  `AgentWrapperError::Backend { message }` with a safe/redacted `message`.

Implementation note (non-normative): keep the `--help` parser a pure function so unit tests can pin
both the allow-flag-supported and allow-flag-not-supported cases without spawning Claude.

## `agent_api.session.resume.v1` mapping (pinned)

The resume extension key is owned by `docs/specs/universal-agent-api/extensions-spec.md`. This
section pins the Claude CLI mapping.

Let `PROMPT == AgentWrapperRunRequest.prompt` (non-empty after trimming; validated pre-spawn by the
Universal Agent API run protocol).

### selector `"last"`

The backend MUST spawn an argv containing the following **ordered subsequence**:

`--print --output-format stream-json [--permission-mode bypassPermissions] --continue --verbose PROMPT`

### selector `"id"`

Let `ID == extensions["agent_api.session.resume.v1"].id` (non-empty after trimming).

The backend MUST spawn an argv containing the following **ordered subsequence**:

`--print --output-format stream-json [--permission-mode bypassPermissions] --resume ID --verbose PROMPT`

## `agent_api.session.fork.v1` mapping (pinned)

The fork extension key is owned by `docs/specs/universal-agent-api/extensions-spec.md`. This
section pins the Claude CLI mapping.

Let `PROMPT == AgentWrapperRunRequest.prompt` (non-empty after trimming; validated pre-spawn).

### selector `"last"`

The backend MUST spawn an argv containing the following **ordered subsequence**:

`--print --output-format stream-json [--permission-mode bypassPermissions] --continue --fork-session --verbose PROMPT`

### selector `"id"`

Let `ID == extensions["agent_api.session.fork.v1"].id` (non-empty after trimming).

The backend MUST spawn an argv containing the following **ordered subsequence**:

`--print --output-format stream-json [--permission-mode bypassPermissions] --fork-session --resume ID --verbose PROMPT`

## Error translation requirements (pinned)

This contract does not define the complete Claude error taxonomy. It pins the **universal**
translation requirements the Claude backend MUST satisfy.

### Selection failures (resume/fork)

For `agent_api.session.resume.v1` and `agent_api.session.fork.v1`, selection-failure behavior and
pinned safe messages are owned by `docs/specs/universal-agent-api/extensions-spec.md`.

The Claude backend MUST:

- surface selection failures as `AgentWrapperError::Backend { message }` with `message` exactly
  equal to the pinned strings from `extensions-spec.md`:
  - `"no session found"` for `selector == "last"`
  - `"session not found"` for `selector == "id"`
- MUST NOT translate a generic resume/fork failure into the pinned selection-failure messages just
  because the run produced no assistant message.
- MUST NOT embed raw Claude output (stdout/stderr) in error messages or in
  `AgentWrapperEvent.data` / `AgentWrapperCompletion.data`.

### Other backend failures

All other Claude backend failures (spawn failures, parse failures, non-zero exit unrelated to
selection) MUST be surfaced as `AgentWrapperError::Backend { message }` with a safe/redacted
message (MUST NOT embed raw Claude output).
