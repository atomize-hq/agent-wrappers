# Contract — Universal Agent API (authoritative)

Status: Draft  
Date (UTC): 2026-02-16  
Feature directory: `docs/project_management/next/universal-agent-api/`

This document is the authoritative contract for the new `agent_api` crate’s public Rust API surface.

Normative language: this contract uses RFC 2119 requirement keywords (`MUST`, `MUST NOT`, `SHOULD`).

## Crate

- Crate: `agent_api` (new workspace member under `crates/agent_api`)
- The crate MUST compile with default features (no backends) enabled.
- The crate MUST NOT publicly re-export any `codex` or `claude_code` types in v1.

## Feature flags (crate features; normative)

- `codex`: enable Codex backend support (depends on `crates/codex`)
- `claude_code`: enable Claude Code backend support (depends on `crates/claude_code`)

Consumers must enable features using Cargo’s standard syntax, e.g.:
- `cargo test -p agent_api --features codex`
- `cargo test -p agent_api --features claude_code`
- `cargo test -p agent_api --all-features`

## Public API (v1, normative)

The `agent_api` crate MUST expose the following items at the crate root (i.e., these paths MUST
resolve for downstream consumers):

```rust
use agent_api::{
    AgentBackend, AgentCapabilities, AgentCompletion, AgentError, AgentEvent, AgentEventKind,
    AgentGateway, AgentKind, AgentRunHandle, AgentRunRequest, AgentRunResult,
};
```

### Core types (v1, normative)

```rust
use std::collections::{BTreeMap, BTreeSet};
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::process::ExitStatus;
use std::sync::Arc;
use std::time::Duration;

use futures_core::Stream;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct AgentKind(String);

impl AgentKind {
    /// Creates an agent kind from a string.
    ///
    /// The value MUST follow `capabilities-schema-spec.md` naming rules.
    pub fn new(value: impl Into<String>) -> Result<Self, AgentError>;

    /// Returns the canonical string id.
    pub fn as_str(&self) -> &str;
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AgentCapabilities {
    /// Set of namespaced capability ids (see `capabilities-schema-spec.md`).
    pub ids: BTreeSet<String>,
}

impl AgentCapabilities {
    pub fn contains(&self, capability_id: &str) -> bool;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AgentEventKind {
    TextOutput,
    ToolCall,
    ToolResult,
    Status,
    Error,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentEvent {
    pub agent_kind: AgentKind,
    pub kind: AgentEventKind,
    pub channel: Option<String>,
    /// Stable payload for `TextOutput` events.
    pub text: Option<String>,
    /// Stable payload for `Status` and `Error` events.
    pub message: Option<String>,
    pub data: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Default)]
pub struct AgentRunRequest {
    pub prompt: String,
    pub working_dir: Option<PathBuf>,
    pub timeout: Option<Duration>,
    pub env: BTreeMap<String, String>,
    /// Extension options are namespaced keys with JSON values.
    pub extensions: BTreeMap<String, serde_json::Value>,
}

pub type DynAgentEventStream = Pin<Box<dyn Stream<Item = AgentEvent> + Send>>;
pub type DynAgentCompletion =
    Pin<Box<dyn Future<Output = Result<AgentCompletion, AgentError>> + Send>>;

#[derive(Debug)]
pub struct AgentRunHandle {
    pub events: DynAgentEventStream,
    pub completion: DynAgentCompletion,
}

#[derive(Clone, Debug)]
pub struct AgentCompletion {
    pub status: ExitStatus,
    /// A backend may populate `final_text` when it can deterministically extract it.
    pub final_text: Option<String>,
    /// Optional backend-specific completion payload.
    ///
    /// This payload MUST obey the bounds and enforcement behavior defined in
    /// `event-envelope-schema-spec.md` (see "Completion payload bounds").
    pub data: Option<serde_json::Value>,
}

#[derive(Clone, Debug)]
pub struct AgentRunResult {
    pub completion: AgentCompletion,
}

#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("unknown backend: {agent_kind}")]
    UnknownBackend { agent_kind: String },
    #[error("unsupported capability for {agent_kind}: {capability}")]
    UnsupportedCapability { agent_kind: String, capability: String },
    #[error("invalid agent kind: {message}")]
    InvalidAgentKind { message: String },
    #[error("invalid request: {message}")]
    InvalidRequest { message: String },
    #[error("backend error: {message}")]
    Backend { message: String },
}

pub trait AgentBackend: Send + Sync {
    fn kind(&self) -> AgentKind;
    fn capabilities(&self) -> AgentCapabilities;

    /// Starts a run and returns a handle producing events and a completion result.
    ///
    /// Backends MUST enforce capability gating per `run-protocol-spec.md`.
    fn run(&self, request: AgentRunRequest) -> Pin<Box<dyn Future<Output = Result<AgentRunHandle, AgentError>> + Send + '_>>;
}

#[derive(Clone, Default)]
pub struct AgentGateway {
    // private
}

impl AgentGateway {
    pub fn new() -> Self;

    /// Registers a backend.
    ///
    /// If a backend with the same `AgentKind` is already registered, this MUST return an error.
    pub fn register(&mut self, backend: Arc<dyn AgentBackend>) -> Result<(), AgentError>;

    /// Resolves a backend by `AgentKind`.
    pub fn backend(&self, agent_kind: &AgentKind) -> Option<Arc<dyn AgentBackend>>;

    /// Convenience entrypoint: resolves a backend and starts a run.
    ///
    /// This MUST return `AgentError::UnknownBackend` when no backend is registered for `agent_kind`.
    pub fn run(&self, agent_kind: &AgentKind, request: AgentRunRequest) -> Pin<Box<dyn Future<Output = Result<AgentRunHandle, AgentError>> + Send + '_>>;
}
```

## Stable payload rules for core event kinds (v1, normative)

For each emitted `AgentEvent`:

- `AgentEventKind::TextOutput`
  - `text` MUST be `Some`.
  - `message` MUST be `None`.
- `AgentEventKind::Status`
  - `message` SHOULD be `Some`.
  - `text` MUST be `None`.
- `AgentEventKind::Error`
  - `message` MUST be `Some` and MUST be safe/redacted.
  - `text` MUST be `None`.
- `AgentEventKind::{ToolCall, ToolResult, Unknown}`
  - `text` MUST be `None`.
  - `message` MAY be `Some` (safe/redacted) for operator-facing summaries, but SHOULD be `None` by default.

`data`:
- MAY be present for any kind.
- MUST conform to size/safety constraints in `event-envelope-schema-spec.md`.

## Provided backends (feature-gated; v1, normative)

When enabled, `agent_api` MUST provide built-in backends with stable paths and constructor/config
types that use **only** std + serde-friendly types (no `codex::*` / `claude_code::*` in the public API).

### Backend module layout (normative)

The crate MUST expose:

```rust
pub mod backends {
    // Codex backend (`feature = "codex"`)
    #[cfg(feature = "codex")]
    pub mod codex {
        use std::{collections::BTreeMap, path::PathBuf, time::Duration};

        use super::super::{AgentBackend, AgentError, AgentKind, AgentRunHandle, AgentRunRequest};

        #[derive(Clone, Debug, Default)]
        pub struct CodexBackendConfig {
            pub binary: Option<PathBuf>,
            pub codex_home: Option<PathBuf>,
            pub default_timeout: Option<Duration>,
            pub default_working_dir: Option<PathBuf>,
            pub env: BTreeMap<String, String>,
        }

        pub struct CodexBackend { /* private */ }

        impl CodexBackend {
            pub fn new(config: CodexBackendConfig) -> Self;
        }

        impl AgentBackend for CodexBackend {
            fn kind(&self) -> AgentKind; // MUST be "codex"
            fn capabilities(&self) -> super::super::AgentCapabilities;
            fn run(&self, request: AgentRunRequest) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<AgentRunHandle, AgentError>> + Send + '_>>;
        }
    }

    // Claude Code backend (`feature = "claude_code"`)
    #[cfg(feature = "claude_code")]
    pub mod claude_code {
        use std::{collections::BTreeMap, path::PathBuf, time::Duration};

        use super::super::{AgentBackend, AgentError, AgentKind, AgentRunHandle, AgentRunRequest};

        #[derive(Clone, Debug, Default)]
        pub struct ClaudeCodeBackendConfig {
            pub binary: Option<PathBuf>,
            pub default_timeout: Option<Duration>,
            pub default_working_dir: Option<PathBuf>,
            pub env: BTreeMap<String, String>,
        }

        pub struct ClaudeCodeBackend { /* private */ }

        impl ClaudeCodeBackend {
            pub fn new(config: ClaudeCodeBackendConfig) -> Self;
        }

        impl AgentBackend for ClaudeCodeBackend {
            fn kind(&self) -> AgentKind; // MUST be "claude_code"
            fn capabilities(&self) -> super::super::AgentCapabilities;
            fn run(&self, request: AgentRunRequest) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<AgentRunHandle, AgentError>> + Send + '_>>;
        }
    }
}
```

### Config and request precedence (v1, normative)

- Backend config provides defaults.
- `AgentRunRequest` fields MUST override backend config defaults for that run.
- The backend MUST apply `AgentRunRequest.env` on top of backend config env (request keys win).

## Extensions and capability gating (v1, normative)

- Every supported `AgentRunRequest.extensions` key MUST correspond 1:1 to a capability id of the
  same string present in `AgentCapabilities.ids`.
- If a request includes an extension key that is not present in `AgentCapabilities.ids`, the backend
  MUST fail-closed with `AgentError::UnsupportedCapability { capability: <key> }`.
- If an extension key is supported but its value is invalid, the backend MUST return
  `AgentError::InvalidRequest`.
- Validation of extension keys and values MUST occur before spawning any backend process.

### Extension option key naming (v1, normative)

Keys in `AgentRunRequest.extensions` MUST:

- be lowercase ASCII
- match regex: `^[a-z][a-z0-9_.-]*$`
- be namespaced:
  - MUST contain at least one `.` character
  - MUST start with either:
    - `agent_api.` (reserved for universal options; none defined in v1), or
    - `backend.<agent_kind>.` (backend-specific options)

If a key starts with `backend.`, the backend MUST validate that the key begins with
`backend.<this backend's AgentKind>.` and MUST reject other backends’ namespaces as
`AgentError::UnsupportedCapability`.

## Error taxonomy (normative)

- `AgentError::UnknownBackend` MUST be emitted when a caller targets an `AgentKind` with no registered backend.
- `AgentError::UnsupportedCapability` MUST be emitted when a caller invokes an operation not supported by that backend’s capabilities.
- `AgentGateway::register` MUST emit `AgentError::InvalidRequest` when a backend is registered with an already-registered `AgentKind`.

All error messages MUST be safe-by-default and MUST NOT include raw backend output in v1.

## Absence semantics (normative)

- If `AgentRunRequest.timeout` is absent: backend-specific default applies (the universal API MUST NOT invent a global default).
- If `AgentRunRequest.working_dir` is absent: backend-specific default applies (wrappers may use temp dirs).
- The universal API MUST NOT mutate the parent process environment; `AgentRunRequest.env` applies only to spawned backend processes.
- If `AgentRunRequest.extensions` contains any key that the backend does not recognize, the backend MUST fail-closed with `AgentError::UnsupportedCapability` per the 1:1 mapping rule above.
