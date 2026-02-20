use std::{
    collections::{BTreeMap, BTreeSet},
    future::Future,
    path::PathBuf,
    pin::Pin,
    time::Duration,
};

use codex::{CodexError, ExecStreamError, ExecStreamRequest, ThreadEvent};
use futures_util::StreamExt;
use serde_json::Value;
use tokio::sync::{mpsc, oneshot};

use crate::{
    AgentWrapperBackend, AgentWrapperCapabilities, AgentWrapperCompletion, AgentWrapperError,
    AgentWrapperEvent, AgentWrapperEventKind, AgentWrapperKind, AgentWrapperRunHandle,
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

pub struct CodexBackend {
    config: CodexBackendConfig,
}

impl CodexBackend {
    pub fn new(config: CodexBackendConfig) -> Self {
        Self { config }
    }
}

const EXT_NON_INTERACTIVE: &str = "agent_api.exec.non_interactive";
const EXT_CODEX_APPROVAL_POLICY: &str = "backend.codex.exec.approval_policy";
const EXT_CODEX_SANDBOX_MODE: &str = "backend.codex.exec.sandbox_mode";

#[derive(Clone, Debug, Eq, PartialEq)]
enum CodexApprovalPolicy {
    Untrusted,
    OnFailure,
    OnRequest,
    Never,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum CodexSandboxMode {
    ReadOnly,
    WorkspaceWrite,
    DangerFullAccess,
}

fn parse_bool(value: &Value, key: &str) -> Result<bool, AgentWrapperError> {
    value
        .as_bool()
        .ok_or_else(|| AgentWrapperError::InvalidRequest {
            message: format!("{key} must be a boolean"),
        })
}

fn parse_string<'a>(value: &'a Value, key: &str) -> Result<&'a str, AgentWrapperError> {
    value
        .as_str()
        .ok_or_else(|| AgentWrapperError::InvalidRequest {
            message: format!("{key} must be a string"),
        })
}

fn parse_codex_approval_policy(value: &Value) -> Result<CodexApprovalPolicy, AgentWrapperError> {
    let raw = parse_string(value, EXT_CODEX_APPROVAL_POLICY)?;
    match raw {
        "untrusted" => Ok(CodexApprovalPolicy::Untrusted),
        "on-failure" => Ok(CodexApprovalPolicy::OnFailure),
        "on-request" => Ok(CodexApprovalPolicy::OnRequest),
        "never" => Ok(CodexApprovalPolicy::Never),
        other => Err(AgentWrapperError::InvalidRequest {
            message: format!(
                "{EXT_CODEX_APPROVAL_POLICY} must be one of: untrusted | on-failure | on-request | never (got {other:?})"
            ),
        }),
    }
}

fn parse_codex_sandbox_mode(value: &Value) -> Result<CodexSandboxMode, AgentWrapperError> {
    let raw = parse_string(value, EXT_CODEX_SANDBOX_MODE)?;
    match raw {
        "read-only" => Ok(CodexSandboxMode::ReadOnly),
        "workspace-write" => Ok(CodexSandboxMode::WorkspaceWrite),
        "danger-full-access" => Ok(CodexSandboxMode::DangerFullAccess),
        other => Err(AgentWrapperError::InvalidRequest {
            message: format!(
                "{EXT_CODEX_SANDBOX_MODE} must be one of: read-only | workspace-write | danger-full-access (got {other:?})"
            ),
        }),
    }
}

#[derive(Clone, Debug)]
struct CodexExecPolicy {
    non_interactive: bool,
    approval_policy: Option<CodexApprovalPolicy>,
    sandbox_mode: CodexSandboxMode,
}

fn validate_and_extract_exec_policy(
    request: &AgentWrapperRunRequest,
) -> Result<CodexExecPolicy, AgentWrapperError> {
    for key in request.extensions.keys() {
        if key != EXT_NON_INTERACTIVE
            && key != EXT_CODEX_APPROVAL_POLICY
            && key != EXT_CODEX_SANDBOX_MODE
        {
            return Err(AgentWrapperError::UnsupportedCapability {
                agent_kind: "codex".to_string(),
                capability: key.clone(),
            });
        }
    }

    let non_interactive = request
        .extensions
        .get(EXT_NON_INTERACTIVE)
        .map(|value| parse_bool(value, EXT_NON_INTERACTIVE))
        .transpose()?
        .unwrap_or(true);

    let approval_policy = request
        .extensions
        .get(EXT_CODEX_APPROVAL_POLICY)
        .map(parse_codex_approval_policy)
        .transpose()?;

    let sandbox_mode = request
        .extensions
        .get(EXT_CODEX_SANDBOX_MODE)
        .map(parse_codex_sandbox_mode)
        .transpose()?
        .unwrap_or(CodexSandboxMode::WorkspaceWrite);

    if non_interactive
        && matches!(
            approval_policy,
            Some(ref policy) if policy != &CodexApprovalPolicy::Never
        )
    {
        return Err(AgentWrapperError::InvalidRequest {
            message: format!(
                "{EXT_CODEX_APPROVAL_POLICY} must be \"never\" when {EXT_NON_INTERACTIVE} is true"
            ),
        });
    }

    Ok(CodexExecPolicy {
        non_interactive,
        approval_policy,
        sandbox_mode,
    })
}

fn map_thread_event(event: &ThreadEvent) -> AgentWrapperEvent {
    match event {
        ThreadEvent::ThreadStarted(_) => status_event(None),
        ThreadEvent::TurnStarted(_) => status_event(None),
        ThreadEvent::TurnCompleted(_) => status_event(None),
        ThreadEvent::TurnFailed(_) => status_event(Some("turn failed".to_string())),
        ThreadEvent::Error(err) => error_event(err.message.clone()),
        ThreadEvent::ItemFailed(envelope) => error_event(envelope.item.error.message.clone()),
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
        codex::ItemPayload::AgentMessage(content) | codex::ItemPayload::Reasoning(content) => {
            AgentWrapperEvent {
                agent_kind: AgentWrapperKind("codex".to_string()),
                kind: AgentWrapperEventKind::TextOutput,
                channel: Some("assistant".to_string()),
                text: Some(content.text.clone()),
                message: None,
                data: None,
            }
        }
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

impl AgentWrapperBackend for CodexBackend {
    fn kind(&self) -> AgentWrapperKind {
        AgentWrapperKind("codex".to_string())
    }

    fn capabilities(&self) -> AgentWrapperCapabilities {
        let mut ids = BTreeSet::new();
        ids.insert("agent_api.run".to_string());
        ids.insert("agent_api.events".to_string());
        ids.insert("agent_api.events.live".to_string());
        ids.insert("backend.codex.exec_stream".to_string());
        ids.insert(EXT_NON_INTERACTIVE.to_string());
        ids.insert(EXT_CODEX_APPROVAL_POLICY.to_string());
        ids.insert(EXT_CODEX_SANDBOX_MODE.to_string());
        AgentWrapperCapabilities { ids }
    }

    fn run(
        &self,
        request: AgentWrapperRunRequest,
    ) -> Pin<Box<dyn Future<Output = Result<AgentWrapperRunHandle, AgentWrapperError>> + Send + '_>>
    {
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

    let policy = validate_and_extract_exec_policy(&request)?;

    let (tx, rx) = mpsc::channel::<AgentWrapperEvent>(32);
    let (completion_tx, completion_rx) =
        oneshot::channel::<Result<AgentWrapperCompletion, AgentWrapperError>>();

    tokio::spawn(async move {
        let result = run_codex(config, request, policy, tx).await;
        let _ = completion_tx.send(result);
    });

    Ok(crate::run_handle_gate::build_gated_run_handle(
        rx,
        completion_rx,
    ))
}

async fn run_codex(
    config: CodexBackendConfig,
    request: AgentWrapperRunRequest,
    policy: CodexExecPolicy,
    tx: mpsc::Sender<AgentWrapperEvent>,
) -> Result<AgentWrapperCompletion, AgentWrapperError> {
    let timeout = request.timeout.or(config.default_timeout);
    if let Some(timeout) = timeout {
        return tokio::time::timeout(timeout, run_codex_inner(config, request, policy, tx))
            .await
            .map_err(|_| AgentWrapperError::Backend {
                message: format!("codex exceeded timeout of {timeout:?}"),
            })?;
    }

    run_codex_inner(config, request, policy, tx).await
}

async fn run_codex_inner(
    config: CodexBackendConfig,
    request: AgentWrapperRunRequest,
    policy: CodexExecPolicy,
    tx: mpsc::Sender<AgentWrapperEvent>,
) -> Result<AgentWrapperCompletion, AgentWrapperError> {
    fn map_approval_policy(policy: &CodexApprovalPolicy) -> codex::ApprovalPolicy {
        match policy {
            CodexApprovalPolicy::Untrusted => codex::ApprovalPolicy::Untrusted,
            CodexApprovalPolicy::OnFailure => codex::ApprovalPolicy::OnFailure,
            CodexApprovalPolicy::OnRequest => codex::ApprovalPolicy::OnRequest,
            CodexApprovalPolicy::Never => codex::ApprovalPolicy::Never,
        }
    }

    fn map_sandbox_mode(mode: &CodexSandboxMode) -> codex::SandboxMode {
        match mode {
            CodexSandboxMode::ReadOnly => codex::SandboxMode::ReadOnly,
            CodexSandboxMode::WorkspaceWrite => codex::SandboxMode::WorkspaceWrite,
            CodexSandboxMode::DangerFullAccess => codex::SandboxMode::DangerFullAccess,
        }
    }

    let mut builder = codex::CodexClient::builder()
        .json(true)
        .mirror_stdout(false)
        .quiet(true)
        .color_mode(codex::ColorMode::Never)
        .sandbox_mode(map_sandbox_mode(&policy.sandbox_mode));

    if policy.non_interactive {
        builder = builder.approval_policy(codex::ApprovalPolicy::Never);
    } else if let Some(value) = policy.approval_policy.as_ref() {
        builder = builder.approval_policy(map_approval_policy(value));
    }

    if let Some(binary) = config.binary.as_ref() {
        builder = builder.binary(binary.clone());
    }

    if let Some(codex_home) = config.codex_home.as_ref() {
        builder = builder.codex_home(codex_home.clone());
    }

    let working_dir = request
        .working_dir
        .clone()
        .or_else(|| config.default_working_dir.clone())
        .or_else(|| std::env::current_dir().ok());
    let working_dir = working_dir.ok_or_else(|| AgentWrapperError::Backend {
        message: "failed to resolve working directory".to_string(),
    })?;
    builder = builder.working_dir(working_dir);

    let timeout = request
        .timeout
        .or(config.default_timeout)
        .unwrap_or(Duration::ZERO);
    builder = builder.timeout(timeout);

    let client = builder.build();

    let mut env_overrides = BTreeMap::new();
    env_overrides.extend(config.env);
    env_overrides.extend(request.env);

    let handle = match client
        .stream_exec_with_env_overrides(
            ExecStreamRequest {
                prompt: request.prompt,
                idle_timeout: None,
                output_last_message: None,
                output_schema: None,
                json_event_log: None,
            },
            &env_overrides,
        )
        .await
    {
        Ok(handle) => handle,
        Err(err) => {
            for bounded in
                crate::bounds::enforce_event_bounds(error_event(redacted_exec_error(&err)))
            {
                if tx.send(bounded).await.is_err() {
                    break;
                }
            }
            drop(tx);
            return Err(AgentWrapperError::Backend {
                message: redacted_exec_error(&err),
            });
        }
    };

    let mut events = handle.events;
    let completion = handle.completion;

    // If the caller drops the universal events stream, we MUST keep draining the backend stream so
    // the underlying process isn't accidentally cancelled (and so we avoid deadlocks on bounded
    // channels). We simply stop forwarding once the receiver is gone.
    let mut forward = true;
    while let Some(outcome) = events.next().await {
        if !forward {
            continue;
        }

        let mapped_events = match outcome {
            Ok(event) => vec![map_thread_event(&event)],
            Err(err) => vec![error_event(redacted_exec_error(&err))],
        };

        for event in mapped_events {
            for bounded in crate::bounds::enforce_event_bounds(event) {
                if tx.send(bounded).await.is_err() {
                    forward = false;
                    break;
                }
            }
            if !forward {
                break;
            }
        }
    }

    let completion = match completion.await {
        Ok(completion) => completion,
        Err(ExecStreamError::Codex(CodexError::NonZeroExit { status, .. })) => {
            for bounded in crate::bounds::enforce_event_bounds(error_event(format!(
                "codex exited non-zero: {status:?} (stderr redacted)"
            ))) {
                let _ = tx.send(bounded).await;
            }
            drop(tx);
            return Ok(crate::bounds::enforce_completion_bounds(
                AgentWrapperCompletion {
                    status,
                    final_text: None,
                    data: None,
                },
            ));
        }
        Err(err) => {
            for bounded in
                crate::bounds::enforce_event_bounds(error_event(redacted_exec_error(&err)))
            {
                let _ = tx.send(bounded).await;
            }
            drop(tx);
            return Err(AgentWrapperError::Backend {
                message: redacted_exec_error(&err),
            });
        }
    };

    drop(tx);

    Ok(crate::bounds::enforce_completion_bounds(
        AgentWrapperCompletion {
            status: completion.status,
            final_text: completion.last_message,
            data: None,
        },
    ))
}

#[cfg(test)]
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
        assert!(capabilities.contains("backend.codex.exec_stream"));
        assert!(capabilities.contains(EXT_NON_INTERACTIVE));
        assert!(capabilities.contains(EXT_CODEX_APPROVAL_POLICY));
        assert!(capabilities.contains(EXT_CODEX_SANDBOX_MODE));
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
        let mapped = map(r#"{"type":"turn.started","thread_id":"thread-1","turn_id":"turn-1"}"#);
        assert_eq!(mapped.kind, AgentWrapperEventKind::Status);
        assert_eq!(mapped.text, None);
    }

    #[test]
    fn turn_completed_maps_to_status() {
        let mapped = map(r#"{"type":"turn.completed","thread_id":"thread-1","turn_id":"turn-1"}"#);
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
