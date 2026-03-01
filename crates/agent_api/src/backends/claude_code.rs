use std::{
    collections::{BTreeMap, BTreeSet},
    future::Future,
    path::PathBuf,
    pin::Pin,
    sync::{Arc, Mutex},
    time::Duration,
};

use claude_code::{ClaudeOutputFormat, ClaudePrintRequest};
use futures_util::stream;
use futures_util::StreamExt;
use serde_json::Value;
use tokio::sync::oneshot;

use super::session_selectors::{
    parse_session_resume_v1, validate_resume_fork_mutual_exclusion, SessionSelectorV1,
    EXT_SESSION_RESUME_V1,
};

use crate::{
    backend_harness::{
        BackendDefaults, BackendHarnessAdapter, BackendHarnessErrorPhase, BackendSpawn,
        NormalizedRequest,
    },
    AgentWrapperBackend, AgentWrapperCapabilities, AgentWrapperCompletion, AgentWrapperError,
    AgentWrapperEvent, AgentWrapperEventKind, AgentWrapperKind, AgentWrapperRunControl,
    AgentWrapperRunHandle, AgentWrapperRunRequest,
};

impl super::termination::TerminationHandle for claude_code::ClaudeTerminationHandle {
    fn request_termination(&self) {
        claude_code::ClaudeTerminationHandle::request_termination(self);
    }
}

const AGENT_KIND: &str = "claude_code";
const CHANNEL_ASSISTANT: &str = "assistant";
const CHANNEL_TOOL: &str = "tool";

const EXT_NON_INTERACTIVE: &str = "agent_api.exec.non_interactive";

const CAP_TOOLS_STRUCTURED_V1: &str = "agent_api.tools.structured.v1";
const CAP_TOOLS_RESULTS_V1: &str = "agent_api.tools.results.v1";
const CAP_ARTIFACTS_FINAL_TEXT_V1: &str = "agent_api.artifacts.final_text.v1";
const CAP_SESSION_HANDLE_V1: &str = "agent_api.session.handle.v1";

const SESSION_HANDLE_ID_BOUND_BYTES: usize = 1024;
const SESSION_HANDLE_OVERSIZE_WARNING: &str = "session handle omitted: id exceeds 1024 bytes";

fn parse_bool(value: &Value, key: &str) -> Result<bool, AgentWrapperError> {
    value
        .as_bool()
        .ok_or_else(|| AgentWrapperError::InvalidRequest {
            message: format!("{key} must be a boolean"),
        })
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

#[derive(Clone, Debug)]
struct ClaudeHarnessAdapter {
    config: ClaudeCodeBackendConfig,
    termination: Option<
        std::sync::Arc<super::termination::TerminationState<claude_code::ClaudeTerminationHandle>>,
    >,
    handle_state: Arc<Mutex<ClaudeHandleFacetState>>,
}

#[derive(Clone, Debug)]
struct ClaudeExecPolicy {
    non_interactive: bool,
    resume: Option<SessionSelectorV1>,
}

#[derive(Clone, Debug)]
struct ClaudeBackendCompletion {
    status: std::process::ExitStatus,
    final_text: Option<String>,
    selection_failure_message: Option<String>,
}

#[derive(Debug, Default)]
struct ClaudeStreamState {
    last_assistant_text: Option<String>,
    saw_assistant_message: bool,
    saw_stream_error: bool,
}

#[derive(Debug)]
enum ClaudeBackendEvent {
    Stream(claude_code::ClaudeStreamJsonEvent),
    TerminalError { message: String },
}

#[derive(Debug, Default)]
struct ClaudeHandleFacetState {
    session_id: Option<String>,
    handle_facet_emitted: bool,
    oversize_warning_emitted: bool,
}

#[derive(Debug)]
enum ClaudeBackendError {
    Spawn(claude_code::ClaudeCodeError),
    StreamParse(claude_code::ClaudeStreamJsonParseError),
    Completion(claude_code::ClaudeCodeError),
}

impl BackendHarnessAdapter for ClaudeHarnessAdapter {
    fn kind(&self) -> AgentWrapperKind {
        AgentWrapperKind(AGENT_KIND.to_string())
    }

    fn supported_extension_keys(&self) -> &'static [&'static str] {
        const SUPPORTED: [&str; 2] = [EXT_NON_INTERACTIVE, EXT_SESSION_RESUME_V1];
        &SUPPORTED
    }

    type Policy = ClaudeExecPolicy;

    fn validate_and_extract_policy(
        &self,
        request: &AgentWrapperRunRequest,
    ) -> Result<Self::Policy, AgentWrapperError> {
        let non_interactive = request
            .extensions
            .get(EXT_NON_INTERACTIVE)
            .map(|value| parse_bool(value, EXT_NON_INTERACTIVE))
            .transpose()?
            .unwrap_or(true);

        let resume = request
            .extensions
            .get(EXT_SESSION_RESUME_V1)
            .map(parse_session_resume_v1)
            .transpose()?;

        validate_resume_fork_mutual_exclusion(&request.extensions)?;

        Ok(ClaudeExecPolicy {
            non_interactive,
            resume,
        })
    }

    type BackendEvent = ClaudeBackendEvent;
    type BackendCompletion = ClaudeBackendCompletion;
    type BackendError = ClaudeBackendError;

    fn spawn(
        &self,
        req: NormalizedRequest<Self::Policy>,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<
                        BackendSpawn<
                            Self::BackendEvent,
                            Self::BackendCompletion,
                            Self::BackendError,
                        >,
                        Self::BackendError,
                    >,
                > + Send
                + 'static,
        >,
    > {
        let config = self.config.clone();
        let termination = self.termination.clone();
        Box::pin(async move {
            let mut builder = claude_code::ClaudeClient::builder();
            if let Some(binary) = config.binary.as_ref() {
                builder = builder.binary(binary.clone());
            }

            let working_dir = req
                .working_dir
                .clone()
                .or_else(|| config.default_working_dir.clone());
            if let Some(dir) = working_dir {
                builder = builder.working_dir(dir);
            }

            let timeout = match req.effective_timeout {
                Some(t) if t == Duration::ZERO => None,
                other => other,
            };
            builder = builder.timeout(timeout);

            for (k, v) in req.env.iter() {
                builder = builder.env(k.clone(), v.clone());
            }

            let client = builder.build();

            let mut print_req = ClaudePrintRequest::new(req.prompt)
                .output_format(ClaudeOutputFormat::StreamJson)
                .include_partial_messages(true);
            if req.policy.non_interactive {
                print_req = print_req.permission_mode("bypassPermissions");
            }

            if let Some(resume) = req.policy.resume.as_ref() {
                match resume {
                    SessionSelectorV1::Last => {
                        print_req = print_req.continue_session(true);
                    }
                    SessionSelectorV1::Id { id } => {
                        print_req = print_req.resume_value(id.clone());
                    }
                }
            }

            let handle = client
                .print_stream_json_control(print_req)
                .await
                .map_err(ClaudeBackendError::Spawn)?;

            if let Some(state) = termination.as_ref() {
                state.set_handle(handle.termination.clone());
            }

            let resume_selector = req.policy.resume.clone();
            let stream_state: Arc<Mutex<ClaudeStreamState>> =
                Arc::new(Mutex::new(ClaudeStreamState::default()));
            let (events_done_tx, events_done_rx) = oneshot::channel::<()>();

            let (tail_tx, tail_rx) = if resume_selector.is_some() {
                let (tx, rx) = oneshot::channel::<Option<String>>();
                (Some(tx), Some(rx))
            } else {
                (None, None)
            };

            let events = Box::pin(stream::unfold(
                (
                    handle.events,
                    stream_state.clone(),
                    Some(events_done_tx),
                    tail_rx,
                    resume_selector.clone(),
                    false,
                ),
                |(
                    mut events,
                    stream_state,
                    mut events_done_tx,
                    mut tail_rx,
                    resume_selector,
                    tail_emitted,
                )| async move {
                    loop {
                        match events.next().await {
                            Some(Ok(ev)) => {
                                if let claude_code::ClaudeStreamJsonEvent::AssistantMessage {
                                    raw,
                                    ..
                                } = &ev
                                {
                                    if let Ok(mut state) = stream_state.lock() {
                                        state.saw_assistant_message = true;
                                        if let Some(text) =
                                            extract_assistant_message_final_text(raw)
                                        {
                                            state.last_assistant_text = Some(text);
                                        }
                                    }
                                }

                                if resume_selector.is_some()
                                    && matches!(
                                        ev,
                                        claude_code::ClaudeStreamJsonEvent::ResultError { .. }
                                    )
                                {
                                    continue;
                                }

                                return Some((
                                    Ok(ClaudeBackendEvent::Stream(ev)),
                                    (
                                        events,
                                        stream_state,
                                        events_done_tx,
                                        tail_rx,
                                        resume_selector,
                                        tail_emitted,
                                    ),
                                ));
                            }
                            Some(Err(err)) => {
                                if let Ok(mut state) = stream_state.lock() {
                                    state.saw_stream_error = true;
                                }

                                return Some((
                                    Err(ClaudeBackendError::StreamParse(err)),
                                    (
                                        events,
                                        stream_state,
                                        events_done_tx,
                                        tail_rx,
                                        resume_selector,
                                        tail_emitted,
                                    ),
                                ));
                            }
                            None => {
                                if let Some(tx) = events_done_tx.take() {
                                    let _ = tx.send(());
                                }

                                if tail_emitted {
                                    return None;
                                }

                                let Some(rx) = tail_rx.take() else {
                                    return None;
                                };

                                let message = rx.await.ok().flatten();
                                let Some(message) = message else {
                                    return None;
                                };

                                return Some((
                                    Ok(ClaudeBackendEvent::TerminalError { message }),
                                    (
                                        events,
                                        stream_state,
                                        events_done_tx,
                                        tail_rx,
                                        resume_selector,
                                        true,
                                    ),
                                ));
                            }
                        }
                    }
                },
            ));

            let completion = Box::pin(async move {
                let _ = events_done_rx.await;

                let status = handle
                    .completion
                    .await
                    .map_err(ClaudeBackendError::Completion)?;

                let final_text = stream_state
                    .lock()
                    .ok()
                    .and_then(|guard| guard.last_assistant_text.clone());

                let selection_failure_message = match resume_selector {
                    Some(SessionSelectorV1::Last) => {
                        let state = stream_state.lock().ok();
                        if !status.success()
                            && state
                                .as_ref()
                                .is_some_and(|state| !state.saw_assistant_message)
                            && state.as_ref().is_some_and(|state| !state.saw_stream_error)
                        {
                            Some("no session found".to_string())
                        } else {
                            None
                        }
                    }
                    Some(SessionSelectorV1::Id { .. }) => {
                        let state = stream_state.lock().ok();
                        if !status.success()
                            && state
                                .as_ref()
                                .is_some_and(|state| !state.saw_assistant_message)
                            && state.as_ref().is_some_and(|state| !state.saw_stream_error)
                        {
                            Some("session not found".to_string())
                        } else {
                            None
                        }
                    }
                    None => None,
                };

                if let Some(tx) = tail_tx {
                    let _ = tx.send(selection_failure_message.clone());
                }

                Ok(ClaudeBackendCompletion {
                    status,
                    final_text,
                    selection_failure_message,
                })
            });

            Ok(BackendSpawn { events, completion })
        })
    }

    fn map_event(&self, event: Self::BackendEvent) -> Vec<AgentWrapperEvent> {
        let event = match event {
            ClaudeBackendEvent::Stream(event) => event,
            ClaudeBackendEvent::TerminalError { message } => {
                return vec![error_event(message)];
            }
        };

        let mut emit_oversize_warning = false;

        if let Ok(mut state) = self.handle_state.lock() {
            if state.session_id.is_none() {
                if let Some(session_id) = event.session_id() {
                    let session_id = session_id.trim();
                    if !session_id.is_empty() {
                        if session_id.len() <= SESSION_HANDLE_ID_BOUND_BYTES {
                            state.session_id = Some(session_id.to_string());
                        } else if !state.oversize_warning_emitted {
                            state.oversize_warning_emitted = true;
                            emit_oversize_warning = true;
                        }
                    }
                }
            }
        }

        let mut mapped = map_stream_json_event(event);

        let emit_handle_facet: Option<String> =
            self.handle_state.lock().ok().and_then(|mut state| {
                if state.handle_facet_emitted {
                    return None;
                }
                let session_id = state.session_id.clone()?;
                state.handle_facet_emitted = true;
                Some(session_id)
            });

        if let Some(session_id) = emit_handle_facet.as_deref() {
            let mut attached = false;
            for event in mapped.iter_mut() {
                if event.kind == AgentWrapperEventKind::Status && event.data.is_none() {
                    event.data = Some(session_handle_facet(session_id));
                    attached = true;
                    break;
                }
            }

            if !attached {
                let mut event = status_event(None);
                event.data = Some(session_handle_facet(session_id));
                mapped.push(event);
            }
        }

        if emit_oversize_warning {
            mapped.push(status_event(Some(
                SESSION_HANDLE_OVERSIZE_WARNING.to_string(),
            )));
        }

        mapped
    }

    fn map_completion(
        &self,
        completion: Self::BackendCompletion,
    ) -> Result<AgentWrapperCompletion, AgentWrapperError> {
        if let Some(message) = completion.selection_failure_message {
            return Err(AgentWrapperError::Backend { message });
        }

        let handle_facet = self
            .handle_state
            .lock()
            .ok()
            .and_then(|state| state.session_id.clone())
            .map(|session_id| session_handle_facet(&session_id));

        Ok(AgentWrapperCompletion {
            status: completion.status,
            final_text: crate::bounds::enforce_final_text_bound(completion.final_text),
            data: handle_facet,
        })
    }

    fn redact_error(&self, phase: BackendHarnessErrorPhase, err: &Self::BackendError) -> String {
        match (phase, err) {
            (BackendHarnessErrorPhase::Stream, ClaudeBackendError::StreamParse(err)) => {
                err.message.clone()
            }
            (_, ClaudeBackendError::Spawn(err) | ClaudeBackendError::Completion(err)) => {
                format!("claude_code error: {err}")
            }
            (_, ClaudeBackendError::StreamParse(err)) => err.message.clone(),
        }
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
        ids.insert("agent_api.events.live".to_string());
        ids.insert(crate::CAPABILITY_CONTROL_CANCEL_V1.to_string());
        ids.insert(CAP_TOOLS_STRUCTURED_V1.to_string());
        ids.insert(CAP_TOOLS_RESULTS_V1.to_string());
        ids.insert(CAP_ARTIFACTS_FINAL_TEXT_V1.to_string());
        ids.insert(CAP_SESSION_HANDLE_V1.to_string());
        ids.insert("backend.claude_code.print_stream_json".to_string());
        ids.insert(EXT_NON_INTERACTIVE.to_string());
        ids.insert(EXT_SESSION_RESUME_V1.to_string());
        AgentWrapperCapabilities { ids }
    }

    fn run(
        &self,
        request: AgentWrapperRunRequest,
    ) -> Pin<Box<dyn Future<Output = Result<AgentWrapperRunHandle, AgentWrapperError>> + Send + '_>>
    {
        let config = self.config.clone();
        Box::pin(async move {
            let adapter = Arc::new(ClaudeHarnessAdapter {
                config: config.clone(),
                termination: None,
                handle_state: Arc::new(Mutex::new(ClaudeHandleFacetState::default())),
            });
            let defaults = BackendDefaults {
                env: config.env,
                default_timeout: config.default_timeout,
            };

            crate::backend_harness::run_harnessed_backend(adapter, defaults, request)
        })
    }

    fn run_control(
        &self,
        request: AgentWrapperRunRequest,
    ) -> Pin<Box<dyn Future<Output = Result<AgentWrapperRunControl, AgentWrapperError>> + Send + '_>>
    {
        if !self
            .capabilities()
            .contains(crate::CAPABILITY_CONTROL_CANCEL_V1)
        {
            let agent_kind = self.kind().as_str().to_string();
            return Box::pin(async move {
                Err(AgentWrapperError::UnsupportedCapability {
                    agent_kind,
                    capability: crate::CAPABILITY_CONTROL_CANCEL_V1.to_string(),
                })
            });
        }

        let config = self.config.clone();
        Box::pin(async move {
            let termination_state = Arc::new(super::termination::TerminationState::new());
            let request_termination: Option<Arc<dyn Fn() + Send + Sync + 'static>> = Some({
                let termination_state = Arc::clone(&termination_state);
                Arc::new(move || termination_state.request())
            });

            let adapter = Arc::new(ClaudeHarnessAdapter {
                config: config.clone(),
                termination: Some(termination_state),
                handle_state: Arc::new(Mutex::new(ClaudeHandleFacetState::default())),
            });
            let defaults = BackendDefaults {
                env: config.env,
                default_timeout: config.default_timeout,
            };

            crate::backend_harness::run_harnessed_backend_control(
                adapter,
                defaults,
                request,
                request_termination,
            )
        })
    }
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

fn extract_assistant_message_final_text(raw: &serde_json::Value) -> Option<String> {
    let blocks = raw
        .get("message")
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_array())?;

    let mut texts = Vec::new();
    for block in blocks {
        let Some(obj) = block.as_object() else {
            continue;
        };
        if obj.get("type").and_then(|v| v.as_str()) != Some("text") {
            continue;
        }
        let Some(text) = obj.get("text").and_then(|v| v.as_str()) else {
            continue;
        };
        texts.push(text);
    }

    if texts.is_empty() {
        None
    } else {
        Some(texts.join("\n"))
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
            "tool_use" => {
                let tool_name = obj
                    .get("name")
                    .and_then(|v| v.as_str())
                    .map(|v| v.to_string());
                let tool_use_id = obj
                    .get("id")
                    .and_then(|v| v.as_str())
                    .map(|v| v.to_string());
                out.push(tool_call_start_event(tool_name, tool_use_id));
            }
            "tool_result" => {
                let tool_use_id = obj
                    .get("tool_use_id")
                    .and_then(|v| v.as_str())
                    .map(|v| v.to_string());
                out.push(tool_result_complete_event(tool_use_id));
            }
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
            let Some(content_block) = obj.get("content_block").and_then(|v| v.as_object()) else {
                return vec![unknown_event()];
            };
            let block_type = content_block
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            match block_type {
                "tool_use" => {
                    let tool_name = content_block
                        .get("name")
                        .and_then(|v| v.as_str())
                        .map(|v| v.to_string());
                    let tool_use_id = content_block
                        .get("id")
                        .and_then(|v| v.as_str())
                        .map(|v| v.to_string());
                    vec![tool_call_start_event(tool_name, tool_use_id)]
                }
                "tool_result" => {
                    let tool_use_id = content_block
                        .get("tool_use_id")
                        .and_then(|v| v.as_str())
                        .map(|v| v.to_string());
                    vec![tool_result_complete_event(tool_use_id)]
                }
                _ => vec![unknown_event()],
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
                "input_json_delta" => vec![tool_call_delta_event()],
                _ => vec![unknown_event()],
            }
        }
        _ => vec![unknown_event()],
    }
}

fn tool_call_start_event(
    tool_name: Option<String>,
    tool_use_id: Option<String>,
) -> AgentWrapperEvent {
    AgentWrapperEvent {
        agent_kind: AgentWrapperKind(AGENT_KIND.to_string()),
        kind: AgentWrapperEventKind::ToolCall,
        channel: Some(CHANNEL_TOOL.to_string()),
        text: None,
        message: None,
        data: Some(tool_facet(
            "tool_use",
            "start",
            "running",
            tool_name,
            tool_use_id,
        )),
    }
}

fn tool_call_delta_event() -> AgentWrapperEvent {
    AgentWrapperEvent {
        agent_kind: AgentWrapperKind(AGENT_KIND.to_string()),
        kind: AgentWrapperEventKind::ToolCall,
        channel: Some(CHANNEL_TOOL.to_string()),
        text: None,
        message: None,
        data: Some(tool_facet("tool_use", "delta", "running", None, None)),
    }
}

fn tool_result_complete_event(tool_use_id: Option<String>) -> AgentWrapperEvent {
    AgentWrapperEvent {
        agent_kind: AgentWrapperKind(AGENT_KIND.to_string()),
        kind: AgentWrapperEventKind::ToolResult,
        channel: Some(CHANNEL_TOOL.to_string()),
        text: None,
        message: None,
        data: Some(tool_facet(
            "tool_result",
            "complete",
            "completed",
            None,
            tool_use_id,
        )),
    }
}

fn tool_facet(
    kind: &'static str,
    phase: &'static str,
    status: &'static str,
    tool_name: Option<String>,
    tool_use_id: Option<String>,
) -> serde_json::Value {
    serde_json::json!({
        "schema": CAP_TOOLS_STRUCTURED_V1,
        "tool": {
            "backend_item_id": null,
            "thread_id": null,
            "turn_id": null,
            "kind": kind,
            "phase": phase,
            "status": status,
            "exit_code": null,
            "bytes": { "stdout": 0, "stderr": 0, "diff": 0, "result": 0 },
            "tool_name": tool_name,
            "tool_use_id": tool_use_id,
        },
    })
}

fn session_handle_facet(session_id: &str) -> serde_json::Value {
    serde_json::json!({
        "schema": CAP_SESSION_HANDLE_V1,
        "session": { "id": session_id },
    })
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

#[cfg(test)]
#[path = "claude_code/tests.rs"]
mod tests;
