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
use tokio::sync::{oneshot, OnceCell};

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
const EXT_EXTERNAL_SANDBOX_V1: &str = "agent_api.exec.external_sandbox.v1";
const CLAUDE_EXEC_POLICY_PREFIX: &str = "backend.claude_code.exec.";

const SUPPORTED_EXTENSION_KEYS_DEFAULT: &[&str] = &[
    EXT_NON_INTERACTIVE,
    EXT_SESSION_RESUME_V1,
    EXT_SESSION_FORK_V1,
];

const SUPPORTED_EXTENSION_KEYS_EXTERNAL_SANDBOX_OPT_IN: &[&str] = &[
    EXT_NON_INTERACTIVE,
    EXT_SESSION_RESUME_V1,
    EXT_SESSION_FORK_V1,
    EXT_EXTERNAL_SANDBOX_V1,
];

const CAP_TOOLS_STRUCTURED_V1: &str = "agent_api.tools.structured.v1";
const CAP_TOOLS_RESULTS_V1: &str = "agent_api.tools.results.v1";
const CAP_ARTIFACTS_FINAL_TEXT_V1: &str = "agent_api.artifacts.final_text.v1";
const CAP_SESSION_HANDLE_V1: &str = "agent_api.session.handle.v1";

const SESSION_HANDLE_ID_BOUND_BYTES: usize = 1024;
const SESSION_HANDLE_OVERSIZE_WARNING: &str = "session handle omitted: id exceeds 1024 bytes";
const PINNED_EXTERNAL_SANDBOX_WARNING: &str =
    "DANGEROUS: external sandbox exec policy enabled (agent_api.exec.external_sandbox.v1=true)";

#[path = "claude_code/util.rs"]
mod util;

use util::{
    generic_non_zero_exit_message, json_contains_not_found_signal, parse_bool,
    preflight_allow_flag_support,
};

#[path = "claude_code/mapping.rs"]
mod mapping;

use mapping::{
    error_event, extract_assistant_message_final_text, map_stream_json_event, session_handle_facet,
    status_event,
};

#[cfg(test)]
use mapping::{map_assistant_message, map_stream_event};

#[derive(Clone, Debug, Default)]
pub struct ClaudeCodeBackendConfig {
    pub binary: Option<PathBuf>,
    pub claude_home: Option<PathBuf>,
    pub default_timeout: Option<Duration>,
    pub default_working_dir: Option<PathBuf>,
    pub env: BTreeMap<String, String>,
    pub allow_external_sandbox_exec: bool,
}

pub struct ClaudeCodeBackend {
    config: ClaudeCodeBackendConfig,
    allow_flag_preflight: Arc<OnceCell<bool>>,
}

impl ClaudeCodeBackend {
    pub fn new(config: ClaudeCodeBackendConfig) -> Self {
        Self {
            config,
            allow_flag_preflight: Arc::new(OnceCell::new()),
        }
    }
}

#[derive(Clone, Debug)]
struct ClaudeHarnessAdapter {
    config: ClaudeCodeBackendConfig,
    termination: Option<
        std::sync::Arc<super::termination::TerminationState<claude_code::ClaudeTerminationHandle>>,
    >,
    handle_state: Arc<Mutex<ClaudeHandleFacetState>>,
    allow_flag_preflight: Arc<OnceCell<bool>>,
}

#[derive(Clone, Debug)]
struct ClaudeExecPolicy {
    non_interactive: bool,
    external_sandbox: bool,
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
    ExternalSandboxWarning,
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
    ExternalSandboxPreflight { message: String },
}

fn render_backend_error_message(err: &ClaudeBackendError) -> String {
    match err {
        ClaudeBackendError::Spawn(err) | ClaudeBackendError::Completion(err) => {
            format!("claude_code error: {err}")
        }
        ClaudeBackendError::ExternalSandboxPreflight { message } => message.clone(),
        ClaudeBackendError::StreamParse(err) => err.message.clone(),
    }
}

fn startup_failure_spawn(
    err: ClaudeBackendError,
    emit_external_sandbox_warning: bool,
) -> BackendSpawn<ClaudeBackendEvent, ClaudeBackendCompletion, ClaudeBackendError> {
    let message = render_backend_error_message(&err);
    let events: crate::backend_harness::DynBackendEventStream<
        ClaudeBackendEvent,
        ClaudeBackendError,
    > = if emit_external_sandbox_warning {
        Box::pin(stream::iter(vec![
            Ok(ClaudeBackendEvent::ExternalSandboxWarning),
            Ok(ClaudeBackendEvent::TerminalError { message }),
        ]))
    } else {
        Box::pin(stream::once(async move {
            Ok(ClaudeBackendEvent::TerminalError { message })
        }))
    };
    let completion = Box::pin(async move { Err(err) });
    BackendSpawn { events, completion }
}

impl BackendHarnessAdapter for ClaudeHarnessAdapter {
    fn kind(&self) -> AgentWrapperKind {
        AgentWrapperKind(AGENT_KIND.to_string())
    }

    fn supported_extension_keys(&self) -> &'static [&'static str] {
        if self.config.allow_external_sandbox_exec {
            SUPPORTED_EXTENSION_KEYS_EXTERNAL_SANDBOX_OPT_IN
        } else {
            SUPPORTED_EXTENSION_KEYS_DEFAULT
        }
    }

    type Policy = ClaudeExecPolicy;

    fn validate_and_extract_policy(
        &self,
        request: &AgentWrapperRunRequest,
    ) -> Result<Self::Policy, AgentWrapperError> {
        let non_interactive_requested: Option<bool> = request
            .extensions
            .get(EXT_NON_INTERACTIVE)
            .map(|value| parse_bool(value, EXT_NON_INTERACTIVE))
            .transpose()?;
        let non_interactive = non_interactive_requested.unwrap_or(true);

        let external_sandbox = request
            .extensions
            .get(EXT_EXTERNAL_SANDBOX_V1)
            .map(|value| parse_bool(value, EXT_EXTERNAL_SANDBOX_V1))
            .transpose()?
            .unwrap_or(false);

        if external_sandbox {
            if non_interactive_requested == Some(false) {
                return Err(AgentWrapperError::InvalidRequest {
                    message: format!(
                        "{EXT_EXTERNAL_SANDBOX_V1}=true must not be combined with {EXT_NON_INTERACTIVE}=false"
                    ),
                });
            }

            if request
                .extensions
                .keys()
                .any(|key| key.starts_with(CLAUDE_EXEC_POLICY_PREFIX))
            {
                return Err(AgentWrapperError::InvalidRequest {
                    message: format!(
                        "{EXT_EXTERNAL_SANDBOX_V1}=true must not be combined with backend.claude_code.exec.* keys"
                    ),
                });
            }
        }

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
            external_sandbox,
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
        let allow_flag_preflight = Arc::clone(&self.allow_flag_preflight);
        Box::pin(async move {
            let mut builder = claude_code::ClaudeClient::builder();
            if let Some(binary) = config.binary.as_ref() {
                builder = builder.binary(binary.clone());
            }
            if let Some(claude_home) = config.claude_home.as_ref() {
                builder = builder.claude_home(claude_home.clone());
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

            let mut allow_dangerously_skip_permissions = false;
            if req.policy.external_sandbox {
                allow_dangerously_skip_permissions =
                    match preflight_allow_flag_support(allow_flag_preflight.as_ref(), || {
                        client.help()
                    })
                    .await
                    {
                        Ok(supported) => supported,
                        Err(message) => {
                            return Ok(startup_failure_spawn(
                                ClaudeBackendError::ExternalSandboxPreflight { message },
                                true,
                            ));
                        }
                    };
            }

            let mut print_req = ClaudePrintRequest::new(req.prompt)
                .output_format(ClaudeOutputFormat::StreamJson)
                .include_partial_messages(true);
            if req.policy.non_interactive {
                print_req = print_req.permission_mode("bypassPermissions");
            }
            if req.policy.external_sandbox {
                print_req = print_req.dangerously_skip_permissions(true);
                if allow_dangerously_skip_permissions {
                    print_req = print_req.allow_dangerously_skip_permissions(true);
                }
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

            let handle = match client.print_stream_json_control(print_req).await {
                Ok(handle) => handle,
                Err(err) if req.policy.external_sandbox => {
                    return Ok(startup_failure_spawn(ClaudeBackendError::Spawn(err), true));
                }
                Err(err) => return Err(ClaudeBackendError::Spawn(err)),
            };

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

            let events: crate::backend_harness::DynBackendEventStream<
                ClaudeBackendEvent,
                ClaudeBackendError,
            > = Box::pin(stream::unfold(
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

            let events = if req.policy.external_sandbox {
                Box::pin(
                    stream::once(async move { Ok(ClaudeBackendEvent::ExternalSandboxWarning) })
                        .chain(events),
                )
            } else {
                events
            };

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
            ClaudeBackendEvent::ExternalSandboxWarning => {
                return vec![status_event(Some(
                    PINNED_EXTERNAL_SANDBOX_WARNING.to_string(),
                ))];
            }
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
            _ => render_backend_error_message(err),
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
        if self.config.allow_external_sandbox_exec {
            ids.insert(EXT_EXTERNAL_SANDBOX_V1.to_string());
        }
        AgentWrapperCapabilities { ids }
    }

    fn run(
        &self,
        request: AgentWrapperRunRequest,
    ) -> Pin<Box<dyn Future<Output = Result<AgentWrapperRunHandle, AgentWrapperError>> + Send + '_>>
    {
        let config = self.config.clone();
        let allow_flag_preflight = Arc::clone(&self.allow_flag_preflight);
        Box::pin(async move {
            let adapter = Arc::new(ClaudeHarnessAdapter {
                config: config.clone(),
                termination: None,
                handle_state: Arc::new(Mutex::new(ClaudeHandleFacetState::default())),
                allow_flag_preflight,
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
        let allow_flag_preflight = Arc::clone(&self.allow_flag_preflight);
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
                allow_flag_preflight,
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
