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
    parse_session_fork_v1, parse_session_resume_v1, validate_resume_fork_mutual_exclusion,
    SessionSelectorV1, EXT_SESSION_FORK_V1, EXT_SESSION_RESUME_V1,
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

fn is_session_not_found_signal(text: &str) -> bool {
    let text = text.to_ascii_lowercase();

    (text.contains("not found")
        && (text.contains("session") || text.contains("thread") || text.contains("conversation")))
        || text.contains("no session")
        || text.contains("unknown session")
        || text.contains("no thread")
        || text.contains("unknown thread")
        || text.contains("no conversation")
        || text.contains("unknown conversation")
}

fn json_contains_not_found_signal(raw: &serde_json::Value) -> bool {
    const MAX_DEPTH: usize = 6;
    const MAX_STRING_LEAVES: usize = 64;

    fn visit(raw: &serde_json::Value, depth: usize, strings_seen: &mut usize) -> bool {
        if depth > MAX_DEPTH || *strings_seen >= MAX_STRING_LEAVES {
            return false;
        }

        match raw {
            serde_json::Value::String(s) => {
                *strings_seen += 1;
                is_session_not_found_signal(s)
            }
            serde_json::Value::Array(arr) => arr
                .iter()
                .any(|child| visit(child, depth + 1, strings_seen)),
            serde_json::Value::Object(obj) => obj
                .values()
                .any(|child| visit(child, depth + 1, strings_seen)),
            _ => false,
        }
    }

    let mut strings_seen = 0usize;
    visit(raw, 0, &mut strings_seen)
}

fn generic_non_zero_exit_message(status: &std::process::ExitStatus) -> String {
    match status.code() {
        Some(code) => format!("claude_code exited non-zero: code={code} (output redacted)"),
        None => "claude_code exited non-zero (output redacted)".to_string(),
    }
}

#[path = "claude_code/mapping.rs"]
mod mapping;

use mapping::{
    error_event, extract_assistant_message_final_text, map_stream_json_event, session_handle_facet,
    status_event,
};

#[cfg(test)]
use mapping::{map_assistant_message, map_stream_event};

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
    fork: Option<SessionSelectorV1>,
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
    saw_not_found_signal: bool,
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
        const SUPPORTED: [&str; 3] = [
            EXT_NON_INTERACTIVE,
            EXT_SESSION_RESUME_V1,
            EXT_SESSION_FORK_V1,
        ];
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

        let fork = request
            .extensions
            .get(EXT_SESSION_FORK_V1)
            .map(parse_session_fork_v1)
            .transpose()?;

        validate_resume_fork_mutual_exclusion(&request.extensions)?;

        Ok(ClaudeExecPolicy {
            non_interactive,
            resume,
            fork,
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

            if let Some(fork) = req.policy.fork.as_ref() {
                print_req = print_req.fork_session(true);
                match fork {
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

            let selection_selector = req.policy.resume.clone().or(req.policy.fork.clone());
            let stream_state: Arc<Mutex<ClaudeStreamState>> =
                Arc::new(Mutex::new(ClaudeStreamState::default()));
            let (events_done_tx, events_done_rx) = oneshot::channel::<()>();

            let (tail_tx, tail_rx) = if selection_selector.is_some() {
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
                    selection_selector.clone(),
                    false,
                ),
                |(
                    mut events,
                    stream_state,
                    mut events_done_tx,
                    mut tail_rx,
                    selection_selector,
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

                                if selection_selector.is_some()
                                    && matches!(
                                        ev,
                                        claude_code::ClaudeStreamJsonEvent::ResultError { .. }
                                    )
                                {
                                    if let claude_code::ClaudeStreamJsonEvent::ResultError {
                                        raw,
                                        ..
                                    } = &ev
                                    {
                                        if json_contains_not_found_signal(raw) {
                                            if let Ok(mut state) = stream_state.lock() {
                                                state.saw_not_found_signal = true;
                                            }
                                        }
                                    }
                                    continue;
                                }

                                return Some((
                                    Ok(ClaudeBackendEvent::Stream(ev)),
                                    (
                                        events,
                                        stream_state,
                                        events_done_tx,
                                        tail_rx,
                                        selection_selector,
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
                                        selection_selector,
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

                                let rx = tail_rx.take()?;
                                let message = rx.await.ok().flatten()?;

                                return Some((
                                    Ok(ClaudeBackendEvent::TerminalError { message }),
                                    (
                                        events,
                                        stream_state,
                                        events_done_tx,
                                        tail_rx,
                                        selection_selector,
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

                let (final_text, saw_stream_error, saw_not_found_signal) = stream_state
                    .lock()
                    .map(|guard| {
                        (
                            guard.last_assistant_text.clone(),
                            guard.saw_stream_error,
                            guard.saw_not_found_signal,
                        )
                    })
                    .unwrap_or((None, true, false));

                let selection_failure_message = if selection_selector.is_some()
                    && !status.success()
                    && !saw_stream_error
                    && saw_not_found_signal
                {
                    match selection_selector {
                        Some(SessionSelectorV1::Last) => Some("no session found".to_string()),
                        Some(SessionSelectorV1::Id { .. }) => Some("session not found".to_string()),
                        None => None,
                    }
                } else {
                    None
                };

                let terminal_error_event_message =
                    if selection_selector.is_some() && !status.success() && !saw_stream_error {
                        Some(
                            selection_failure_message
                                .clone()
                                .unwrap_or_else(|| generic_non_zero_exit_message(&status)),
                        )
                    } else {
                        None
                    };

                if let Some(tx) = tail_tx {
                    let _ = tx.send(terminal_error_event_message.clone());
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
        ids.insert(EXT_SESSION_FORK_V1.to_string());
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

#[cfg(test)]
#[path = "claude_code/tests.rs"]
mod tests;
