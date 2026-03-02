use std::{
    collections::BTreeMap,
    ffi::OsString,
    path::PathBuf,
    process::ExitStatus,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use serde_json::Value;
use tokio::sync::{mpsc, oneshot};

use crate::{
    backend_harness::{BackendSpawn, DynBackendCompletionFuture, DynBackendEventStream},
    AgentWrapperError, AgentWrapperEvent, AgentWrapperEventKind, AgentWrapperKind,
    AgentWrapperRunRequest,
};

use super::{
    mapping::{error_event, status_event},
    CodexApprovalPolicy, CodexBackendCompletion, CodexBackendConfig, CodexBackendError,
    CodexBackendEvent, CodexHandleFacetState, CodexSandboxMode, CodexTerminationHandle,
    PINNED_APPROVAL_REQUIRED,
};

use super::super::{
    session_selectors::{parse_session_fork_v1, SessionSelectorV1, EXT_SESSION_FORK_V1},
    termination::{TerminationHandle, TerminationState},
};

#[derive(Clone, Debug)]
pub(super) struct ForkFlowRequest {
    pub(super) selector: SessionSelectorV1,
    pub(super) prompt: String,
    pub(super) working_dir: Option<PathBuf>,
    pub(super) env: BTreeMap<String, String>,
    pub(super) config: CodexBackendConfig,
    pub(super) run_start_cwd: Option<PathBuf>,
    pub(super) termination: Option<Arc<TerminationState<CodexTerminationHandle>>>,
    pub(super) non_interactive: bool,
    pub(super) approval_policy: Option<CodexApprovalPolicy>,
    pub(super) sandbox_mode: CodexSandboxMode,
    pub(super) handle_state: Arc<std::sync::Mutex<CodexHandleFacetState>>,
}

pub(super) fn extract_fork_selector_v1(
    request: &AgentWrapperRunRequest,
) -> Result<Option<SessionSelectorV1>, AgentWrapperError> {
    request
        .extensions
        .get(EXT_SESSION_FORK_V1)
        .map(parse_session_fork_v1)
        .transpose()
}

#[derive(Clone)]
pub(super) struct AppServerTurnCancelHandle {
    server: Arc<codex::mcp::CodexAppServer>,
    request_id: codex::mcp::RequestId,
}

impl TerminationHandle for AppServerTurnCancelHandle {
    fn request_termination(&self) {
        let _ = self.server.cancel(self.request_id);
    }
}

fn synthetic_success_exit_status() -> ExitStatus {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        ExitStatus::from_raw(0)
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::ExitStatusExt;
        ExitStatus::from_raw(0)
    }
}

fn map_sandbox_mode(mode: &CodexSandboxMode) -> &'static str {
    match mode {
        CodexSandboxMode::ReadOnly => "read-only",
        CodexSandboxMode::WorkspaceWrite => "workspace-write",
        CodexSandboxMode::DangerFullAccess => "danger-full-access",
    }
}

fn map_approval_policy(policy: &CodexApprovalPolicy) -> &'static str {
    match policy {
        CodexApprovalPolicy::Untrusted => "untrusted",
        CodexApprovalPolicy::OnFailure => "on-failure",
        CodexApprovalPolicy::OnRequest => "on-request",
        CodexApprovalPolicy::Never => "never",
    }
}

fn effective_approval_policy(
    non_interactive: bool,
    approval_policy: Option<&CodexApprovalPolicy>,
) -> Option<String> {
    if non_interactive {
        return Some("never".to_string());
    }
    approval_policy.map(|policy| map_approval_policy(policy).to_string())
}

fn install_thread_id(handle_state: &Arc<std::sync::Mutex<CodexHandleFacetState>>, thread_id: &str) {
    if thread_id.trim().is_empty() {
        return;
    }
    let Ok(mut state) = handle_state.lock() else {
        return;
    };
    if state.thread_id.is_none() {
        let len = thread_id.len();
        if len <= super::SESSION_HANDLE_ID_BOUND_BYTES {
            state.thread_id = Some(thread_id.to_string());
        } else if !state.oversize_warning_emitted {
            state.oversize_warning_emitted = true;
            state.oversize_warning_len_bytes = Some(len);
        }
    }
}

fn tools_facet_data(
    backend_item_id: Option<&str>,
    thread_id: Option<&str>,
    turn_id: Option<&str>,
    kind: &str,
    phase: &str,
    status: &str,
) -> Value {
    serde_json::json!({
        "schema": super::TOOLS_FACET_SCHEMA,
        "tool": {
            "backend_item_id": backend_item_id,
            "thread_id": thread_id,
            "turn_id": turn_id,
            "kind": kind,
            "phase": phase,
            "status": status,
            "exit_code": null,
            "bytes": { "stdout": 0, "stderr": 0, "diff": 0, "result": 0 },
            "tool_name": null,
            "tool_use_id": null
        }
    })
}

pub(super) fn map_app_server_notification(
    method: &str,
    params: &Value,
) -> Option<AgentWrapperEvent> {
    match method {
        "agentMessage/delta" => {
            let delta = params.as_str()?;
            Some(AgentWrapperEvent {
                agent_kind: AgentWrapperKind("codex".to_string()),
                kind: AgentWrapperEventKind::TextOutput,
                channel: Some("assistant".to_string()),
                text: Some(delta.to_string()),
                message: None,
                data: None,
            })
        }
        "reasoning/text/delta" => {
            let text = params
                .get("content")
                .and_then(|content| content.get("text"))
                .and_then(Value::as_str)
                .or_else(|| params.get("text").and_then(Value::as_str))
                .or_else(|| params.as_str())?;
            Some(AgentWrapperEvent {
                agent_kind: AgentWrapperKind("codex".to_string()),
                kind: AgentWrapperEventKind::TextOutput,
                channel: Some("assistant".to_string()),
                text: Some(text.to_string()),
                message: None,
                data: None,
            })
        }
        "turn/started" | "turn/completed" => Some(status_event(None)),
        "item/started" => {
            let item_id = params.get("item_id").and_then(Value::as_str);
            let thread_id = params.get("thread_id").and_then(Value::as_str);
            let turn_id = params.get("turn_id").and_then(Value::as_str);
            let kind = params
                .get("item_type")
                .and_then(Value::as_str)
                .unwrap_or("unknown");

            Some(AgentWrapperEvent {
                agent_kind: AgentWrapperKind("codex".to_string()),
                kind: AgentWrapperEventKind::ToolCall,
                channel: Some("tool".to_string()),
                text: None,
                message: None,
                data: Some(tools_facet_data(
                    item_id, thread_id, turn_id, kind, "start", "running",
                )),
            })
        }
        "item/completed" => {
            let item_id = params.get("item_id").and_then(Value::as_str);
            let thread_id = params.get("thread_id").and_then(Value::as_str);
            let turn_id = params.get("turn_id").and_then(Value::as_str);
            let kind = params
                .get("item_type")
                .and_then(Value::as_str)
                .unwrap_or("unknown");

            Some(AgentWrapperEvent {
                agent_kind: AgentWrapperKind("codex".to_string()),
                kind: AgentWrapperEventKind::ToolResult,
                channel: Some("tool".to_string()),
                text: None,
                message: None,
                data: Some(tools_facet_data(
                    item_id,
                    thread_id,
                    turn_id,
                    kind,
                    "complete",
                    "completed",
                )),
            })
        }
        "error" => {
            let message = params
                .get("error")
                .and_then(|err| err.get("message"))
                .and_then(Value::as_str)
                .or_else(|| params.get("message").and_then(Value::as_str))
                .unwrap_or_default();
            if message.trim().is_empty() {
                return None;
            }
            Some(error_event(message.to_string()))
        }
        _ => None,
    }
}

pub(super) fn is_approval_request_notification(method: &str, params: &Value) -> bool {
    if method != "codex/event" {
        return false;
    }

    let payload = params.get("msg").unwrap_or(params);
    let payload = match payload.as_object() {
        Some(payload) => payload,
        None => return false,
    };

    let event_type = payload.get("type").and_then(Value::as_str);
    if !matches!(event_type, Some("approval_required" | "approval")) {
        return false;
    }

    let approval_id = payload
        .get("approval_id")
        .or_else(|| payload.get("id"))
        .and_then(Value::as_str);
    approval_id.is_some_and(|id| !id.trim().is_empty())
}

pub(super) async fn spawn_fork_v1_flow(
    req: ForkFlowRequest,
) -> Result<
    BackendSpawn<CodexBackendEvent, CodexBackendCompletion, CodexBackendError>,
    CodexBackendError,
> {
    let ForkFlowRequest {
        selector,
        prompt,
        working_dir,
        env,
        config,
        run_start_cwd,
        termination,
        non_interactive,
        approval_policy,
        sandbox_mode,
        handle_state,
    } = req;

    let effective_cwd = working_dir
        .or_else(|| config.default_working_dir.clone())
        .or(run_start_cwd)
        .ok_or(CodexBackendError::WorkingDirectoryUnresolved)?;

    let server = codex::mcp::CodexAppServer::with_capabilities(
        codex::mcp::StdioServerConfig {
            binary: config
                .binary
                .clone()
                .unwrap_or_else(|| PathBuf::from("codex")),
            code_home: config.codex_home.clone(),
            current_dir: Some(effective_cwd.clone()),
            env: env
                .iter()
                .map(|(k, v)| (OsString::from(k), OsString::from(v)))
                .collect(),
            app_server_analytics_default_enabled: false,
            mirror_stdio: false,
            startup_timeout: Duration::from_secs(3),
        },
        codex::mcp::ClientInfo {
            name: "agent_api".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
        serde_json::json!({"experimentalApi": true}),
    )
    .await
    .map_err(CodexBackendError::AppServer)?;
    let server = Arc::new(server);

    let source_thread_id = match selector {
        SessionSelectorV1::Id { id } => id,
        SessionSelectorV1::Last => server
            .select_last_thread_id(effective_cwd.clone())
            .await
            .map_err(CodexBackendError::AppServer)?
            .ok_or(CodexBackendError::ForkSelectionEmpty)?,
    };

    let approval_policy = effective_approval_policy(non_interactive, approval_policy.as_ref());
    let sandbox = Some(map_sandbox_mode(&sandbox_mode).to_string());

    let forked = server
        .thread_fork(codex::mcp::ThreadForkParams {
            thread_id: source_thread_id,
            cwd: Some(effective_cwd.clone()),
            approval_policy: approval_policy.clone(),
            sandbox,
            persist_extended_history: None,
        })
        .await;

    let forked = match forked {
        Ok(forked) => forked,
        Err(codex::mcp::McpError::Rpc { message, .. }) if super::is_not_found_signal(&message) => {
            return Err(CodexBackendError::ForkSessionNotFound);
        }
        Err(err) => return Err(CodexBackendError::AppServer(err)),
    };

    install_thread_id(&handle_state, forked.thread.id.as_str());

    let turn = server
        .turn_start_v2(codex::mcp::TurnStartParamsV2 {
            thread_id: forked.thread.id.clone(),
            input: vec![codex::mcp::UserInputV2::text(prompt)],
            approval_policy,
            cwd: Some(effective_cwd),
        })
        .await
        .map_err(CodexBackendError::AppServer)?;

    if let Some(state) = termination.as_ref() {
        state.set_handle(CodexTerminationHandle::AppServerTurn(
            AppServerTurnCancelHandle {
                server: Arc::clone(&server),
                request_id: turn.request_id,
            },
        ));
    }

    let approval_required = Arc::new(AtomicBool::new(false));
    let (approval_signal_tx, mut approval_signal_rx) = oneshot::channel::<()>();

    let (event_tx, event_rx) = mpsc::unbounded_channel::<CodexBackendEvent>();
    let (completion_tx, completion_rx) =
        oneshot::channel::<Result<CodexBackendCompletion, CodexBackendError>>();

    let codex::mcp::AppCallHandle {
        request_id: turn_request_id,
        events: mut turn_events,
        response: turn_response,
    } = turn;

    let event_tx_for_notifications = event_tx.clone();
    let approval_required_for_notifications = Arc::clone(&approval_required);

    tokio::spawn(async move {
        let mut approval_signal_tx = Some(approval_signal_tx);
        while let Some(notification) = turn_events.recv().await {
            if let codex::mcp::AppNotification::Raw { method, params } = &notification {
                if is_approval_request_notification(method, params)
                    && !approval_required_for_notifications.swap(true, Ordering::SeqCst)
                {
                    if let Some(tx) = approval_signal_tx.take() {
                        let _ = tx.send(());
                    }
                    // Stop forwarding so the `"approval required"` tail event is terminal.
                    return;
                }
            }

            let _ = event_tx_for_notifications
                .send(CodexBackendEvent::AppServerNotification(notification));
        }
    });

    tokio::spawn({
        let server = Arc::clone(&server);
        let approval_required = Arc::clone(&approval_required);
        async move {
            let mut cancel_sent = false;
            tokio::pin!(turn_response);

            loop {
                tokio::select! {
                    biased;
                    _ = &mut approval_signal_rx, if !cancel_sent => {
                        cancel_sent = true;
                        if approval_required.load(Ordering::SeqCst) {
                            let _ = server.cancel(turn_request_id);
                        }
                    }
                    response_outcome = &mut turn_response => {
                        let response_outcome = match response_outcome {
                            Ok(outcome) => outcome,
                            Err(_) => Err(codex::mcp::McpError::ChannelClosed),
                        };

                        let mut selection_failure_message = None;
                        if approval_required.load(Ordering::SeqCst) {
                            selection_failure_message = Some(PINNED_APPROVAL_REQUIRED.to_string());
                            let _ = event_tx.send(CodexBackendEvent::TerminalError {
                                message: PINNED_APPROVAL_REQUIRED.to_string(),
                            });
                        } else if let Err(err) = response_outcome {
                            let _ = completion_tx.send(Err(CodexBackendError::AppServer(err)));
                            let _ = server.shutdown().await;
                            return;
                        }

                        let _ = server.shutdown().await;
                        let _ = completion_tx.send(Ok(CodexBackendCompletion {
                            status: synthetic_success_exit_status(),
                            final_text: None,
                            selection_failure_message,
                        }));
                        return;
                    }
                }
            }
        }
    });

    let events: DynBackendEventStream<CodexBackendEvent, CodexBackendError> = Box::pin(
        futures_util::stream::unfold(event_rx, |mut rx| async move {
            rx.recv().await.map(|event| (Ok(event), rx))
        }),
    );

    let completion: DynBackendCompletionFuture<CodexBackendCompletion, CodexBackendError> =
        Box::pin(async move {
            completion_rx
                .await
                .unwrap_or(Err(CodexBackendError::CompletionTaskDropped))
        });

    Ok(BackendSpawn { events, completion })
}
