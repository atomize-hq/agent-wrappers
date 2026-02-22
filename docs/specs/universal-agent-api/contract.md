# Contract — Universal Agent API (authoritative)

Status: Approved  
Approved (UTC): 2026-02-21  
Date (UTC): 2026-02-16  
Canonical location: `docs/specs/universal-agent-api/`

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
    AgentWrapperBackend, AgentWrapperCapabilities, AgentWrapperCompletion, AgentWrapperError,
    AgentWrapperEvent, AgentWrapperEventKind, AgentWrapperGateway, AgentWrapperKind,
    AgentWrapperRunHandle, AgentWrapperRunRequest, AgentWrapperRunResult,
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
pub struct AgentWrapperKind(String);

impl AgentWrapperKind {
    /// Creates an agent kind from a string.
    ///
    /// The value MUST follow `capabilities-schema-spec.md` naming rules.
    pub fn new(value: impl Into<String>) -> Result<Self, AgentWrapperError>;

    /// Returns the canonical string id.
    pub fn as_str(&self) -> &str;
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AgentWrapperCapabilities {
    /// Set of namespaced capability ids (see `capabilities-schema-spec.md`).
    pub ids: BTreeSet<String>,
}

impl AgentWrapperCapabilities {
    pub fn contains(&self, capability_id: &str) -> bool;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AgentWrapperEventKind {
    TextOutput,
    ToolCall,
    ToolResult,
    Status,
    Error,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentWrapperEvent {
    pub agent_kind: AgentWrapperKind,
    pub kind: AgentWrapperEventKind,
    pub channel: Option<String>,
    /// Stable payload for `TextOutput` events.
    pub text: Option<String>,
    /// Stable payload for `Status` and `Error` events.
    pub message: Option<String>,
    pub data: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Default)]
pub struct AgentWrapperRunRequest {
    pub prompt: String,
    pub working_dir: Option<PathBuf>,
    pub timeout: Option<Duration>,
    pub env: BTreeMap<String, String>,
    /// Extension options are namespaced keys with JSON values.
    pub extensions: BTreeMap<String, serde_json::Value>,
}

pub type DynAgentWrapperEventStream = Pin<Box<dyn Stream<Item = AgentWrapperEvent> + Send>>;
pub type DynAgentWrapperCompletion =
    Pin<Box<dyn Future<Output = Result<AgentWrapperCompletion, AgentWrapperError>> + Send>>;

#[derive(Debug)]
pub struct AgentWrapperRunHandle {
    pub events: DynAgentWrapperEventStream,
    pub completion: DynAgentWrapperCompletion,
}

#[derive(Clone, Debug)]
pub struct AgentWrapperCompletion {
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
pub struct AgentWrapperRunResult {
    pub completion: AgentWrapperCompletion,
}

#[derive(Debug, thiserror::Error)]
pub enum AgentWrapperError {
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

pub trait AgentWrapperBackend: Send + Sync {
    fn kind(&self) -> AgentWrapperKind;
    fn capabilities(&self) -> AgentWrapperCapabilities;

    /// Starts a run and returns a handle producing events and a completion result.
    ///
    /// Backends MUST enforce capability gating per `run-protocol-spec.md`.
    fn run(&self, request: AgentWrapperRunRequest) -> Pin<Box<dyn Future<Output = Result<AgentWrapperRunHandle, AgentWrapperError>> + Send + '_>>;
}

#[derive(Clone, Default)]
pub struct AgentWrapperGateway {
    // private
}

impl AgentWrapperGateway {
    pub fn new() -> Self;

    /// Registers a backend.
    ///
    /// If a backend with the same `AgentWrapperKind` is already registered, this MUST return an error.
    pub fn register(&mut self, backend: Arc<dyn AgentWrapperBackend>) -> Result<(), AgentWrapperError>;

    /// Resolves a backend by `AgentWrapperKind`.
    pub fn backend(&self, agent_kind: &AgentWrapperKind) -> Option<Arc<dyn AgentWrapperBackend>>;

    /// Convenience entrypoint: resolves a backend and starts a run.
    ///
    /// This MUST return `AgentWrapperError::UnknownBackend` when no backend is registered for `agent_kind`.
    pub fn run(&self, agent_kind: &AgentWrapperKind, request: AgentWrapperRunRequest) -> Pin<Box<dyn Future<Output = Result<AgentWrapperRunHandle, AgentWrapperError>> + Send + '_>>;
}
```

## Stable payload rules for core event kinds (v1, normative)

For each emitted `AgentWrapperEvent`:

- `AgentWrapperEventKind::TextOutput`
  - `text` MUST be `Some`.
  - `message` MUST be `None`.
- `AgentWrapperEventKind::Status`
  - `message` SHOULD be `Some`.
  - `text` MUST be `None`.
- `AgentWrapperEventKind::Error`
  - `message` MUST be `Some` and MUST be safe/redacted.
  - `text` MUST be `None`.
- `AgentWrapperEventKind::{ToolCall, ToolResult, Unknown}`
  - `text` MUST be `None`.
  - `message` MAY be `Some` (safe/redacted) for operator-facing summaries, but SHOULD be `None` by default.

`data`:
- MAY be present for any kind.
- MUST conform to size/safety constraints in `event-envelope-schema-spec.md`.

## Provided backends (feature-gated; v1, normative)

When enabled, `agent_api` MUST provide built-in backends with stable paths and constructor/config
types that use **only** std + serde-friendly types (no `codex::*` / `claude_code::*` in the public API).

## Extensions (authoritative; v1)

`AgentWrapperRunRequest.extensions` is an open map of namespaced keys to JSON values.

Canonical rules for:
- core extension keys under `agent_api.*` (schema, defaults, absence semantics), and
- ownership rules for backend keys under `backend.<agent_kind>.*`

are defined in:
- `docs/specs/universal-agent-api/extensions-spec.md`

### Backend module layout (normative)

The crate MUST expose:

```rust
pub mod backends {
    // Codex backend (`feature = "codex"`)
    #[cfg(feature = "codex")]
    pub mod codex {
        use std::{collections::BTreeMap, path::PathBuf, time::Duration};

        use super::super::{
            AgentWrapperBackend, AgentWrapperError, AgentWrapperKind, AgentWrapperRunHandle,
            AgentWrapperRunRequest,
        };

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

        impl AgentWrapperBackend for CodexBackend {
            fn kind(&self) -> AgentWrapperKind; // MUST be "codex"
            fn capabilities(&self) -> super::super::AgentWrapperCapabilities;
            fn run(&self, request: AgentWrapperRunRequest) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<AgentWrapperRunHandle, AgentWrapperError>> + Send + '_>>;
        }
    }

    // Claude Code backend (`feature = "claude_code"`)
    #[cfg(feature = "claude_code")]
    pub mod claude_code {
        use std::{collections::BTreeMap, path::PathBuf, time::Duration};

        use super::super::{
            AgentWrapperBackend, AgentWrapperError, AgentWrapperKind, AgentWrapperRunHandle,
            AgentWrapperRunRequest,
        };

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

        impl AgentWrapperBackend for ClaudeCodeBackend {
            fn kind(&self) -> AgentWrapperKind; // MUST be "claude_code"
            fn capabilities(&self) -> super::super::AgentWrapperCapabilities;
            fn run(&self, request: AgentWrapperRunRequest) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<AgentWrapperRunHandle, AgentWrapperError>> + Send + '_>>;
        }
    }
}
```

### Config and request precedence (v1, normative)

- Backend config provides defaults.
- `AgentWrapperRunRequest` fields MUST override backend config defaults for that run.
- The backend MUST apply `AgentWrapperRunRequest.env` on top of backend config env (request keys win).

## Extensions and capability gating (v1, normative)

- Every supported `AgentWrapperRunRequest.extensions` key MUST correspond 1:1 to a capability id of the
  same string present in `AgentWrapperCapabilities.ids`.
- If a request includes an extension key that is not present in `AgentWrapperCapabilities.ids`, the backend
  MUST fail-closed with `AgentWrapperError::UnsupportedCapability { capability: <key> }`.
- If an extension key is supported but its value is invalid, the backend MUST return
  `AgentWrapperError::InvalidRequest`.
- Validation of extension keys and values MUST occur before spawning any backend process.

### Extension option key naming (v1, normative)

Keys in `AgentWrapperRunRequest.extensions` MUST:

- be lowercase ASCII
- match regex: `^[a-z][a-z0-9_.-]*$`
- be namespaced:
  - MUST contain at least one `.` character
  - MUST start with either:
    - `agent_api.` (reserved for universal options; none defined in v1), or
    - `backend.<agent_kind>.` (backend-specific options)

If a key starts with `backend.`, the backend MUST validate that the key begins with
`backend.<this backend's AgentWrapperKind>.` and MUST reject other backends’ namespaces as
`AgentWrapperError::UnsupportedCapability`.

## Error taxonomy (normative)

- `AgentWrapperError::UnknownBackend` MUST be emitted when a caller targets an `AgentWrapperKind` with no registered backend.
- `AgentWrapperError::UnsupportedCapability` MUST be emitted when a caller invokes an operation not supported by that backend’s capabilities.
- `AgentWrapperGateway::register` MUST emit `AgentWrapperError::InvalidRequest` when a backend is registered with an already-registered `AgentWrapperKind`.

All error messages MUST be safe-by-default and MUST NOT include raw backend output in v1.

## Absence semantics (normative)

- If `AgentWrapperRunRequest.timeout` is absent: backend-specific default applies (the universal API MUST NOT invent a global default).
- If `AgentWrapperRunRequest.working_dir` is absent: backend-specific default applies (wrappers may use temp dirs).
- The universal API MUST NOT mutate the parent process environment; `AgentWrapperRunRequest.env` applies only to spawned backend processes.
- If `AgentWrapperRunRequest.extensions` contains any key that the backend does not recognize, the backend MUST fail-closed with `AgentWrapperError::UnsupportedCapability` per the 1:1 mapping rule above.
