#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::process::ExitStatus;
use std::sync::Arc;
use std::time::Duration;

use futures_core::Stream;

mod bounds;

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

        fn map_thread_event(event: &ThreadEvent) -> AgentWrapperEvent {
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
                .stderr(Stdio::null())
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
                        for mapped in crate::bounds::enforce_event_bounds(map_thread_event(&event))
                        {
                            if tx.send(mapped).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(err) => {
                        let message = redacted_exec_error(&err);
                        for mapped in crate::bounds::enforce_event_bounds(error_event(message)) {
                            if tx.send(mapped).await.is_err() {
                                break;
                            }
                        }
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

            Ok(crate::bounds::enforce_completion_bounds(
                AgentWrapperCompletion {
                    status,
                    final_text: None,
                    data: None,
                },
            ))
        }

        #[cfg(all(test, feature = "codex"))]
        mod tests {
            use super::*;
            use crate::{AgentWrapperBackend, AgentWrapperEventKind};

            fn parse_thread_event(json: &str) -> ThreadEvent {
                serde_json::from_str(json).expect("valid codex::ThreadEvent JSON")
            }

            fn map(json: &str) -> AgentWrapperEvent {
                let event = parse_thread_event(json);
                map_thread_event(&event)
            }

            #[test]
            fn codex_backend_reports_required_capabilities() {
                let backend = CodexBackend::new(CodexBackendConfig::default());
                let capabilities = backend.capabilities();
                assert!(capabilities.contains("agent_api.run"));
                assert!(capabilities.contains("agent_api.events"));
                assert!(capabilities.contains("agent_api.events.live"));
            }

            #[test]
            fn thread_started_maps_to_status() {
                let mapped = map(r#"{"type":"thread.started","thread_id":"thread-1"}"#);
                assert_eq!(mapped.agent_kind.as_str(), "codex");
                assert_eq!(mapped.kind, AgentWrapperEventKind::Status);
                assert_eq!(mapped.text, None);
            }

            #[test]
            fn turn_started_maps_to_status() {
                let mapped =
                    map(r#"{"type":"turn.started","thread_id":"thread-1","turn_id":"turn-1"}"#);
                assert_eq!(mapped.kind, AgentWrapperEventKind::Status);
                assert_eq!(mapped.text, None);
            }

            #[test]
            fn turn_completed_maps_to_status() {
                let mapped =
                    map(r#"{"type":"turn.completed","thread_id":"thread-1","turn_id":"turn-1"}"#);
                assert_eq!(mapped.kind, AgentWrapperEventKind::Status);
                assert_eq!(mapped.text, None);
            }

            #[test]
            fn turn_failed_maps_to_status() {
                let mapped = map(
                    r#"{"type":"turn.failed","thread_id":"thread-1","turn_id":"turn-1","error":{"message":"boom"}}"#,
                );
                assert_eq!(mapped.kind, AgentWrapperEventKind::Status);
                assert_eq!(mapped.text, None);
            }

            #[test]
            fn transport_error_maps_to_error_with_message() {
                let mapped = map(r#"{"type":"error","message":"transport failed"}"#);
                assert_eq!(mapped.kind, AgentWrapperEventKind::Error);
                assert_eq!(mapped.text, None);
                assert!(mapped.message.is_some());
            }

            #[test]
            fn item_failed_maps_to_error_with_message() {
                let mapped = map(
                    r#"{"type":"item.failed","thread_id":"thread-1","turn_id":"turn-1","item_id":"item-1","error":{"message":"tool failed"}}"#,
                );
                assert_eq!(mapped.kind, AgentWrapperEventKind::Error);
                assert_eq!(mapped.text, None);
                assert!(mapped.message.is_some());
            }

            #[test]
            fn agent_message_item_maps_to_text_output_and_uses_text_field() {
                let mapped = map(
                    r#"{"type":"item.started","thread_id":"thread-1","turn_id":"turn-1","item_id":"item-1","item_type":"agent_message","content":{"text":"hello"}}"#,
                );
                assert_eq!(mapped.kind, AgentWrapperEventKind::TextOutput);
                assert_eq!(mapped.text.as_deref(), Some("hello"));
                assert_eq!(mapped.message, None);
            }

            #[test]
            fn agent_message_delta_maps_to_text_output_and_uses_text_field() {
                let mapped = map(
                    r#"{"type":"item.delta","thread_id":"thread-1","turn_id":"turn-1","item_id":"item-1","item_type":"agent_message","delta":{"text_delta":"hi"}}"#,
                );
                assert_eq!(mapped.kind, AgentWrapperEventKind::TextOutput);
                assert_eq!(mapped.text.as_deref(), Some("hi"));
                assert_eq!(mapped.message, None);
            }

            #[test]
            fn reasoning_item_maps_to_text_output_and_uses_text_field() {
                let mapped = map(
                    r#"{"type":"item.completed","thread_id":"thread-1","turn_id":"turn-1","item_id":"item-2","item_type":"reasoning","content":{"text":"thinking"}}"#,
                );
                assert_eq!(mapped.kind, AgentWrapperEventKind::TextOutput);
                assert_eq!(mapped.text.as_deref(), Some("thinking"));
                assert_eq!(mapped.message, None);
            }

            #[test]
            fn command_execution_item_maps_to_tool_call() {
                let mapped = map(
                    r#"{"type":"item.started","thread_id":"thread-1","turn_id":"turn-1","item_id":"item-3","item_type":"command_execution","content":{"command":"echo hi"}}"#,
                );
                assert_eq!(mapped.kind, AgentWrapperEventKind::ToolCall);
            }

            #[test]
            fn todo_list_item_maps_to_status() {
                let mapped = map(
                    r#"{"type":"item.started","thread_id":"thread-1","turn_id":"turn-1","item_id":"item-4","item_type":"todo_list","content":{"items":[{"title":"one","completed":false}]}}"#,
                );
                assert_eq!(mapped.kind, AgentWrapperEventKind::Status);
            }

            #[test]
            fn item_payload_error_maps_to_error_with_message() {
                let mapped = map(
                    r#"{"type":"item.completed","thread_id":"thread-1","turn_id":"turn-1","item_id":"item-5","item_type":"error","content":{"message":"bad"}}"#,
                );
                assert_eq!(mapped.kind, AgentWrapperEventKind::Error);
                assert!(mapped.message.is_some());
            }
        }
    }

    #[cfg(feature = "claude_code")]
    pub mod claude_code {
        use std::{
            collections::{BTreeMap, BTreeSet},
            future::Future,
            path::PathBuf,
            pin::Pin,
            task::{Context, Poll},
            time::Duration,
        };

        use claude_code::{ClaudeOutputFormat, ClaudePrintRequest, ClaudeStreamJsonParser};
        use futures_core::Stream;
        use tokio::sync::{mpsc, oneshot};

        use super::super::{
            AgentWrapperBackend, AgentWrapperCapabilities, AgentWrapperCompletion,
            AgentWrapperError, AgentWrapperEvent, AgentWrapperEventKind, AgentWrapperKind,
            AgentWrapperRunHandle, AgentWrapperRunRequest,
        };

        const AGENT_KIND: &str = "claude_code";
        const CHANNEL_ASSISTANT: &str = "assistant";
        const CHANNEL_TOOL: &str = "tool";

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

        #[derive(Clone, Debug, Default)]
        pub struct ClaudeCodeBackendConfig {
            pub binary: Option<PathBuf>,
            pub default_timeout: Option<Duration>,
            pub default_working_dir: Option<PathBuf>,
            pub env: BTreeMap<String, String>,
        }

        pub struct ClaudeCodeBackend {
            config: ClaudeCodeBackendConfig,
        }

        impl ClaudeCodeBackend {
            pub fn new(config: ClaudeCodeBackendConfig) -> Self {
                Self { config }
            }
        }

        impl AgentWrapperBackend for ClaudeCodeBackend {
            fn kind(&self) -> AgentWrapperKind {
                AgentWrapperKind(AGENT_KIND.to_string())
            }

            fn capabilities(&self) -> AgentWrapperCapabilities {
                let mut ids = BTreeSet::new();
                ids.insert("agent_api.run".to_string());
                ids.insert("agent_api.events".to_string());
                ids.insert("backend.claude_code.print_stream_json".to_string());
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
            config: ClaudeCodeBackendConfig,
            request: AgentWrapperRunRequest,
        ) -> Result<AgentWrapperRunHandle, AgentWrapperError> {
            if request.prompt.trim().is_empty() {
                return Err(AgentWrapperError::InvalidRequest {
                    message: "prompt must not be empty".to_string(),
                });
            }

            if let Some((capability, _)) = request.extensions.iter().next() {
                return Err(AgentWrapperError::UnsupportedCapability {
                    agent_kind: AGENT_KIND.to_string(),
                    capability: capability.clone(),
                });
            }

            let (tx, rx) = mpsc::channel::<AgentWrapperEvent>(32);
            let (completion_tx, completion_rx) =
                oneshot::channel::<Result<AgentWrapperCompletion, AgentWrapperError>>();

            tokio::spawn(async move {
                let result = run_claude_code(config, request, tx).await;
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

        async fn run_claude_code(
            config: ClaudeCodeBackendConfig,
            request: AgentWrapperRunRequest,
            tx: mpsc::Sender<AgentWrapperEvent>,
        ) -> Result<AgentWrapperCompletion, AgentWrapperError> {
            let timeout = request.timeout.or(config.default_timeout);
            if let Some(timeout) = timeout {
                return tokio::time::timeout(timeout, run_claude_code_inner(config, request, tx))
                    .await
                    .map_err(|_| AgentWrapperError::Backend {
                        message: format!("claude_code exceeded timeout of {timeout:?}"),
                    })?;
            }

            run_claude_code_inner(config, request, tx).await
        }

        async fn run_claude_code_inner(
            config: ClaudeCodeBackendConfig,
            request: AgentWrapperRunRequest,
            tx: mpsc::Sender<AgentWrapperEvent>,
        ) -> Result<AgentWrapperCompletion, AgentWrapperError> {
            let mut builder = claude_code::ClaudeClient::builder();
            if let Some(binary) = config.binary.as_ref() {
                builder = builder.binary(binary.clone());
            }

            let working_dir = request
                .working_dir
                .clone()
                .or_else(|| config.default_working_dir.clone());
            if let Some(dir) = working_dir {
                builder = builder.working_dir(dir);
            }

            builder = builder.timeout(request.timeout.or(config.default_timeout));

            for (k, v) in config.env.iter() {
                builder = builder.env(k.clone(), v.clone());
            }
            for (k, v) in request.env.iter() {
                builder = builder.env(k.clone(), v.clone());
            }

            let client = builder.build();
            let print_req = ClaudePrintRequest::new(request.prompt)
                .output_format(ClaudeOutputFormat::StreamJson)
                .include_partial_messages(true);

            let res = match client.print(print_req).await {
                Ok(res) => res,
                Err(err) => {
                    let _ = tx
                        .send(error_event(format!("claude_code error: {err}")))
                        .await;
                    drop(tx);
                    return Err(AgentWrapperError::Backend {
                        message: format!("claude_code error: {err}"),
                    });
                }
            };

            let stdout = String::from_utf8_lossy(&res.output.stdout);
            let mut parser = ClaudeStreamJsonParser::new();

            for line in stdout.lines() {
                match parser.parse_line(line) {
                    Ok(None) => {}
                    Ok(Some(ev)) => {
                        let mapped = map_stream_json_event(ev);
                        for event in mapped {
                            for bounded in crate::bounds::enforce_event_bounds(event) {
                                if tx.send(bounded).await.is_err() {
                                    break;
                                }
                            }
                        }
                    }
                    Err(err) => {
                        for bounded in crate::bounds::enforce_event_bounds(error_event(
                            redact_parse_error(&err),
                        )) {
                            if tx.send(bounded).await.is_err() {
                                break;
                            }
                        }
                    }
                }
            }

            drop(tx);

            Ok(crate::bounds::enforce_completion_bounds(
                AgentWrapperCompletion {
                    status: res.output.status,
                    final_text: None,
                    data: None,
                },
            ))
        }

        fn map_stream_json_event(ev: claude_code::ClaudeStreamJsonEvent) -> Vec<AgentWrapperEvent> {
            match ev {
                claude_code::ClaudeStreamJsonEvent::SystemInit { .. } => {
                    vec![status_event(Some("system init".to_string()))]
                }
                claude_code::ClaudeStreamJsonEvent::SystemOther { subtype, .. } => {
                    vec![status_event(Some(format!("system {subtype}")))]
                }
                claude_code::ClaudeStreamJsonEvent::ResultError { .. } => {
                    vec![error_event("result error".to_string())]
                }
                claude_code::ClaudeStreamJsonEvent::ResultSuccess { .. } => {
                    vec![status_event(Some("result success".to_string()))]
                }
                claude_code::ClaudeStreamJsonEvent::AssistantMessage { raw, .. } => {
                    map_assistant_message(&raw)
                }
                claude_code::ClaudeStreamJsonEvent::StreamEvent { stream, .. } => {
                    map_stream_event(&stream.raw)
                }
                claude_code::ClaudeStreamJsonEvent::UserMessage { .. } => vec![status_event(None)],
                claude_code::ClaudeStreamJsonEvent::Unknown { .. } => vec![unknown_event()],
            }
        }

        fn map_assistant_message(raw: &serde_json::Value) -> Vec<AgentWrapperEvent> {
            let Some(blocks) = raw
                .get("message")
                .and_then(|m| m.get("content"))
                .and_then(|c| c.as_array())
            else {
                return vec![unknown_event()];
            };

            let mut out = Vec::new();
            for block in blocks {
                let Some(obj) = block.as_object() else {
                    out.push(unknown_event());
                    continue;
                };
                let block_type = obj.get("type").and_then(|v| v.as_str()).unwrap_or("");
                match block_type {
                    "text" => {
                        if let Some(text) = obj.get("text").and_then(|v| v.as_str()) {
                            out.extend(text_output_events(text, Some(CHANNEL_ASSISTANT)));
                        } else {
                            out.push(unknown_event());
                        }
                    }
                    "tool_use" => out.push(tool_call_event()),
                    "tool_result" => out.push(tool_result_event()),
                    _ => out.push(unknown_event()),
                }
            }
            out
        }

        fn map_stream_event(raw: &serde_json::Value) -> Vec<AgentWrapperEvent> {
            let Some(obj) = raw.as_object() else {
                return vec![unknown_event()];
            };
            let event_type = obj.get("type").and_then(|v| v.as_str()).unwrap_or("");
            match event_type {
                "content_block_start" => {
                    let Some(content_block) = obj.get("content_block").and_then(|v| v.as_object())
                    else {
                        return vec![unknown_event()];
                    };
                    let block_type = content_block
                        .get("type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    match block_type {
                        "tool_use" => vec![tool_call_event()],
                        "tool_result" => vec![tool_result_event()],
                        _ => vec![status_event(None)],
                    }
                }
                "content_block_delta" => {
                    let Some(delta) = obj.get("delta").and_then(|v| v.as_object()) else {
                        return vec![unknown_event()];
                    };
                    let delta_type = delta.get("type").and_then(|v| v.as_str()).unwrap_or("");
                    match delta_type {
                        "text_delta" => {
                            let Some(text) = delta.get("text").and_then(|v| v.as_str()) else {
                                return vec![unknown_event()];
                            };
                            text_output_events(text, Some(CHANNEL_ASSISTANT))
                        }
                        "input_json_delta" => vec![tool_call_event()],
                        _ => vec![unknown_event()],
                    }
                }
                _ => vec![status_event(None)],
            }
        }

        fn tool_call_event() -> AgentWrapperEvent {
            AgentWrapperEvent {
                agent_kind: AgentWrapperKind(AGENT_KIND.to_string()),
                kind: AgentWrapperEventKind::ToolCall,
                channel: Some(CHANNEL_TOOL.to_string()),
                text: None,
                message: None,
                data: None,
            }
        }

        fn tool_result_event() -> AgentWrapperEvent {
            AgentWrapperEvent {
                agent_kind: AgentWrapperKind(AGENT_KIND.to_string()),
                kind: AgentWrapperEventKind::ToolResult,
                channel: Some(CHANNEL_TOOL.to_string()),
                text: None,
                message: None,
                data: None,
            }
        }

        fn status_event(message: Option<String>) -> AgentWrapperEvent {
            AgentWrapperEvent {
                agent_kind: AgentWrapperKind(AGENT_KIND.to_string()),
                kind: AgentWrapperEventKind::Status,
                channel: Some("status".to_string()),
                text: None,
                message,
                data: None,
            }
        }

        fn error_event(message: String) -> AgentWrapperEvent {
            AgentWrapperEvent {
                agent_kind: AgentWrapperKind(AGENT_KIND.to_string()),
                kind: AgentWrapperEventKind::Error,
                channel: Some("error".to_string()),
                text: None,
                message: Some(message),
                data: None,
            }
        }

        fn unknown_event() -> AgentWrapperEvent {
            AgentWrapperEvent {
                agent_kind: AgentWrapperKind(AGENT_KIND.to_string()),
                kind: AgentWrapperEventKind::Unknown,
                channel: None,
                text: None,
                message: None,
                data: None,
            }
        }

        fn text_output_events(text: &str, channel: Option<&str>) -> Vec<AgentWrapperEvent> {
            vec![AgentWrapperEvent {
                agent_kind: AgentWrapperKind(AGENT_KIND.to_string()),
                kind: AgentWrapperEventKind::TextOutput,
                channel: channel.map(|c| c.to_string()),
                text: Some(text.to_string()),
                message: None,
                data: None,
            }]
        }

        fn redact_parse_error(err: &claude_code::ClaudeStreamJsonParseError) -> String {
            err.message.clone()
        }

        #[cfg(all(test, feature = "claude_code"))]
        mod tests {
            use super::*;
            use crate::{AgentWrapperBackend, AgentWrapperEventKind};
            use claude_code::{ClaudeStreamJsonEvent, ClaudeStreamJsonParser};

            const SYSTEM_INIT: &str =
                include_str!("../../claude_code/tests/fixtures/stream_json/v1/system_init.jsonl");
            const SYSTEM_OTHER: &str =
                include_str!("../../claude_code/tests/fixtures/stream_json/v1/system_other.jsonl");
            const RESULT_ERROR: &str =
                include_str!("../../claude_code/tests/fixtures/stream_json/v1/result_error.jsonl");
            const ASSISTANT_MESSAGE_TEXT: &str = include_str!(
                "../../claude_code/tests/fixtures/stream_json/v1/assistant_message_text.jsonl"
            );
            const ASSISTANT_MESSAGE_TOOL_USE: &str = include_str!(
                "../../claude_code/tests/fixtures/stream_json/v1/assistant_message_tool_use.jsonl"
            );
            const ASSISTANT_MESSAGE_TOOL_RESULT: &str = include_str!(
                "../../claude_code/tests/fixtures/stream_json/v1/assistant_message_tool_result.jsonl"
            );
            const STREAM_EVENT_TEXT_DELTA: &str = include_str!(
                "../../claude_code/tests/fixtures/stream_json/v1/stream_event_text_delta.jsonl"
            );
            const STREAM_EVENT_INPUT_JSON_DELTA: &str = include_str!(
                "../../claude_code/tests/fixtures/stream_json/v1/stream_event_input_json_delta.jsonl"
            );
            const STREAM_EVENT_TOOL_USE_START: &str = include_str!(
                "../../claude_code/tests/fixtures/stream_json/v1/stream_event_tool_use_start.jsonl"
            );
            const STREAM_EVENT_TOOL_RESULT_START: &str = include_str!(
                "../../claude_code/tests/fixtures/stream_json/v1/stream_event_tool_result_start.jsonl"
            );
            const UNKNOWN_OUTER_TYPE: &str = include_str!(
                "../../claude_code/tests/fixtures/stream_json/v1/unknown_outer_type.jsonl"
            );

            fn parse_stream_json_fixture(text: &str) -> ClaudeStreamJsonEvent {
                let line = text
                    .lines()
                    .find(|line| !line.chars().all(|ch| ch.is_whitespace()))
                    .expect("fixture contains a non-empty line");
                let mut parser = ClaudeStreamJsonParser::new();
                parser
                    .parse_line(line)
                    .expect("fixture parses")
                    .expect("fixture yields a typed event")
            }

            fn map_fixture(text: &str) -> AgentWrapperEvent {
                let event = parse_stream_json_fixture(text);
                let mapped = map_stream_json_event(event);
                assert_eq!(
                    mapped.len(),
                    1,
                    "fixture should map to exactly one wrapper event"
                );
                mapped
                    .into_iter()
                    .next()
                    .expect("fixture mapping returns at least one event")
            }

            #[test]
            fn claude_backend_reports_required_capabilities() {
                let backend = ClaudeCodeBackend::new(ClaudeCodeBackendConfig::default());
                let capabilities = backend.capabilities();
                assert!(capabilities.contains("agent_api.run"));
                assert!(capabilities.contains("agent_api.events"));
                assert!(!capabilities.contains("agent_api.events.live"));
            }

            #[test]
            fn claude_backend_registers_under_claude_code_kind_id() {
                let backend = ClaudeCodeBackend::new(ClaudeCodeBackendConfig::default());
                assert_eq!(backend.kind().as_str(), "claude_code");
            }

            #[test]
            fn system_init_maps_to_status() {
                let mapped = map_fixture(SYSTEM_INIT);
                assert_eq!(mapped.agent_kind.as_str(), "claude_code");
                assert_eq!(mapped.kind, AgentWrapperEventKind::Status);
                assert_eq!(mapped.text, None);
            }

            #[test]
            fn system_other_maps_to_status() {
                let mapped = map_fixture(SYSTEM_OTHER);
                assert_eq!(mapped.kind, AgentWrapperEventKind::Status);
                assert_eq!(mapped.text, None);
            }

            #[test]
            fn result_error_maps_to_error_with_message() {
                let mapped = map_fixture(RESULT_ERROR);
                assert_eq!(mapped.kind, AgentWrapperEventKind::Error);
                assert_eq!(mapped.text, None);
                assert!(mapped.message.is_some());
            }

            #[test]
            fn assistant_message_text_maps_to_text_output_and_uses_text_field() {
                let mapped = map_fixture(ASSISTANT_MESSAGE_TEXT);
                assert_eq!(mapped.kind, AgentWrapperEventKind::TextOutput);
                assert_eq!(mapped.text.as_deref(), Some("hello"));
                assert_eq!(mapped.message, None);
            }

            #[test]
            fn assistant_message_tool_use_maps_to_tool_call() {
                let mapped = map_fixture(ASSISTANT_MESSAGE_TOOL_USE);
                assert_eq!(mapped.kind, AgentWrapperEventKind::ToolCall);
                assert_eq!(mapped.text, None);
                assert_eq!(mapped.message, None);
            }

            #[test]
            fn assistant_message_tool_result_maps_to_tool_result() {
                let mapped = map_fixture(ASSISTANT_MESSAGE_TOOL_RESULT);
                assert_eq!(mapped.kind, AgentWrapperEventKind::ToolResult);
                assert_eq!(mapped.text, None);
                assert_eq!(mapped.message, None);
            }

            #[test]
            fn stream_event_text_delta_maps_to_text_output_and_uses_text_field() {
                let mapped = map_fixture(STREAM_EVENT_TEXT_DELTA);
                assert_eq!(mapped.kind, AgentWrapperEventKind::TextOutput);
                assert_eq!(mapped.text.as_deref(), Some("hel"));
                assert_eq!(mapped.message, None);
            }

            #[test]
            fn stream_event_input_json_delta_maps_to_tool_call() {
                let mapped = map_fixture(STREAM_EVENT_INPUT_JSON_DELTA);
                assert_eq!(mapped.kind, AgentWrapperEventKind::ToolCall);
                assert_eq!(mapped.text, None);
                assert_eq!(mapped.message, None);
            }

            #[test]
            fn stream_event_tool_use_start_maps_to_tool_call() {
                let mapped = map_fixture(STREAM_EVENT_TOOL_USE_START);
                assert_eq!(mapped.kind, AgentWrapperEventKind::ToolCall);
            }

            #[test]
            fn stream_event_tool_result_start_maps_to_tool_result() {
                let mapped = map_fixture(STREAM_EVENT_TOOL_RESULT_START);
                assert_eq!(mapped.kind, AgentWrapperEventKind::ToolResult);
            }

            #[test]
            fn unknown_outer_type_maps_to_unknown() {
                let mapped = map_fixture(UNKNOWN_OUTER_TYPE);
                assert_eq!(mapped.kind, AgentWrapperEventKind::Unknown);
                assert_eq!(mapped.text, None);
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
