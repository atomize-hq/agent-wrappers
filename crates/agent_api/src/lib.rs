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
        use std::{
            collections::{BTreeMap, BTreeSet},
            future::Future,
            path::PathBuf,
            pin::Pin,
            process::Stdio,
            task::{Context, Poll},
            time::Duration,
        };

        use codex::{ExecStreamError, JsonlThreadEventParser, ThreadEvent};
        use futures_core::Stream;
        use tokio::{
            io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
            process::Command,
            sync::{mpsc, oneshot},
        };

        use super::super::{
            AgentWrapperBackend, AgentWrapperCapabilities, AgentWrapperCompletion,
            AgentWrapperError, AgentWrapperEvent, AgentWrapperEventKind, AgentWrapperKind,
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
            config: CodexBackendConfig,
        }

        impl CodexBackend {
            pub fn new(config: CodexBackendConfig) -> Self {
                Self { config }
            }
        }

        pub fn map_thread_event(event: &ThreadEvent) -> AgentWrapperEvent {
            match event {
                ThreadEvent::ThreadStarted(_) => status_event(None),
                ThreadEvent::TurnStarted(_) => status_event(None),
                ThreadEvent::TurnCompleted(_) => status_event(None),
                ThreadEvent::TurnFailed(_) => status_event(Some("turn failed".to_string())),
                ThreadEvent::Error(err) => error_event(err.message.clone()),
                ThreadEvent::ItemFailed(envelope) => {
                    error_event(envelope.item.error.message.clone())
                }
                ThreadEvent::ItemStarted(envelope) | ThreadEvent::ItemCompleted(envelope) => {
                    map_item_payload(&envelope.item.payload)
                }
                ThreadEvent::ItemDelta(delta) => map_item_delta(&delta.delta),
            }
        }

        fn status_event(message: Option<String>) -> AgentWrapperEvent {
            AgentWrapperEvent {
                agent_kind: AgentWrapperKind("codex".to_string()),
                kind: AgentWrapperEventKind::Status,
                channel: Some("status".to_string()),
                text: None,
                message,
                data: None,
            }
        }

        fn error_event(message: String) -> AgentWrapperEvent {
            AgentWrapperEvent {
                agent_kind: AgentWrapperKind("codex".to_string()),
                kind: AgentWrapperEventKind::Error,
                channel: Some("error".to_string()),
                text: None,
                message: Some(message),
                data: None,
            }
        }

        fn map_item_payload(payload: &codex::ItemPayload) -> AgentWrapperEvent {
            match payload {
                codex::ItemPayload::AgentMessage(content)
                | codex::ItemPayload::Reasoning(content) => AgentWrapperEvent {
                    agent_kind: AgentWrapperKind("codex".to_string()),
                    kind: AgentWrapperEventKind::TextOutput,
                    channel: Some("assistant".to_string()),
                    text: Some(content.text.clone()),
                    message: None,
                    data: None,
                },
                codex::ItemPayload::CommandExecution(_)
                | codex::ItemPayload::FileChange(_)
                | codex::ItemPayload::McpToolCall(_)
                | codex::ItemPayload::WebSearch(_) => AgentWrapperEvent {
                    agent_kind: AgentWrapperKind("codex".to_string()),
                    kind: AgentWrapperEventKind::ToolCall,
                    channel: Some("tool".to_string()),
                    text: None,
                    message: None,
                    data: None,
                },
                codex::ItemPayload::TodoList(_) => status_event(None),
                codex::ItemPayload::Error(err) => error_event(err.message.clone()),
            }
        }

        fn map_item_delta(delta: &codex::ItemDeltaPayload) -> AgentWrapperEvent {
            match delta {
                codex::ItemDeltaPayload::AgentMessage(content)
                | codex::ItemDeltaPayload::Reasoning(content) => AgentWrapperEvent {
                    agent_kind: AgentWrapperKind("codex".to_string()),
                    kind: AgentWrapperEventKind::TextOutput,
                    channel: Some("assistant".to_string()),
                    text: Some(content.text_delta.clone()),
                    message: None,
                    data: None,
                },
                codex::ItemDeltaPayload::CommandExecution(_)
                | codex::ItemDeltaPayload::FileChange(_)
                | codex::ItemDeltaPayload::McpToolCall(_)
                | codex::ItemDeltaPayload::WebSearch(_) => AgentWrapperEvent {
                    agent_kind: AgentWrapperKind("codex".to_string()),
                    kind: AgentWrapperEventKind::ToolCall,
                    channel: Some("tool".to_string()),
                    text: None,
                    message: None,
                    data: None,
                },
                codex::ItemDeltaPayload::TodoList(_) => status_event(None),
                codex::ItemDeltaPayload::Error(err) => error_event(err.message.clone()),
            }
        }

        fn redacted_exec_error(err: &ExecStreamError) -> String {
            match err {
                ExecStreamError::Codex(_) => "codex error".to_string(),
                ExecStreamError::Parse { source, .. } => format!("parse error: {source}"),
                ExecStreamError::Normalize { message, .. } => format!("normalize error: {message}"),
                ExecStreamError::IdleTimeout { .. } => "idle timeout".to_string(),
                ExecStreamError::ChannelClosed => "stream closed unexpectedly".to_string(),
            }
        }

        struct ReceiverEventStream {
            rx: mpsc::Receiver<AgentWrapperEvent>,
        }

        impl Stream for ReceiverEventStream {
            type Item = AgentWrapperEvent;

            fn poll_next(
                mut self: Pin<&mut Self>,
                cx: &mut Context<'_>,
            ) -> Poll<Option<Self::Item>> {
                Pin::new(&mut self.rx).poll_recv(cx)
            }
        }

        impl AgentWrapperBackend for CodexBackend {
            fn kind(&self) -> AgentWrapperKind {
                AgentWrapperKind("codex".to_string())
            }

            fn capabilities(&self) -> AgentWrapperCapabilities {
                let mut ids = BTreeSet::new();
                ids.insert("agent_api.run".to_string());
                ids.insert("agent_api.events".to_string());
                ids.insert("agent_api.events.live".to_string());
                AgentWrapperCapabilities { ids }
            }

            fn run(
                &self,
                request: AgentWrapperRunRequest,
            ) -> Pin<
                Box<
                    dyn Future<Output = Result<AgentWrapperRunHandle, AgentWrapperError>>
                        + Send
                        + '_,
                >,
            > {
                let config = self.config.clone();
                Box::pin(async move { run_impl(config, request).await })
            }
        }

        async fn run_impl(
            config: CodexBackendConfig,
            request: AgentWrapperRunRequest,
        ) -> Result<AgentWrapperRunHandle, AgentWrapperError> {
            if request.prompt.trim().is_empty() {
                return Err(AgentWrapperError::InvalidRequest {
                    message: "prompt must not be empty".to_string(),
                });
            }

            if let Some((capability, _)) = request.extensions.iter().next() {
                return Err(AgentWrapperError::UnsupportedCapability {
                    agent_kind: "codex".to_string(),
                    capability: capability.clone(),
                });
            }

            let (tx, rx) = mpsc::channel::<AgentWrapperEvent>(32);
            let (completion_tx, completion_rx) =
                oneshot::channel::<Result<AgentWrapperCompletion, AgentWrapperError>>();

            tokio::spawn(async move {
                let result = run_codex(config, request, tx).await;
                let _ = completion_tx.send(result);
            });

            let events: super::super::DynAgentWrapperEventStream =
                Box::pin(ReceiverEventStream { rx });

            let completion: super::super::DynAgentWrapperCompletion = Box::pin(async move {
                completion_rx.await.unwrap_or_else(|_| {
                    Err(AgentWrapperError::Backend {
                        message: "completion channel dropped".to_string(),
                    })
                })
            });

            Ok(AgentWrapperRunHandle { events, completion })
        }

        async fn run_codex(
            config: CodexBackendConfig,
            request: AgentWrapperRunRequest,
            tx: mpsc::Sender<AgentWrapperEvent>,
        ) -> Result<AgentWrapperCompletion, AgentWrapperError> {
            let timeout = request.timeout.or(config.default_timeout);
            if let Some(timeout) = timeout {
                return tokio::time::timeout(timeout, run_codex_inner(config, request, tx))
                    .await
                    .map_err(|_| AgentWrapperError::Backend {
                        message: format!("codex exceeded timeout of {timeout:?}"),
                    })?;
            }

            run_codex_inner(config, request, tx).await
        }

        async fn run_codex_inner(
            config: CodexBackendConfig,
            request: AgentWrapperRunRequest,
            tx: mpsc::Sender<AgentWrapperEvent>,
        ) -> Result<AgentWrapperCompletion, AgentWrapperError> {
            let binary = config.binary.unwrap_or_else(|| PathBuf::from("codex"));
            let mut command = Command::new(binary);
            command
                .arg("exec")
                .arg("--color")
                .arg("never")
                .arg("--skip-git-repo-check")
                .arg("--json")
                .arg("--full-auto")
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .stdin(Stdio::piped())
                .kill_on_drop(true);

            let working_dir = request
                .working_dir
                .clone()
                .or_else(|| config.default_working_dir.clone());
            if let Some(dir) = working_dir {
                command.current_dir(dir);
            }

            if let Some(codex_home) = config.codex_home.clone() {
                command.env("CODEX_HOME", codex_home);
            }

            for (k, v) in config.env.iter() {
                command.env(k, v);
            }
            for (k, v) in request.env.iter() {
                command.env(k, v);
            }

            let mut child = command.spawn().map_err(|err| AgentWrapperError::Backend {
                message: format!("failed to spawn codex: {err}"),
            })?;

            {
                let mut stdin = child
                    .stdin
                    .take()
                    .ok_or_else(|| AgentWrapperError::Backend {
                        message: "stdin unavailable".to_string(),
                    })?;
                if let Err(err) = stdin.write_all(request.prompt.as_bytes()).await {
                    if err.kind() != std::io::ErrorKind::BrokenPipe {
                        return Err(AgentWrapperError::Backend {
                            message: format!("failed to write stdin: {err}"),
                        });
                    }
                }
                let _ = stdin.write_all(b"\n").await;
                let _ = stdin.shutdown().await;
            }

            let stdout = child
                .stdout
                .take()
                .ok_or_else(|| AgentWrapperError::Backend {
                    message: "stdout unavailable".to_string(),
                })?;

            let mut parser = JsonlThreadEventParser::new();
            let mut lines = BufReader::new(stdout).lines();

            while let Ok(Some(line)) = lines.next_line().await {
                match parser.parse_line(&line) {
                    Ok(None) => {}
                    Ok(Some(event)) => {
                        if tx.send(map_thread_event(&event)).await.is_err() {
                            break;
                        }
                    }
                    Err(err) => {
                        let message = redacted_exec_error(&err);
                        let _ = tx.send(error_event(message)).await;
                    }
                }
            }

            let status = child
                .wait()
                .await
                .map_err(|err| AgentWrapperError::Backend {
                    message: format!("failed to wait for codex: {err}"),
                })?;

            drop(tx);

            Ok(AgentWrapperCompletion {
                status,
                final_text: None,
                data: None,
            })
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
