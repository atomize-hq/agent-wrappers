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
  etc.) and the key is requested, the backend MUST fail the run with
  `AgentWrapperError::Backend { message }` using a safe/redacted `message`.
- If that preflight failure is discovered before the backend returns a run handle, it MUST be
  returned directly.
- If that preflight failure is discovered during asynchronous startup after the backend has already
  returned a run handle, it MUST be surfaced through the handle as a terminal
  `AgentWrapperEventKind::Error` event plus the matching completion error, and Claude MUST still
  not spawn a session.

Implementation note (non-normative): keep the `--help` parser a pure function so unit tests can pin
both the allow-flag-supported and allow-flag-not-supported cases without spawning Claude.

## `agent_api.exec.add_dirs.v1` mapping (pinned)

The add-dir extension key is owned by `docs/specs/universal-agent-api/extensions-spec.md`. This
section pins the Claude CLI mapping when the Claude backend advertises and accepts the key.

When `extensions["agent_api.exec.add_dirs.v1"]` is accepted, the Claude backend MUST:

- emit exactly one `--add-dir <DIR...>` argv group,
- include the normalized unique directories in order, and
- keep the group in the root-flags region before the final prompt token.

Placement rules (pinned):

- Fresh run argv MUST contain the following ordered subsequence:

`--print --output-format stream-json [--model ID] [--permission-mode bypassPermissions] [--dangerously-skip-permissions] [--allow-dangerously-skip-permissions] [--add-dir <DIR...>] --verbose PROMPT`

- The group MUST appear after any accepted `--model <trimmed-id>` pair.
- The group MUST appear before `--continue`, `--fork-session`, and `--resume`.
- The group MUST appear before the final `--verbose` token that precedes the prompt.
- The backend MUST NOT emit repeated `--add-dir` flags for this key.

Verification requirements (pinned):

- Regression coverage MUST assert fresh-run ordering and the selector-specific ordered subsequences
  below.
- The verification surface MUST fail if the `--add-dir <DIR...>` group drifts to the right of any
  session-selector flag, the final `--verbose` token, or the final prompt token.

Runtime rejection parity (pinned):

- If an accepted `agent_api.exec.add_dirs.v1` value is rejected after the Claude backend has
  already returned a run handle and the consumer-visible events stream is still open, the backend
  MUST:
  - fail the run as `AgentWrapperError::Backend { message }`,
  - use the backend-owned safe/redacted `message` exactly equal to
    `add_dirs rejected by runtime`,
  - emit exactly one terminal `AgentWrapperEventKind::Error` event carrying that same
    safe/redacted `message`, and
  - close the stream after emitting that terminal error event.
- The safe/redacted `message` in the terminal error event MUST exactly match the safe/redacted
  `message` surfaced through the completion error so downstream consumers can compare them
  deterministically.
- The backend MUST NOT classify selector misses (`"no session found"` / `"session not found"`) as
  add-dir runtime rejection.

## `agent_api.config.model.v1` mapping (pinned)

The model-selection extension key is owned by `docs/specs/universal-agent-api/extensions-spec.md`.
This section pins the Claude CLI mapping when the Claude backend advertises and accepts the key.

When `extensions["agent_api.config.model.v1"]` is accepted, the Claude backend MUST:

- emit exactly one `--model <trimmed-id>` pair,
- use the effective trimmed model id from the universal extension contract, and
- omit `--model` entirely when the key is absent.

Placement rules (pinned):

- The pair MUST appear before any `--add-dir` group.
- The pair MUST appear before `--continue`, `--fork-session`, and `--resume`.
- The pair MUST appear before any `--fallback-model` flag/value.
- The pair MUST appear before the final `--verbose` token that precedes the prompt.

Verification requirements (pinned):

- Regression coverage MUST assert this ordering in fresh print argv construction and in the
  resume/fork session argv subsequences defined below.
- The verification surface MUST fail if `--model <trimmed-id>` drifts to the right of the final
  `--verbose` token or any forbidden flag region listed above.

Runtime rejection parity (pinned):

- If an accepted `agent_api.config.model.v1` value is rejected after the Claude backend has already
  returned a run handle and the consumer-visible events stream is still open, the backend MUST:
  - fail the run as `AgentWrapperError::Backend { message }`,
  - emit exactly one terminal `AgentWrapperEventKind::Error` event carrying that same safe/redacted
    `message`, and
  - close the stream after emitting that terminal error event.
- The safe/redacted `message` in the terminal error event MUST exactly match the safe/redacted
  `message` surfaced through the completion error so downstream consumers can compare them
  deterministically.
- This event/completion parity requirement is owned by
  `docs/specs/universal-agent-api/extensions-spec.md`
  (`agent_api.config.model.v1`, "Runtime rejection behavior (v1, normative)").

Implementation note (non-normative): the active verification plan for this contract clause lives in
`docs/project_management/packs/active/agent-api-model-selection/seam-4-claude-code-mapping.md`.

## `agent_api.session.resume.v1` mapping (pinned)

The resume extension key is owned by `docs/specs/universal-agent-api/extensions-spec.md`. This
section pins the Claude CLI mapping.

Let `PROMPT == AgentWrapperRunRequest.prompt` (non-empty after trimming; validated pre-spawn by the
Universal Agent API run protocol).

### selector `"last"`

The backend MUST spawn an argv containing the following **ordered subsequence**:

`--print --output-format stream-json [--model ID] [--permission-mode bypassPermissions] [--add-dir <DIR...>] --continue --verbose PROMPT`

### selector `"id"`

Let `ID == extensions["agent_api.session.resume.v1"].id` (non-empty after trimming).

The backend MUST spawn an argv containing the following **ordered subsequence**:

`--print --output-format stream-json [--model ID] [--permission-mode bypassPermissions] [--add-dir <DIR...>] --resume ID --verbose PROMPT`

## `agent_api.session.fork.v1` mapping (pinned)

The fork extension key is owned by `docs/specs/universal-agent-api/extensions-spec.md`. This
section pins the Claude CLI mapping.

Let `PROMPT == AgentWrapperRunRequest.prompt` (non-empty after trimming; validated pre-spawn).

### selector `"last"`

The backend MUST spawn an argv containing the following **ordered subsequence**:

`--print --output-format stream-json [--model ID] [--permission-mode bypassPermissions] [--add-dir <DIR...>] --continue --fork-session --verbose PROMPT`

### selector `"id"`

Let `ID == extensions["agent_api.session.fork.v1"].id` (non-empty after trimming).

The backend MUST spawn an argv containing the following **ordered subsequence**:

`--print --output-format stream-json [--model ID] [--permission-mode bypassPermissions] [--add-dir <DIR...>] --fork-session --resume ID --verbose PROMPT`

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
