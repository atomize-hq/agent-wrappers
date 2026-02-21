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
    AgentWrapperEvent, AgentWrapperKind, AgentWrapperRunHandle, AgentWrapperRunRequest,
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

const CAP_TOOLS_STRUCTURED_V1: &str = "agent_api.tools.structured.v1";
const CAP_TOOLS_RESULTS_V1: &str = "agent_api.tools.results.v1";
const CAP_ARTIFACTS_FINAL_TEXT_V1: &str = "agent_api.artifacts.final_text.v1";

const TOOLS_FACET_SCHEMA: &str = "agent_api.tools.structured.v1";

#[path = "codex/mapping.rs"]
mod mapping;

use mapping::{error_event, map_thread_event};

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

impl AgentWrapperBackend for CodexBackend {
    fn kind(&self) -> AgentWrapperKind {
        AgentWrapperKind("codex".to_string())
    }

    fn capabilities(&self) -> AgentWrapperCapabilities {
        let mut ids = BTreeSet::new();
        ids.insert("agent_api.run".to_string());
        ids.insert("agent_api.events".to_string());
        ids.insert("agent_api.events.live".to_string());
        ids.insert(CAP_TOOLS_STRUCTURED_V1.to_string());
        ids.insert(CAP_TOOLS_RESULTS_V1.to_string());
        ids.insert(CAP_ARTIFACTS_FINAL_TEXT_V1.to_string());
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
    let run_start_cwd = std::env::current_dir().ok();

    let (tx, rx) = mpsc::channel::<AgentWrapperEvent>(32);
    let (completion_tx, completion_rx) =
        oneshot::channel::<Result<AgentWrapperCompletion, AgentWrapperError>>();

    tokio::spawn(async move {
        let result = run_codex_inner(config, request, policy, run_start_cwd, tx).await;
        let _ = completion_tx.send(result);
    });

    Ok(crate::run_handle_gate::build_gated_run_handle(
        rx,
        completion_rx,
    ))
}

async fn run_codex_inner(
    config: CodexBackendConfig,
    request: AgentWrapperRunRequest,
    policy: CodexExecPolicy,
    run_start_cwd: Option<PathBuf>,
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
        .or(run_start_cwd);
    let working_dir = working_dir.ok_or_else(|| AgentWrapperError::Backend {
        message: "codex backend failed to resolve working directory".to_string(),
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
            let message = redact_exec_stream_error(&err);
            for bounded in crate::bounds::enforce_event_bounds(error_event(message.clone())) {
                if tx.send(bounded).await.is_err() {
                    break;
                }
            }
            drop(tx);
            return Err(AgentWrapperError::Backend { message });
        }
    };

    let completion_outcome =
        drain_events_while_polling_completion(handle.events, handle.completion, &tx).await?;

    let completion = match completion_outcome {
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
                crate::bounds::enforce_event_bounds(error_event(redact_exec_stream_error(&err)))
            {
                let _ = tx.send(bounded).await;
            }
            drop(tx);
            return Err(AgentWrapperError::Backend {
                message: redact_exec_stream_error(&err),
            });
        }
    };

    drop(tx);

    Ok(crate::bounds::enforce_completion_bounds(
        AgentWrapperCompletion {
            status: completion.status,
            final_text: crate::bounds::enforce_final_text_bound(completion.last_message),
            data: None,
        },
    ))
}

async fn drain_events_while_polling_completion(
    mut events: impl futures_core::Stream<Item = Result<ThreadEvent, ExecStreamError>> + Unpin,
    completion: impl Future<Output = Result<codex::ExecCompletion, ExecStreamError>> + Send + 'static,
    tx: &mpsc::Sender<AgentWrapperEvent>,
) -> Result<Result<codex::ExecCompletion, ExecStreamError>, AgentWrapperError> {
    let (completion_tx, completion_rx) = oneshot::channel();
    tokio::spawn(async move {
        let _ = completion_tx.send(completion.await);
    });

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
            Err(err) => vec![error_event(redact_exec_stream_error(&err))],
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

    completion_rx.await.map_err(|_| AgentWrapperError::Backend {
        message: "codex completion task dropped".to_string(),
    })
}

#[cfg(test)]
#[path = "codex/tests.rs"]
mod tests;
