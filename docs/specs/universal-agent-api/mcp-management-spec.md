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

- A backend that implements an operation MUST advertise the corresponding capability id.
- A backend that does not implement an operation MUST NOT advertise the corresponding capability id.
- If a caller invokes an operation that is not advertised, the backend MUST fail-closed with:
  `AgentWrapperError::UnsupportedCapability { agent_kind, capability }`.

## API surface (v1, normative)

### Non-run command boundary

These operations are **non-run**: they MUST be exposed as dedicated API methods, not as run extensions.

The API SHOULD be expressed as:

- `AgentWrapperGateway` convenience methods (backend resolution + error shaping), and
- default methods on `AgentWrapperBackend` that return `UnsupportedCapability` (non-breaking additive trait evolution),
  mirroring the `run_control` pattern in `docs/specs/universal-agent-api/contract.md`.

### Public types (v1, normative)

The crate SHOULD expose these types under a dedicated module:

- Module: `agent_api::mcp`

The following paths SHOULD resolve for downstream consumers:

```rust
use agent_api::mcp::{
    AgentWrapperMcpAddRequest, AgentWrapperMcpAddTransport, AgentWrapperMcpCommandContext,
    AgentWrapperMcpCommandOutput, AgentWrapperMcpGetRequest, AgentWrapperMcpListRequest,
    AgentWrapperMcpRemoveRequest,
};
```

This module MUST use only std + serde-friendly types (no `codex::*` / `claude_code::*` in the public API).

#### Pinned type shapes (v1)

The API SHOULD use the following type shapes (field names are normative once approved):

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
        /// Additional argv items after `command[0]`.
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

`AgentWrapperGateway` SHOULD expose convenience entrypoints that:

- resolve a backend (else `UnknownBackend`), and
- invoke the corresponding backend operation (else `UnsupportedCapability`).

Suggested signatures:

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

`AgentWrapperBackend` SHOULD expose default methods for these operations (non-breaking additive evolution),
with defaults that return `UnsupportedCapability`:

- `mcp_list`
- `mcp_get`
- `mcp_add`
- `mcp_remove`

#### Server name validation (pinned)

For all operations that accept a server name:

- The provided `name` MUST be trimmed.
- Empty names MUST be rejected as `AgentWrapperError::InvalidRequest`.

#### Process context

The universal request types MUST support:

- `working_dir`: process working directory override (optional)
- `timeout`: per-command timeout override (optional)
- `env`: per-command environment overrides applied only to the spawned backend process (request keys win)

This preserves Universal Agent API posture:

- Backends MUST NOT mutate the parent process environment.

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

## Safety posture (v1, normative)

MCP management commands interact with persistent tool configuration and may read/write sensitive
values (headers, env vars, tokens).

Requirements:

- MCP management APIs MUST NOT emit their stdout/stderr as `AgentWrapperEvent`s.
- Backends SHOULD support isolated homes (e.g., `codex_home`, `claude_home`) so automation can run
  against a dedicated state root.
- Write operations (`add/remove`) MUST require explicit enablement.
  - Built-in backends MUST NOT advertise `agent_api.tools.mcp.add.v1` / `agent_api.tools.mcp.remove.v1`
    unless write enablement is configured.
  - Write enablement MUST be explicit and discoverable (via backend config and/or advertised capabilities).

## Backend-specific MCP operations (v1, informative)

Some MCP subcommands are backend-specific (not universalized in v1), including:

- Codex: `codex mcp login/logout` (OAuth)
- Claude Code: `claude mcp add-json`, `add-from-claude-desktop`, `serve`, `reset-project-choices`

If surfaced through `agent_api` in the future, they MUST:

- remain backend-scoped (`backend.<agent_kind>.*`) until cross-backend semantics are proven, and
- stay bounded/typed (no generic argv pass-through).
