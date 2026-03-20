# MCP Management Spec — Universal Agent API

Status: Draft  
Date (UTC): 2026-03-02  
Source ADR: `docs/adr/0018-universal-mcp-management-commands.md`

This spec defines a **minimal, universal, non-run** API for managing MCP (Model Context Protocol)
server configurations across built-in `agent_api` backends.

This document is normative once approved and uses RFC 2119 keywords (`MUST`, `MUST NOT`, `SHOULD`).

## Motivation

MCP server management is exposed by upstream CLIs as subcommands (e.g., `codex mcp ...`, `claude mcp ...`).
These are **not** run-time options and MUST NOT be modeled as `AgentWrapperRunRequest.extensions`.

Universal Agent API consumers (orchestrators) still need a backend-neutral way to:

- list configured MCP servers,
- get a specific server configuration,
- add a server configuration, and
- remove a server configuration,

without depending directly on backend-specific wrapper crates.

## Capability ids (v1, normative)

Each operation is gated by a distinct capability id:

- `agent_api.tools.mcp.list.v1`
- `agent_api.tools.mcp.get.v1`
- `agent_api.tools.mcp.add.v1`
- `agent_api.tools.mcp.remove.v1`

Backends:

- Capability advertising is **per backend instance** and MAY vary based on environment and configuration.
- For purposes of this spec, a backend **implements** an operation iff it is both:
  - supported in the current environment (e.g., the upstream CLI exposes the required subcommand on this target), and
  - enabled to expose (e.g., write enablement for `add/remove`).
- A backend that implements an operation MUST advertise the corresponding capability id.
- A backend that does not implement an operation (including because it is disabled by configuration) MUST NOT advertise the
  corresponding capability id.
- If a caller invokes an operation that is not advertised, the backend MUST fail-closed with:
  `AgentWrapperError::UnsupportedCapability { agent_kind, capability }`.

## API surface (v1, normative)

### Non-run command boundary

These operations are **non-run**: they MUST be exposed as dedicated API methods, not as run extensions.

The API MUST be expressed as:

- `AgentWrapperGateway` convenience methods (backend resolution + error shaping), and
- default methods on `AgentWrapperBackend` that return `UnsupportedCapability` (non-breaking additive trait evolution),
  mirroring the `run_control` pattern in `docs/specs/universal-agent-api/contract.md`.

### Public types (v1, normative)

The crate MUST expose these types under a dedicated module:

- Module: `agent_api::mcp`

The following paths MUST resolve for downstream consumers:

```rust
use agent_api::mcp::{
    AgentWrapperMcpAddRequest, AgentWrapperMcpAddTransport, AgentWrapperMcpCommandContext,
    AgentWrapperMcpCommandOutput, AgentWrapperMcpGetRequest, AgentWrapperMcpListRequest,
    AgentWrapperMcpRemoveRequest,
};
```

This module MUST use only std + [serde-friendly types](contract.md#serde-friendly-types) (no `codex::*` /
`claude_code::*` in the public API).

#### Pinned type shapes (v1)

The API MUST use the following type shapes (field names are normative once approved):

```rust
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::ExitStatus;
use std::time::Duration;

#[derive(Clone, Debug, Default)]
pub struct AgentWrapperMcpCommandContext {
    pub working_dir: Option<PathBuf>,
    pub timeout: Option<Duration>,
    pub env: BTreeMap<String, String>,
}

#[derive(Clone, Debug)]
pub struct AgentWrapperMcpCommandOutput {
    pub status: ExitStatus,
    pub stdout: String,
    pub stderr: String,
    pub stdout_truncated: bool,
    pub stderr_truncated: bool,
}

#[derive(Clone, Debug, Default)]
pub struct AgentWrapperMcpListRequest {
    pub context: AgentWrapperMcpCommandContext,
}

#[derive(Clone, Debug)]
pub struct AgentWrapperMcpGetRequest {
    pub name: String,
    pub context: AgentWrapperMcpCommandContext,
}

#[derive(Clone, Debug)]
pub struct AgentWrapperMcpRemoveRequest {
    pub name: String,
    pub context: AgentWrapperMcpCommandContext,
}

#[derive(Clone, Debug)]
pub enum AgentWrapperMcpAddTransport {
    /// Launches an MCP server via stdio.
    Stdio {
        /// Command argv (MUST be non-empty).
        command: Vec<String>,
        /// Additional argv items appended after `command`.
        args: Vec<String>,
        /// Env vars injected into the MCP server process.
        env: BTreeMap<String, String>,
    },
    /// Connects to a streamable HTTP MCP server.
    Url {
        url: String,
        bearer_token_env_var: Option<String>,
    },
}

#[derive(Clone, Debug)]
pub struct AgentWrapperMcpAddRequest {
    pub name: String,
    pub transport: AgentWrapperMcpAddTransport,
    pub context: AgentWrapperMcpCommandContext,
}
```

### Request/response typing (v1, normative)

The universal surface MUST be bounded and typed:

- The API MUST NOT accept arbitrary argv arrays.
- The API MUST NOT provide a generic “extra args” escape hatch.

### Gateway entrypoints (v1, normative)

`AgentWrapperGateway` MUST expose convenience entrypoints that:

- resolve a backend (else `UnknownBackend`), and
- invoke the corresponding backend operation (else `UnsupportedCapability`).

To make error ordering deterministic, gateway entrypoints MUST:

1) resolve the backend (else `AgentWrapperError::UnknownBackend`), then
2) check the backend-advertised capability id for the operation (else `AgentWrapperError::UnsupportedCapability`), then
3) invoke the backend hook.

Pinned signatures:

```rust
use std::future::Future;
use std::pin::Pin;

use agent_api::{AgentWrapperError, AgentWrapperGateway, AgentWrapperKind};
use agent_api::mcp::{
    AgentWrapperMcpAddRequest, AgentWrapperMcpCommandOutput, AgentWrapperMcpGetRequest,
    AgentWrapperMcpListRequest, AgentWrapperMcpRemoveRequest,
};

impl AgentWrapperGateway {
    pub fn mcp_list(
        &self,
        agent_kind: &AgentWrapperKind,
        request: AgentWrapperMcpListRequest,
    ) -> Pin<Box<dyn Future<Output = Result<AgentWrapperMcpCommandOutput, AgentWrapperError>> + Send + '_>>;

    pub fn mcp_get(
        &self,
        agent_kind: &AgentWrapperKind,
        request: AgentWrapperMcpGetRequest,
    ) -> Pin<Box<dyn Future<Output = Result<AgentWrapperMcpCommandOutput, AgentWrapperError>> + Send + '_>>;

    pub fn mcp_add(
        &self,
        agent_kind: &AgentWrapperKind,
        request: AgentWrapperMcpAddRequest,
    ) -> Pin<Box<dyn Future<Output = Result<AgentWrapperMcpCommandOutput, AgentWrapperError>> + Send + '_>>;

    pub fn mcp_remove(
        &self,
        agent_kind: &AgentWrapperKind,
        request: AgentWrapperMcpRemoveRequest,
    ) -> Pin<Box<dyn Future<Output = Result<AgentWrapperMcpCommandOutput, AgentWrapperError>> + Send + '_>>;
}
```

### Backend hooks (v1, normative)

`AgentWrapperBackend` MUST expose default methods for these operations (non-breaking additive evolution),
with defaults that return `UnsupportedCapability`:

- `mcp_list`
- `mcp_get`
- `mcp_add`
- `mcp_remove`

#### Server name validation (pinned)

For all operations that accept a server name:

- The provided `name` MUST be trimmed.
- Empty names MUST be rejected as `AgentWrapperError::InvalidRequest`.

#### Transport field validation (pinned)

All transport field validation MUST occur before spawning any backend process. Violations MUST be rejected as
`AgentWrapperError::InvalidRequest` with a safe message that does not echo raw user-provided values.

##### `Stdio` transport (`AgentWrapperMcpAddTransport::Stdio`)

- `command` MUST be non-empty.
- Every item in `command` and `args` MUST be trimmed and MUST be non-empty.
- Final argv used for backend mapping MUST be constructed as:
  - `argv = command + args` (concatenation)

##### `Url` transport (`AgentWrapperMcpAddTransport::Url`)

- `url` MUST be trimmed and MUST be non-empty.
- `url` MUST be an absolute URL with scheme `http` or `https` (parsing is required).
- If `bearer_token_env_var` is `Some(s)`:
  - `s` MUST be trimmed and MUST be non-empty.
  - `s` MUST match regex: `^[A-Za-z_][A-Za-z0-9_]*$`

#### Process context

The universal request types MUST support:

- `working_dir`: process working directory override (optional)
- `timeout`: per-command timeout override (optional)
- `env`: per-command environment overrides applied only to the spawned backend process (request keys win)

This preserves Universal Agent API posture:

- Backends MUST NOT mutate the parent process environment.

#### Context precedence and absence semantics (pinned)

Backends MUST determine an effective process context before spawning any backend process. The precedence rules MUST mirror
`docs/specs/universal-agent-api/contract.md` ("Config and request precedence" + "Absence semantics"):

- Effective working directory:
  1) If `request.context.working_dir` is present, it MUST be used.
  2) Else, if backend config provides `default_working_dir`, it MUST be used.
  3) Else, the backend MAY use an internal default (including inheriting the parent process working directory or using a
     temporary directory).
  - If the selected working directory path is relative, it MUST be interpreted as relative to the backend wrapper process’
    current working directory at invocation time.

- Effective timeout:
  1) If `request.context.timeout` is present, it MUST be used.
  2) Else, if backend config provides `default_timeout`, it MUST be used.
  3) Else, the backend MAY use an internal default (or no timeout).
  - If `timeout` is absent, the universal API MUST NOT invent a global default.
  - For MCP management command execution, a present timeout value MUST be enforced verbatim, including
    `Duration::ZERO` as an immediate fail-fast timeout budget.

- Effective environment:
  - `request.context.env` applies only to the spawned backend process (never to the parent process).
  - When keys collide, `request.context.env` MUST win over any backend-provided environment, including isolated-home
    injection (e.g., `CODEX_HOME`, `CLAUDE_HOME`, `HOME`, `XDG_*`, and Windows equivalents).
  - If a caller explicitly overrides isolation-related keys via `request.context.env`, that override is honored (caller
    assumes responsibility for defeating isolation).

Note: `AgentWrapperMcpAddTransport::Stdio.env` configures environment variables for the *MCP server process* (as persisted
by the upstream CLI). It is distinct from `request.context.env`, which configures the environment for the *upstream CLI
process* invoked to perform the management command.

#### Command execution semantics (pinned)

Gateway/backend MCP methods return:
- `Result<AgentWrapperMcpCommandOutput, AgentWrapperError>`

Pinned rules (v1):

- If the upstream CLI process is spawned and an `ExitStatus` is observed, the operation MUST return
  `Ok(AgentWrapperMcpCommandOutput { status, ... })` **regardless of whether the exit status is success**.
- The following failure modes MUST be surfaced as `Err(AgentWrapperError::Backend { message })`:
  - binary not found / spawn failure,
  - wait/IO errors while running the command,
  - timeout expiration (best-effort kill/cleanup),
  - failures to capture stdout/stderr streams (including join/task failures in async implementations).
- For any `Err(AgentWrapperError::Backend { .. })`, implementations MUST NOT surface partial stdout/stderr in the error
  message and MUST NOT attempt to return `AgentWrapperMcpCommandOutput` (output is available only in the `Ok(...)` case).

#### Output bounds

The command output MUST be bounded:

- stdout and stderr MUST be captured with fixed byte budgets.
- If an output stream exceeds its budget, it MUST be truncated deterministically and marked as truncated.

Pinned budgets (v1):

- `stdout`: 65,536 bytes
- `stderr`: 65,536 bytes

Pinned truncation marker (v1):

- Suffix: `…(truncated)`

If truncation occurs, the output MUST remain valid UTF-8.

These budgets intentionally match the Universal Agent API text bounds used for run events (see
`docs/specs/universal-agent-api/event-envelope-schema-spec.md` and `docs/specs/universal-agent-api/contract.md`).

##### Output capture + truncation algorithm (pinned)

Budgets are measured in **UTF-8 bytes** of the returned `stdout` / `stderr` strings.

Implementations MUST capture stdout/stderr in a bounded fashion and MUST NOT buffer unbounded output in memory.

For each stream (`stdout`, `stderr`), the backend MUST enforce the following deterministic algorithm:

1) Capture bytes from the subprocess stream in a bounded/streaming manner:
   - The backend MUST retain at most `bound_bytes + 1` bytes (or equivalently: retain `bound_bytes` bytes plus a
     "saw more bytes" flag).
   - Any additional bytes beyond the retained bound MUST be discarded (never buffer unbounded output in memory).
2) Decode captured bytes as UTF-8:
   - If decoding fails, the backend MUST use lossy decoding (replace invalid byte sequences with U+FFFD) so the final
     output is valid UTF-8.
3) Enforce the byte bound using the same truncation behavior as `event-envelope-schema-spec.md` ("Enforcement behavior"):
   - Let `bound_bytes` be the pinned budget (65,536).
   - Let `suffix = "…(truncated)"`.
   - If the stream output exceeds `bound_bytes` (either because the backend observed additional bytes past the bound, or
     because the decoded output exceeds the bound), set `<stream>_truncated = true` and:
     - if `bound_bytes > len(suffix_bytes)`: truncate to `bound_bytes - len(suffix_bytes)` bytes (UTF-8 safe) and append
       `suffix`;
     - else: set the output to `"…"` truncated to `bound_bytes` bytes.
   - If the stream output does not exceed the bound, set `<stream>_truncated = false` and return the decoded output as-is.

The intent of this algorithm is:
- deterministic output for tests, and
- bounded memory posture (never buffer beyond the fixed budgets).

## Safety posture (v1, normative)

MCP management commands interact with persistent tool configuration and may read/write sensitive
values (headers, env vars, tokens).

Requirements:

- MCP management APIs MUST NOT emit their stdout/stderr as `AgentWrapperEvent`s.
- Backends SHOULD support isolated homes so automation can run against a dedicated state root.
- For built-in backends, any host-facing isolated-home config field MUST be defined in:
  - `docs/specs/universal-agent-api/contract.md`
  - Canonical approved field(s) in v1:
    - `agent_api::backends::codex::CodexBackendConfig.codex_home: Option<PathBuf>`
    - `agent_api::backends::claude_code::ClaudeCodeBackendConfig.claude_home: Option<PathBuf>`
  - When a built-in backend exposes such a home override and it is `Some`, the backend MUST invoke
    the upstream CLI such that its persistent state/config is read/written beneath the configured
    root (while still honoring request-level env overrides per “Context precedence and absence
    semantics (pinned)”).
  - Claude-specific caveat (pinned): `claude_home` is wrapper-managed user-home isolation only; it
    does not isolate project-local `.claude/` content or `.mcp.json`.
- Write operations (`add/remove`) MUST require explicit enablement.
  - Built-in backends MUST expose explicit host-facing write enablement via public config:
    - `agent_api::backends::codex::CodexBackendConfig.allow_mcp_write: bool`
    - `agent_api::backends::claude_code::ClaudeCodeBackendConfig.allow_mcp_write: bool`
  - Both built-in fields MUST default to `false` (safe by default).
  - Built-in backends MUST NOT advertise `agent_api.tools.mcp.add.v1` /
    `agent_api.tools.mcp.remove.v1` unless write enablement is configured.
  - For built-in backends, `allow_mcp_write == true` MUST enable advertising only when the pinned
    CLI manifest snapshot shows the required subcommand is available on the current target.
  - The advertising predicate is fully determined by `(allow_mcp_write == true)` AND
    `(manifest indicates subcommand available on this target)`; no additional env/config gates
    exist in v1.
  - If the manifest indicates availability but the runtime binary lacks the subcommand, the
    backend MUST still advertise based on the manifest and MUST fail the operation with
    `AgentWrapperError::Backend` per “Target availability source of truth (pinned)”.
  - Read operations (`list/get`) are unchanged and have no extra write-enablement knob in v1.
  - This enablement applies only to non-run MCP management config mutation. It does not change what
    a configured MCP server may do during a normal run.

## Built-in backend behavior (v1, normative)

This section pins behavior for the built-in backends shipped by `agent_api` (Codex + Claude Code).

### Target availability source of truth (pinned)

For built-in backends, upstream MCP subcommand availability MUST be determined from the pinned CLI manifest snapshots:

- Codex: `cli_manifests/codex/current.json`
- Claude Code: `cli_manifests/claude_code/current.json`

The manifest snapshots are the authoritative source of truth for v1 capability advertising and MUST be treated as
normative for built-in backends.

If the manifest snapshot conflicts with the observed upstream CLI behavior at runtime:
- the backend MUST NOT silently change its advertised capabilities, and
- the operation MUST fail with `AgentWrapperError::Backend` (backend fault).

The required remediation is to update the pinned manifest snapshot and mapping logic in a subsequent repo update.

### Default capability advertising posture (built-in backends, pinned)

Legend:
- ✅ = advertised by default (when the upstream CLI subcommand is available on this target)
- ❌ = not advertised by default

| Backend | Target availability (pinned) | `list` | `get` | `add` | `remove` |
| --- | --- | --- | --- | --- | --- |
| Codex (`codex`) | `cli_manifests/codex/current.json` | ✅ | ✅ | ❌ (requires `allow_mcp_write=true`) | ❌ (requires `allow_mcp_write=true`) |
| Claude Code (`claude_code`) | `cli_manifests/claude_code/current.json` | ✅ | ✅ on `win32-x64` only | ❌ (requires `win32-x64` and `allow_mcp_write=true`) | ❌ (requires `win32-x64` and `allow_mcp_write=true`) |

Notes:
- Read operations (`list/get`) have no additional enablement knob in v1. If the upstream CLI exposes the subcommand on
  this target, the backend advertises the capability by default.
- Write operations (`add/remove`) remain safe-by-default. Built-in backends expose
  `allow_mcp_write`, but it defaults to `false`, so these capabilities are not part of the default
  built-in backend surface.
- The generated capability matrix is derived from default built-in backend configs. Because
  `allow_mcp_write` defaults to `false`, `docs/specs/universal-agent-api/capability-matrix.md` may
  omit `agent_api.tools.mcp.add.v1` and `agent_api.tools.mcp.remove.v1`; runtime truth remains the
  selected backend instance's `AgentWrapperCapabilities.ids`.
- The committed capability matrix is generated against the repository's canonical built-in Linux
  target profile (`codex=x86_64-unknown-linux-musl`, `claude_code=linux-x64`) so the artifact is
  deterministic across developer hosts and CI runners.

### Built-in backend mappings (pinned)

This section pins argv construction for built-in backends. It does not imply cross-backend stdout/stderr parity.

#### Codex backend mapping (`agent_kind == "codex"`)

- `mcp_list` MUST invoke: `codex mcp list --json`
- `mcp_get` MUST invoke: `codex mcp get --json <name>`
- `mcp_remove` MUST invoke: `codex mcp remove <name>`
- `mcp_add`:
  - `Stdio` MUST invoke:
    - `codex mcp add <name> [--env KEY=VALUE]* -- <argv...>`
    - where `<argv...>` is the concatenated `command + args` from the request.
  - `Url` MUST invoke:
    - `codex mcp add <name> --url <url> [--bearer-token-env-var <ENV_VAR>]`

#### Claude Code backend mapping (`agent_kind == "claude_code"`)

- `mcp_list` MUST invoke: `claude mcp list`
- `mcp_get` MUST invoke: `claude mcp get <name>` (only when available on this target per the pinned manifest)
- `mcp_remove` MUST invoke: `claude mcp remove <name>` (only when available on this target per the pinned manifest)
- `mcp_add` (only when available on this target per the pinned manifest):
  - `Stdio` MUST invoke:
    - `claude mcp add --transport stdio [--env KEY=VALUE]* <name> <commandOrUrl> [args...]`
    - where `<commandOrUrl>` is `argv[0]` and `[args...]` is the remaining items from `argv[1..]`.
  - `Url` MUST invoke:
    - if `bearer_token_env_var == None`:
      - `claude mcp add --transport http <name> <url>`
    - if `bearer_token_env_var == Some(_)`:
      - reject as `AgentWrapperError::InvalidRequest` (no deterministic/safe mapping to `--header` in v1; fail closed).

## Verification policy (this repo; v1, pinned)

This section pins how this repository verifies conformance to this spec for MCP management.

### Unit coverage (pinned)

Unit tests MUST pin:
- capability gating (`UnsupportedCapability` for unadvertised operations),
- request validation (trimmed/non-empty server names),
- transport validation + argv composition (see “Transport field validation (pinned)”),
- process context precedence + absence semantics (see “Context precedence and absence semantics (pinned)”),
- command execution semantics (`Ok(output)` for non-zero exit status; execution faults are `Err(Backend)`),
- output bounds + truncation algorithm (see “Output capture + truncation algorithm (pinned)”),
- built-in backend advertising + mapping behavior (see “Built-in backend behavior (v1, normative)”).

### Integration coverage + gating (pinned)

To keep CI deterministic and offline:

- Default integration coverage (CI + local) MUST use **hermetic fake binaries** for MCP operations.
  - Tests MUST generate a fake `codex` / `claude` executable that:
    - records received argv + relevant environment variables, and
    - performs any “state mutation” by writing sentinel files beneath the injected isolated home directory.
  - These tests MUST run under the normal `cargo test` / `make test` flow (no opt-in).
  - “No network required” is enforced by construction (no real upstream binaries are executed).

- Optional live smoke tests MAY target real installed upstream binaries, but MUST be opt-in and MUST NOT run in CI by
  default.
  - Gating mechanism (pinned):
    - mark live tests as `#[ignore]`, and
    - require an explicit environment opt-in: `AGENT_API_MCP_LIVE=1`, plus a configured binary path for the targeted
      backend (backend config `binary`, or environment selection that the wrapper honors such as `CODEX_BINARY` / `CLAUDE_BINARY`).
  - Live smoke tests MUST still:
    - run against isolated homes, and
    - avoid requiring network access to pass (use only `list/get/add/remove`; no networked MCP servers).

## `Url.bearer_token_env_var` semantics (v1, normative)

`AgentWrapperMcpAddTransport::Url.bearer_token_env_var` is an optional *environment variable name*, not a secret value.

Pinned rules (v1):

- A backend MUST NOT expand an env var value into argv, headers, or any other persisted config.
- If `bearer_token_env_var` is `Some`, a backend MUST either:
  - map it to an upstream CLI mechanism that accepts an env var **name** as configuration (preferred), or
  - reject the request as `AgentWrapperError::InvalidRequest` (fail closed).

## Backend-specific MCP operations (v1, informative)

Some MCP subcommands are backend-specific (not universalized in v1), including:

- Codex: `codex mcp login/logout` (OAuth)
- Claude Code: `claude mcp add-json`, `add-from-claude-desktop`, `serve`, `reset-project-choices`

If surfaced through `agent_api` in the future, they MUST:

- remain backend-scoped (`backend.<agent_kind>.*`) until cross-backend semantics are proven, and
- stay bounded/typed (no generic argv pass-through).
