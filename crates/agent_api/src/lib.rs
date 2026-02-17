#![forbid(unsafe_code)]

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
    pub fn new(value: impl Into<String>) -> Result<Self, AgentWrapperError> {
        let value = value.into();
        validate_agent_kind(&value)?;
        Ok(Self(value))
    }

    /// Returns the canonical string id.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AgentWrapperCapabilities {
    /// Set of namespaced capability ids (see `capabilities-schema-spec.md`).
    pub ids: BTreeSet<String>,
}

impl AgentWrapperCapabilities {
    pub fn contains(&self, capability_id: &str) -> bool {
        self.ids.contains(capability_id)
    }
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

pub struct AgentWrapperRunHandle {
    pub events: DynAgentWrapperEventStream,
    pub completion: DynAgentWrapperCompletion,
}

impl std::fmt::Debug for AgentWrapperRunHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentWrapperRunHandle")
            .field("events", &"<stream>")
            .field("completion", &"<future>")
            .finish()
    }
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
    UnsupportedCapability {
        agent_kind: String,
        capability: String,
    },
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
    fn run(
        &self,
        request: AgentWrapperRunRequest,
    ) -> Pin<Box<dyn Future<Output = Result<AgentWrapperRunHandle, AgentWrapperError>> + Send + '_>>;
}

#[derive(Clone, Default)]
pub struct AgentWrapperGateway {
    backends: BTreeMap<AgentWrapperKind, Arc<dyn AgentWrapperBackend>>,
}

impl AgentWrapperGateway {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a backend.
    ///
    /// If a backend with the same `AgentWrapperKind` is already registered, this MUST return an error.
    pub fn register(
        &mut self,
        backend: Arc<dyn AgentWrapperBackend>,
    ) -> Result<(), AgentWrapperError> {
        let kind = backend.kind();
        if self.backends.contains_key(&kind) {
            return Err(AgentWrapperError::InvalidRequest {
                message: format!("backend already registered: {}", kind.as_str()),
            });
        }
        self.backends.insert(kind, backend);
        Ok(())
    }

    /// Resolves a backend by `AgentWrapperKind`.
    pub fn backend(&self, agent_kind: &AgentWrapperKind) -> Option<Arc<dyn AgentWrapperBackend>> {
        self.backends.get(agent_kind).cloned()
    }

    /// Convenience entrypoint: resolves a backend and starts a run.
    ///
    /// This MUST return `AgentWrapperError::UnknownBackend` when no backend is registered for `agent_kind`.
    pub fn run(
        &self,
        agent_kind: &AgentWrapperKind,
        request: AgentWrapperRunRequest,
    ) -> Pin<Box<dyn Future<Output = Result<AgentWrapperRunHandle, AgentWrapperError>> + Send + '_>>
    {
        let backend = self.backends.get(agent_kind).cloned();
        let agent_kind = agent_kind.as_str().to_string();
        Box::pin(async move {
            let backend = backend.ok_or(AgentWrapperError::UnknownBackend { agent_kind })?;
            backend.run(request).await
        })
    }
}

pub mod backends {
    #[cfg(feature = "codex")]
    pub mod codex {
        use std::{collections::BTreeMap, path::PathBuf, pin::Pin, time::Duration};

        use super::super::{
            AgentWrapperBackend, AgentWrapperCapabilities, AgentWrapperError, AgentWrapperKind,
            AgentWrapperRunHandle, AgentWrapperRunRequest,
        };

        #[derive(Clone, Debug, Default)]
        pub struct CodexBackendConfig {
            pub binary: Option<PathBuf>,
            pub codex_home: Option<PathBuf>,
            pub default_timeout: Option<Duration>,
            pub default_working_dir: Option<PathBuf>,
            pub env: BTreeMap<String, String>,
        }

        pub struct CodexBackend {
            _config: CodexBackendConfig,
        }

        impl CodexBackend {
            pub fn new(config: CodexBackendConfig) -> Self {
                Self { _config: config }
            }
        }

        impl AgentWrapperBackend for CodexBackend {
            fn kind(&self) -> AgentWrapperKind {
                AgentWrapperKind("codex".to_string())
            }

            fn capabilities(&self) -> AgentWrapperCapabilities {
                let mut ids = std::collections::BTreeSet::new();
                ids.insert("agent_api.run".to_string());
                ids.insert("agent_api.events".to_string());
                AgentWrapperCapabilities { ids }
            }

            fn run(
                &self,
                request: AgentWrapperRunRequest,
            ) -> Pin<
                Box<
                    dyn std::future::Future<
                            Output = Result<AgentWrapperRunHandle, AgentWrapperError>,
                        > + Send
                        + '_,
                >,
            > {
                Box::pin(async move {
                    if let Some((capability, _)) = request.extensions.into_iter().next() {
                        return Err(AgentWrapperError::UnsupportedCapability {
                            agent_kind: "codex".to_string(),
                            capability,
                        });
                    }
                    Err(AgentWrapperError::Backend {
                        message: "codex backend not implemented in C0".to_string(),
                    })
                })
            }
        }
    }

    #[cfg(feature = "claude_code")]
    pub mod claude_code {
        use std::{collections::BTreeMap, path::PathBuf, pin::Pin, time::Duration};

        use super::super::{
            AgentWrapperBackend, AgentWrapperCapabilities, AgentWrapperError, AgentWrapperKind,
            AgentWrapperRunHandle, AgentWrapperRunRequest,
        };

        #[derive(Clone, Debug, Default)]
        pub struct ClaudeCodeBackendConfig {
            pub binary: Option<PathBuf>,
            pub default_timeout: Option<Duration>,
            pub default_working_dir: Option<PathBuf>,
            pub env: BTreeMap<String, String>,
        }

        pub struct ClaudeCodeBackend {
            _config: ClaudeCodeBackendConfig,
        }

        impl ClaudeCodeBackend {
            pub fn new(config: ClaudeCodeBackendConfig) -> Self {
                Self { _config: config }
            }
        }

        impl AgentWrapperBackend for ClaudeCodeBackend {
            fn kind(&self) -> AgentWrapperKind {
                AgentWrapperKind("claude_code".to_string())
            }

            fn capabilities(&self) -> AgentWrapperCapabilities {
                let mut ids = std::collections::BTreeSet::new();
                ids.insert("agent_api.run".to_string());
                ids.insert("agent_api.events".to_string());
                AgentWrapperCapabilities { ids }
            }

            fn run(
                &self,
                request: AgentWrapperRunRequest,
            ) -> Pin<
                Box<
                    dyn std::future::Future<
                            Output = Result<AgentWrapperRunHandle, AgentWrapperError>,
                        > + Send
                        + '_,
                >,
            > {
                Box::pin(async move {
                    if let Some((capability, _)) = request.extensions.into_iter().next() {
                        return Err(AgentWrapperError::UnsupportedCapability {
                            agent_kind: "claude_code".to_string(),
                            capability,
                        });
                    }
                    Err(AgentWrapperError::Backend {
                        message: "claude_code backend not implemented in C0".to_string(),
                    })
                })
            }
        }
    }
}

fn validate_agent_kind(value: &str) -> Result<(), AgentWrapperError> {
    if value.is_empty() {
        return Err(AgentWrapperError::InvalidAgentKind {
            message: "agent kind must not be empty".to_string(),
        });
    }
    let mut chars = value.chars();
    let first = chars.next().unwrap_or_default();
    if !first.is_ascii_lowercase() {
        return Err(AgentWrapperError::InvalidAgentKind {
            message: "agent kind must start with a lowercase ASCII letter".to_string(),
        });
    }
    for ch in chars {
        if !(ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_') {
            return Err(AgentWrapperError::InvalidAgentKind {
                message: "agent kind must match ^[a-z][a-z0-9_]*$".to_string(),
            });
        }
    }
    Ok(())
}
