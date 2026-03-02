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
use serde_json::Value;

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

const PINNED_APPROVAL_REQUIRED: &str = "approval required";
const PINNED_NO_SESSION_FOUND: &str = "no session found";
const PINNED_SESSION_NOT_FOUND: &str = "session not found";

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
    resume: Option<SessionSelectorV1>,
    fork: Option<SessionSelectorV1>,
}

fn validate_and_extract_exec_policy(
    request: &AgentWrapperRunRequest,
) -> Result<CodexExecPolicy, AgentWrapperError> {
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
        resume: None,
        fork: None,
    })
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
        &[
            EXT_NON_INTERACTIVE,
            EXT_CODEX_APPROVAL_POLICY,
            EXT_CODEX_SANDBOX_MODE,
            EXT_SESSION_RESUME_V1,
            EXT_SESSION_FORK_V1,
        ]
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
            if let Some(selector) = fork {
                return fork::spawn_fork_v1_flow(fork::ForkFlowRequest {
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
                })
                .await;
            }

            exec::spawn_exec_or_resume_flow(exec::ExecFlowRequest {
                config,
                run_start_cwd,
                termination,
                non_interactive,
                approval_policy,
                sandbox_mode,
                resume,
                prompt,
                working_dir,
                effective_timeout,
                env,
            })
            .await
        })
    }

    fn map_event(&self, event: Self::BackendEvent) -> Vec<AgentWrapperEvent> {
        match event {
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

                let emit_oversize_warning_len: Option<usize> =
                    self.handle_state.lock().ok().and_then(|mut state| {
                        state.oversize_warning_len_bytes.take()
                    });

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
        match err {
            CodexBackendError::Exec(err) => redact_exec_stream_error(err),
            CodexBackendError::AppServer(codex::mcp::McpError::Handshake(_)) => {
                "codex app-server handshake failed".to_string()
            }
            CodexBackendError::AppServer(_) => "codex app-server rpc error".to_string(),
            CodexBackendError::ForkSelectionEmpty => PINNED_NO_SESSION_FOUND.to_string(),
            CodexBackendError::ForkSessionNotFound => PINNED_SESSION_NOT_FOUND.to_string(),
            CodexBackendError::CompletionTaskDropped => "codex completion task dropped".to_string(),
            CodexBackendError::WorkingDirectoryUnresolved => {
                "codex backend failed to resolve working directory".to_string()
            }
        }
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
