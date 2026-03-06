use std::{
    collections::{BTreeMap, BTreeSet},
    future::Future,
    path::PathBuf,
    pin::Pin,
    process::ExitStatus,
    sync::{Arc, Mutex},
    time::Duration,
};

use codex::{CodexError, ExecStreamError, ThreadEvent};
use futures_util::{stream, StreamExt};

use super::session_selectors::{
    parse_session_resume_v1, validate_resume_fork_mutual_exclusion, SessionSelectorV1,
    EXT_SESSION_FORK_V1, EXT_SESSION_RESUME_V1,
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

impl super::termination::TerminationHandle for codex::ExecTerminationHandle {
    fn request_termination(&self) {
        codex::ExecTerminationHandle::request_termination(self);
    }
}

#[derive(Clone, Debug, Default)]
pub struct CodexBackendConfig {
    pub binary: Option<PathBuf>,
    pub codex_home: Option<PathBuf>,
    pub default_timeout: Option<Duration>,
    pub default_working_dir: Option<PathBuf>,
    pub env: BTreeMap<String, String>,
    pub allow_external_sandbox_exec: bool,
}

pub struct CodexBackend {
    config: CodexBackendConfig,
}

impl CodexBackend {
    pub fn new(config: CodexBackendConfig) -> Self {
        Self { config }
    }
}

const PINNED_APPROVAL_REQUIRED: &str = "approval required";
const PINNED_TIMEOUT: &str = "codex backend error: timeout (details redacted when unsafe)";
const PINNED_NO_SESSION_FOUND: &str = "no session found";
const PINNED_SESSION_NOT_FOUND: &str = "session not found";
const PINNED_EXTERNAL_SANDBOX_WARNING: &str =
    "DANGEROUS: external sandbox exec policy enabled (agent_api.exec.external_sandbox.v1=true)";
const PINNED_EXTERNAL_SANDBOX_FLAG_UNSUPPORTED: &str =
    "codex backend error: installed codex does not support --dangerously-bypass-approvals-and-sandbox (details redacted)";

fn pinned_selection_failure_message(selector: &SessionSelectorV1) -> &'static str {
    match selector {
        SessionSelectorV1::Last => PINNED_NO_SESSION_FOUND,
        SessionSelectorV1::Id { .. } => PINNED_SESSION_NOT_FOUND,
    }
}

fn is_not_found_signal(text: &str) -> bool {
    let text = text.to_ascii_lowercase();

    (text.contains("not found") && (text.contains("session") || text.contains("thread")))
        || text.contains("no session")
        || text.contains("no sessions")
        || text.contains("unknown session")
        || text.contains("no thread")
        || text.contains("no threads")
        || text.contains("unknown thread")
}

const CAP_TOOLS_STRUCTURED_V1: &str = "agent_api.tools.structured.v1";
const CAP_TOOLS_RESULTS_V1: &str = "agent_api.tools.results.v1";
const CAP_ARTIFACTS_FINAL_TEXT_V1: &str = "agent_api.artifacts.final_text.v1";
const CAP_SESSION_HANDLE_V1: &str = "agent_api.session.handle.v1";

const TOOLS_FACET_SCHEMA: &str = "agent_api.tools.structured.v1";

const SESSION_HANDLE_ID_BOUND_BYTES: usize = 1024;
const SESSION_HANDLE_OVERSIZE_WARNING_MARKER: &str = "session handle id oversize";

#[path = "codex/mapping.rs"]
mod mapping;

#[path = "codex/exec.rs"]
mod exec;

#[path = "codex/fork.rs"]
mod fork;

#[path = "codex/policy.rs"]
mod policy;

use policy::{
    validate_and_extract_exec_policy, CodexApprovalPolicy, CodexExecPolicy, CodexSandboxMode,
    EXT_CODEX_APPROVAL_POLICY, EXT_CODEX_SANDBOX_MODE, EXT_EXTERNAL_SANDBOX_V1,
    EXT_NON_INTERACTIVE, SUPPORTED_EXTENSION_KEYS_DEFAULT,
    SUPPORTED_EXTENSION_KEYS_EXTERNAL_SANDBOX_OPT_IN,
};

use mapping::{error_event, map_thread_event};

enum CodexTerminationHandle {
    Exec(codex::ExecTerminationHandle),
    AppServerTurn(fork::AppServerTurnCancelHandle),
}

impl std::fmt::Debug for CodexTerminationHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CodexTerminationHandle::Exec(_) => f.debug_tuple("Exec").field(&"<handle>").finish(),
            CodexTerminationHandle::AppServerTurn(_) => {
                f.debug_tuple("AppServerTurn").field(&"<handle>").finish()
            }
        }
    }
}

impl super::termination::TerminationHandle for CodexTerminationHandle {
    fn request_termination(&self) {
        match self {
            CodexTerminationHandle::Exec(handle) => {
                codex::ExecTerminationHandle::request_termination(handle);
            }
            CodexTerminationHandle::AppServerTurn(handle) => {
                super::termination::TerminationHandle::request_termination(handle);
            }
        }
    }
}

fn codex_error_kind(err: &CodexError) -> &'static str {
    match err {
        CodexError::Spawn { .. } => "spawn",
        CodexError::Wait { .. } => "wait",
        CodexError::Timeout { .. } => "timeout",
        CodexError::EmptyPrompt
        | CodexError::EmptySandboxCommand
        | CodexError::EmptyExecPolicyCommand
        | CodexError::EmptyApiKey
        | CodexError::EmptyTaskId
        | CodexError::EmptyEnvId
        | CodexError::EmptyMcpServerName
        | CodexError::EmptyMcpCommand
        | CodexError::EmptyMcpUrl
        | CodexError::EmptySocketPath => "invalid_request",
        CodexError::TempDir(_)
        | CodexError::WorkingDirectory { .. }
        | CodexError::PrepareOutputDirectory { .. }
        | CodexError::PrepareCodexHome { .. }
        | CodexError::StdoutUnavailable
        | CodexError::StderrUnavailable
        | CodexError::StdinUnavailable
        | CodexError::CaptureIo(_)
        | CodexError::StdinWrite(_)
        | CodexError::ResponsesApiProxyInfoRead { .. }
        | CodexError::Join(_) => "io",
        CodexError::NonZeroExit { .. }
        | CodexError::InvalidUtf8(_)
        | CodexError::JsonParse { .. }
        | CodexError::ExecPolicyParse { .. }
        | CodexError::FeatureListParse { .. }
        | CodexError::ResponsesApiProxyInfoParse { .. } => "other",
    }
}

fn redact_exec_stream_error(err: &ExecStreamError) -> String {
    match err {
        ExecStreamError::Parse { source, line } => format!(
            "codex stream parse error (redacted): {source} (line_bytes={})",
            line.len()
        ),
        ExecStreamError::Normalize { message, line } => format!(
            "codex stream normalize error (redacted): {message} (line_bytes={})",
            line.len()
        ),
        ExecStreamError::IdleTimeout { idle_for } => {
            format!("codex stream idle timeout: {idle_for:?}")
        }
        ExecStreamError::ChannelClosed => "codex stream closed unexpectedly".to_string(),
        ExecStreamError::Codex(CodexError::NonZeroExit { status, .. }) => {
            format!("codex exited non-zero: {status:?} (stderr redacted)")
        }
        ExecStreamError::Codex(err) => format!(
            "codex backend error: {} (details redacted when unsafe)",
            codex_error_kind(err)
        ),
    }
}

fn render_backend_error_message(err: &CodexBackendError) -> String {
    match err {
        CodexBackendError::Exec(err) => redact_exec_stream_error(err),
        CodexBackendError::AppServer(codex::mcp::McpError::Handshake(_)) => {
            "codex app-server handshake failed".to_string()
        }
        CodexBackendError::AppServer(_) => "codex app-server rpc error".to_string(),
        CodexBackendError::Timeout { timeout: _timeout } => PINNED_TIMEOUT.to_string(),
        CodexBackendError::ForkSelectionEmpty => PINNED_NO_SESSION_FOUND.to_string(),
        CodexBackendError::ForkSessionNotFound => PINNED_SESSION_NOT_FOUND.to_string(),
        CodexBackendError::CompletionTaskDropped => "codex completion task dropped".to_string(),
        CodexBackendError::WorkingDirectoryUnresolved => {
            "codex backend failed to resolve working directory".to_string()
        }
    }
}

fn startup_failure_spawn(
    err: CodexBackendError,
    emit_external_sandbox_warning: bool,
) -> BackendSpawn<CodexBackendEvent, CodexBackendCompletion, CodexBackendError> {
    let message = render_backend_error_message(&err);
    let events: crate::backend_harness::DynBackendEventStream<
        CodexBackendEvent,
        CodexBackendError,
    > = if emit_external_sandbox_warning {
        Box::pin(stream::iter(vec![
            Ok(CodexBackendEvent::ExternalSandboxWarning),
            Ok(CodexBackendEvent::TerminalError { message }),
        ]))
    } else {
        Box::pin(stream::once(async move {
            Ok(CodexBackendEvent::TerminalError { message })
        }))
    };
    let completion = Box::pin(async move { Err(err) });
    BackendSpawn { events, completion }
}

#[derive(Clone, Debug)]
struct CodexHarnessAdapter {
    config: CodexBackendConfig,
    run_start_cwd: Option<PathBuf>,
    termination:
        Option<std::sync::Arc<super::termination::TerminationState<CodexTerminationHandle>>>,
    handle_state: Arc<Mutex<CodexHandleFacetState>>,
}

#[derive(Debug, Default)]
struct CodexHandleFacetState {
    thread_id: Option<String>,
    handle_facet_emitted: bool,
    oversize_warning_emitted: bool,
    oversize_warning_len_bytes: Option<usize>,
}

#[derive(Debug)]
enum CodexBackendEvent {
    ExternalSandboxWarning,
    Thread(Box<ThreadEvent>),
    AppServerNotification(codex::mcp::AppNotification),
    NonZeroExit { status: ExitStatus },
    TerminalError { message: String },
}

#[derive(Clone, Debug)]
struct CodexBackendCompletion {
    status: ExitStatus,
    final_text: Option<String>,
    selection_failure_message: Option<String>,
}

#[derive(Debug)]
enum CodexBackendError {
    Exec(ExecStreamError),
    AppServer(codex::mcp::McpError),
    Timeout { timeout: Duration },
    ForkSelectionEmpty,
    ForkSessionNotFound,
    CompletionTaskDropped,
    WorkingDirectoryUnresolved,
}

fn session_handle_facet(thread_id: &str) -> serde_json::Value {
    serde_json::json!({
        "schema": CAP_SESSION_HANDLE_V1,
        "session": { "id": thread_id },
    })
}

impl BackendHarnessAdapter for CodexHarnessAdapter {
    fn kind(&self) -> AgentWrapperKind {
        AgentWrapperKind("codex".to_string())
    }

    fn supported_extension_keys(&self) -> &'static [&'static str] {
        if self.config.allow_external_sandbox_exec {
            SUPPORTED_EXTENSION_KEYS_EXTERNAL_SANDBOX_OPT_IN
        } else {
            SUPPORTED_EXTENSION_KEYS_DEFAULT
        }
    }

    type Policy = CodexExecPolicy;

    fn validate_and_extract_policy(
        &self,
        request: &AgentWrapperRunRequest,
    ) -> Result<Self::Policy, AgentWrapperError> {
        let exec_policy = validate_and_extract_exec_policy(request)?;

        let resume = request
            .extensions
            .get(EXT_SESSION_RESUME_V1)
            .map(parse_session_resume_v1)
            .transpose()?;

        let fork = fork::extract_fork_selector_v1(request)?;

        validate_resume_fork_mutual_exclusion(&request.extensions)?;

        Ok(CodexExecPolicy {
            resume,
            fork,
            ..exec_policy
        })
    }

    type BackendEvent = CodexBackendEvent;
    type BackendCompletion = CodexBackendCompletion;
    type BackendError = CodexBackendError;

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
        let run_start_cwd = self.run_start_cwd.clone();
        let termination = self.termination.clone();
        let handle_state = self.handle_state.clone();
        let CodexExecPolicy {
            non_interactive,
            external_sandbox,
            approval_policy,
            sandbox_mode,
            resume,
            fork,
        } = req.policy;
        let prompt = req.prompt;
        let working_dir = req.working_dir;
        let effective_timeout = req.effective_timeout;
        let env = req.env;

        Box::pin(async move {
            let spawned = match if let Some(selector) = fork {
                fork::spawn_fork_v1_flow(fork::ForkFlowRequest {
                    selector,
                    prompt,
                    working_dir,
                    effective_timeout,
                    env,
                    config,
                    run_start_cwd,
                    termination,
                    non_interactive,
                    external_sandbox,
                    approval_policy,
                    sandbox_mode,
                    handle_state,
                })
                .await
            } else {
                exec::spawn_exec_or_resume_flow(exec::ExecFlowRequest {
                    config,
                    run_start_cwd,
                    termination,
                    non_interactive,
                    external_sandbox,
                    approval_policy,
                    sandbox_mode,
                    resume,
                    prompt,
                    working_dir,
                    effective_timeout,
                    env,
                })
                .await
            } {
                Ok(spawned) => spawned,
                Err(err) if external_sandbox => return Ok(startup_failure_spawn(err, true)),
                Err(err) => return Err(err),
            };

            let BackendSpawn { events, completion } = spawned;
            let events = if external_sandbox {
                Box::pin(
                    stream::once(async move { Ok(CodexBackendEvent::ExternalSandboxWarning) })
                        .chain(events),
                )
            } else {
                events
            };

            Ok(BackendSpawn { events, completion })
        })
    }

    fn map_event(&self, event: Self::BackendEvent) -> Vec<AgentWrapperEvent> {
        match event {
            CodexBackendEvent::ExternalSandboxWarning => vec![mapping::status_event(Some(
                PINNED_EXTERNAL_SANDBOX_WARNING.to_string(),
            ))],
            CodexBackendEvent::Thread(ev) => {
                let mut emit_oversize_warning_len: Option<usize> = None;

                if let Ok(mut state) = self.handle_state.lock() {
                    if state.thread_id.is_none() {
                        if let Some(thread_id) = ev.thread_id() {
                            if !thread_id.trim().is_empty() {
                                let len = thread_id.len();
                                if len <= SESSION_HANDLE_ID_BOUND_BYTES {
                                    state.thread_id = Some(thread_id.to_string());
                                } else if !state.oversize_warning_emitted {
                                    state.oversize_warning_emitted = true;
                                    emit_oversize_warning_len = Some(len);
                                }
                            }
                        }
                    }
                }

                let mut mapped = vec![map_thread_event(&ev)];

                let emit_handle_facet: Option<String> =
                    self.handle_state.lock().ok().and_then(|mut state| {
                        if state.handle_facet_emitted {
                            return None;
                        }
                        let thread_id = state.thread_id.clone()?;
                        state.handle_facet_emitted = true;
                        Some(thread_id)
                    });

                if let Some(thread_id) = emit_handle_facet.as_deref() {
                    let mut attached = false;
                    for event in mapped.iter_mut() {
                        if event.kind == AgentWrapperEventKind::Status && event.data.is_none() {
                            event.data = Some(session_handle_facet(thread_id));
                            attached = true;
                            break;
                        }
                    }

                    if !attached {
                        let mut event = mapping::status_event(None);
                        event.data = Some(session_handle_facet(thread_id));
                        mapped.push(event);
                    }
                }

                if let Some(len) = emit_oversize_warning_len {
                    mapped.push(mapping::status_event(Some(format!(
                        "{SESSION_HANDLE_OVERSIZE_WARNING_MARKER}: len_bytes={len}"
                    ))));
                }

                mapped
            }
            CodexBackendEvent::AppServerNotification(notification) => {
                let mut mapped = match notification {
                    codex::mcp::AppNotification::Raw { method, params } => {
                        fork::map_app_server_notification(&method, &params)
                            .into_iter()
                            .collect()
                    }
                    codex::mcp::AppNotification::Error { message, .. } => {
                        vec![error_event(message)]
                    }
                    _ => Vec::new(),
                };

                let emit_handle_facet: Option<String> =
                    self.handle_state.lock().ok().and_then(|mut state| {
                        if state.handle_facet_emitted {
                            return None;
                        }
                        let thread_id = state.thread_id.clone()?;
                        state.handle_facet_emitted = true;
                        Some(thread_id)
                    });

                if let Some(thread_id) = emit_handle_facet.as_deref() {
                    let mut attached = false;
                    for event in mapped.iter_mut() {
                        if event.kind == AgentWrapperEventKind::Status && event.data.is_none() {
                            event.data = Some(session_handle_facet(thread_id));
                            attached = true;
                            break;
                        }
                    }

                    if !attached {
                        let mut event = mapping::status_event(None);
                        event.data = Some(session_handle_facet(thread_id));
                        mapped.push(event);
                    }
                }

                let emit_oversize_warning_len: Option<usize> = self
                    .handle_state
                    .lock()
                    .ok()
                    .and_then(|mut state| state.oversize_warning_len_bytes.take());

                if let Some(len) = emit_oversize_warning_len {
                    mapped.push(mapping::status_event(Some(format!(
                        "{SESSION_HANDLE_OVERSIZE_WARNING_MARKER}: len_bytes={len}"
                    ))));
                }

                mapped
            }
            CodexBackendEvent::NonZeroExit { status } => {
                let mut mapped = Vec::new();
                if let Some(len) = self
                    .handle_state
                    .lock()
                    .ok()
                    .and_then(|mut state| state.oversize_warning_len_bytes.take())
                {
                    mapped.push(mapping::status_event(Some(format!(
                        "{SESSION_HANDLE_OVERSIZE_WARNING_MARKER}: len_bytes={len}"
                    ))));
                }
                mapped.push(error_event(format!(
                    "codex exited non-zero: {status:?} (stderr redacted)"
                )));
                mapped
            }
            CodexBackendEvent::TerminalError { message } => {
                let mut mapped = Vec::new();
                if let Some(len) = self
                    .handle_state
                    .lock()
                    .ok()
                    .and_then(|mut state| state.oversize_warning_len_bytes.take())
                {
                    mapped.push(mapping::status_event(Some(format!(
                        "{SESSION_HANDLE_OVERSIZE_WARNING_MARKER}: len_bytes={len}"
                    ))));
                }
                mapped.push(error_event(message));
                mapped
            }
        }
    }

    fn map_completion(
        &self,
        completion: Self::BackendCompletion,
    ) -> Result<AgentWrapperCompletion, AgentWrapperError> {
        let CodexBackendCompletion {
            status,
            final_text,
            selection_failure_message,
        } = completion;

        if let Some(message) = selection_failure_message {
            return Err(AgentWrapperError::Backend { message });
        }

        let handle_facet = self
            .handle_state
            .lock()
            .ok()
            .and_then(|state| state.thread_id.clone())
            .map(|thread_id| session_handle_facet(&thread_id));

        Ok(AgentWrapperCompletion {
            status,
            final_text: crate::bounds::enforce_final_text_bound(final_text),
            data: handle_facet,
        })
    }

    fn redact_error(&self, _phase: BackendHarnessErrorPhase, err: &Self::BackendError) -> String {
        render_backend_error_message(err)
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
        ids.insert(crate::CAPABILITY_CONTROL_CANCEL_V1.to_string());
        ids.insert(CAP_TOOLS_STRUCTURED_V1.to_string());
        ids.insert(CAP_TOOLS_RESULTS_V1.to_string());
        ids.insert(CAP_ARTIFACTS_FINAL_TEXT_V1.to_string());
        ids.insert(CAP_SESSION_HANDLE_V1.to_string());
        ids.insert("backend.codex.exec_stream".to_string());
        ids.insert(EXT_NON_INTERACTIVE.to_string());
        ids.insert(EXT_CODEX_APPROVAL_POLICY.to_string());
        ids.insert(EXT_CODEX_SANDBOX_MODE.to_string());
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
        Box::pin(async move {
            let run_start_cwd = std::env::current_dir().ok();
            let adapter = Arc::new(CodexHarnessAdapter {
                config: config.clone(),
                run_start_cwd,
                termination: None,
                handle_state: Arc::new(Mutex::new(CodexHandleFacetState::default())),
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
            let termination_state: Arc<
                super::termination::TerminationState<CodexTerminationHandle>,
            > = Arc::new(super::termination::TerminationState::new());
            let request_termination: Option<Arc<dyn Fn() + Send + Sync + 'static>> = Some({
                let termination_state = Arc::clone(&termination_state);
                Arc::new(move || termination_state.request())
            });

            let run_start_cwd = std::env::current_dir().ok();
            let adapter = Arc::new(CodexHarnessAdapter {
                config: config.clone(),
                run_start_cwd,
                termination: Some(termination_state),
                handle_state: Arc::new(Mutex::new(CodexHandleFacetState::default())),
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
#[path = "codex/tests.rs"]
mod tests;
