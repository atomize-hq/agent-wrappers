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

const CAP_TOOLS_STRUCTURED_V1: &str = "agent_api.tools.structured.v1";
const CAP_TOOLS_RESULTS_V1: &str = "agent_api.tools.results.v1";
const CAP_ARTIFACTS_FINAL_TEXT_V1: &str = "agent_api.artifacts.final_text.v1";

const TOOLS_FACET_SCHEMA: &str = "agent_api.tools.structured.v1";

fn map_thread_event(event: &ThreadEvent) -> AgentWrapperEvent {
    match event {
        ThreadEvent::ThreadStarted(_) => status_event(None),
        ThreadEvent::TurnStarted(_) => status_event(None),
        ThreadEvent::TurnCompleted(_) => status_event(None),
        ThreadEvent::TurnFailed(_) => status_event(Some("turn failed".to_string())),
        ThreadEvent::Error(err) => error_event(err.message.clone()),
        ThreadEvent::ItemStarted(envelope) => map_item_snapshot_event(envelope, ToolPhase::Start),
        ThreadEvent::ItemCompleted(envelope) => {
            map_item_snapshot_event(envelope, ToolPhase::Complete)
        }
        ThreadEvent::ItemDelta(delta) => map_item_delta_event(delta),
        ThreadEvent::ItemFailed(envelope) => map_item_failed_event(envelope),
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

#[derive(Copy, Clone, Debug)]
enum ToolPhase {
    Start,
    Delta,
    Complete,
    Fail,
}

#[derive(Copy, Clone, Debug, Default)]
struct ToolBytes {
    stdout: usize,
    stderr: usize,
    diff: usize,
    result: usize,
}

fn is_toolish_item_type(item_type: &str) -> bool {
    matches!(
        item_type,
        "command_execution" | "file_change" | "mcp_tool_call" | "web_search"
    )
}

fn tools_facet_data(
    backend_item_id: Option<&str>,
    thread_id: Option<&str>,
    turn_id: Option<&str>,
    kind: &str,
    phase: ToolPhase,
    status: &str,
    exit_code: Option<i32>,
    bytes: ToolBytes,
) -> Value {
    let phase = match phase {
        ToolPhase::Start => "start",
        ToolPhase::Delta => "delta",
        ToolPhase::Complete => "complete",
        ToolPhase::Fail => "fail",
    };

    serde_json::json!({
        "schema": TOOLS_FACET_SCHEMA,
        "tool": {
            "backend_item_id": backend_item_id,
            "thread_id": thread_id,
            "turn_id": turn_id,
            "kind": kind,
            "phase": phase,
            "status": status,
            "exit_code": exit_code,
            "bytes": {
                "stdout": bytes.stdout,
                "stderr": bytes.stderr,
                "diff": bytes.diff,
                "result": bytes.result
            },
            "tool_name": null,
            "tool_use_id": null
        }
    })
}

fn tool_result_bytes(value: &Option<Value>) -> usize {
    let Some(value) = value else {
        return 0;
    };
    serde_json::to_vec(value).map(|v| v.len()).unwrap_or(0)
}

fn map_item_snapshot_event(
    envelope: &codex::ItemEnvelope<codex::ItemSnapshot>,
    phase: ToolPhase,
) -> AgentWrapperEvent {
    match &envelope.item.payload {
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
        codex::ItemPayload::CommandExecution(state) => {
            let (kind, status, event_kind) = match phase {
                ToolPhase::Complete => (
                    "command_execution",
                    "completed",
                    AgentWrapperEventKind::ToolResult,
                ),
                ToolPhase::Start => (
                    "command_execution",
                    "running",
                    AgentWrapperEventKind::ToolCall,
                ),
                ToolPhase::Delta | ToolPhase::Fail => (
                    "command_execution",
                    "running",
                    AgentWrapperEventKind::ToolCall,
                ),
            };
            let bytes = ToolBytes {
                stdout: state.stdout.len(),
                stderr: state.stderr.len(),
                diff: 0,
                result: 0,
            };
            AgentWrapperEvent {
                agent_kind: AgentWrapperKind("codex".to_string()),
                kind: event_kind,
                channel: Some("tool".to_string()),
                text: None,
                message: None,
                data: Some(tools_facet_data(
                    Some(envelope.item.item_id.as_str()),
                    Some(envelope.thread_id.as_str()),
                    Some(envelope.turn_id.as_str()),
                    kind,
                    phase,
                    status,
                    state.exit_code,
                    bytes,
                )),
            }
        }
        codex::ItemPayload::FileChange(state) => {
            let (kind, status, event_kind) = match phase {
                ToolPhase::Complete => (
                    "file_change",
                    "completed",
                    AgentWrapperEventKind::ToolResult,
                ),
                ToolPhase::Start => ("file_change", "running", AgentWrapperEventKind::ToolCall),
                ToolPhase::Delta | ToolPhase::Fail => {
                    ("file_change", "running", AgentWrapperEventKind::ToolCall)
                }
            };
            let bytes = ToolBytes {
                stdout: state.stdout.len(),
                stderr: state.stderr.len(),
                diff: state.diff.as_ref().map(|s| s.len()).unwrap_or(0),
                result: 0,
            };
            AgentWrapperEvent {
                agent_kind: AgentWrapperKind("codex".to_string()),
                kind: event_kind,
                channel: Some("tool".to_string()),
                text: None,
                message: None,
                data: Some(tools_facet_data(
                    Some(envelope.item.item_id.as_str()),
                    Some(envelope.thread_id.as_str()),
                    Some(envelope.turn_id.as_str()),
                    kind,
                    phase,
                    status,
                    state.exit_code,
                    bytes,
                )),
            }
        }
        codex::ItemPayload::McpToolCall(state) => {
            let (kind, status, event_kind) = match phase {
                ToolPhase::Complete => (
                    "mcp_tool_call",
                    "completed",
                    AgentWrapperEventKind::ToolResult,
                ),
                ToolPhase::Start => ("mcp_tool_call", "running", AgentWrapperEventKind::ToolCall),
                ToolPhase::Delta | ToolPhase::Fail => {
                    ("mcp_tool_call", "running", AgentWrapperEventKind::ToolCall)
                }
            };
            let bytes = ToolBytes {
                stdout: 0,
                stderr: 0,
                diff: 0,
                // Count only MCP tool output (`result`), never `arguments`.
                result: tool_result_bytes(&state.result),
            };
            AgentWrapperEvent {
                agent_kind: AgentWrapperKind("codex".to_string()),
                kind: event_kind,
                channel: Some("tool".to_string()),
                text: None,
                message: None,
                data: Some(tools_facet_data(
                    Some(envelope.item.item_id.as_str()),
                    Some(envelope.thread_id.as_str()),
                    Some(envelope.turn_id.as_str()),
                    kind,
                    phase,
                    status,
                    None,
                    bytes,
                )),
            }
        }
        codex::ItemPayload::WebSearch(state) => {
            let (kind, status, event_kind) = match phase {
                ToolPhase::Complete => {
                    ("web_search", "completed", AgentWrapperEventKind::ToolResult)
                }
                ToolPhase::Start => ("web_search", "running", AgentWrapperEventKind::ToolCall),
                ToolPhase::Delta | ToolPhase::Fail => {
                    ("web_search", "running", AgentWrapperEventKind::ToolCall)
                }
            };
            let bytes = ToolBytes {
                stdout: 0,
                stderr: 0,
                diff: 0,
                // Count only web search output (`results`), never `query`.
                result: tool_result_bytes(&state.results),
            };
            AgentWrapperEvent {
                agent_kind: AgentWrapperKind("codex".to_string()),
                kind: event_kind,
                channel: Some("tool".to_string()),
                text: None,
                message: None,
                data: Some(tools_facet_data(
                    Some(envelope.item.item_id.as_str()),
                    Some(envelope.thread_id.as_str()),
                    Some(envelope.turn_id.as_str()),
                    kind,
                    phase,
                    status,
                    None,
                    bytes,
                )),
            }
        }
        codex::ItemPayload::TodoList(_) => status_event(None),
        codex::ItemPayload::Error(err) => error_event(err.message.clone()),
    }
}

fn map_item_delta_event(delta: &codex::ItemDelta) -> AgentWrapperEvent {
    match &delta.delta {
        codex::ItemDeltaPayload::AgentMessage(content)
        | codex::ItemDeltaPayload::Reasoning(content) => AgentWrapperEvent {
            agent_kind: AgentWrapperKind("codex".to_string()),
            kind: AgentWrapperEventKind::TextOutput,
            channel: Some("assistant".to_string()),
            text: Some(content.text_delta.clone()),
            message: None,
            data: None,
        },
        codex::ItemDeltaPayload::CommandExecution(state) => {
            let bytes = ToolBytes {
                stdout: state.stdout.len(),
                stderr: state.stderr.len(),
                diff: 0,
                result: 0,
            };
            AgentWrapperEvent {
                agent_kind: AgentWrapperKind("codex".to_string()),
                kind: AgentWrapperEventKind::ToolCall,
                channel: Some("tool".to_string()),
                text: None,
                message: None,
                data: Some(tools_facet_data(
                    Some(delta.item_id.as_str()),
                    Some(delta.thread_id.as_str()),
                    Some(delta.turn_id.as_str()),
                    "command_execution",
                    ToolPhase::Delta,
                    "running",
                    state.exit_code,
                    bytes,
                )),
            }
        }
        codex::ItemDeltaPayload::FileChange(state) => {
            let bytes = ToolBytes {
                stdout: state.stdout.len(),
                stderr: state.stderr.len(),
                diff: state.diff.as_ref().map(|s| s.len()).unwrap_or(0),
                result: 0,
            };
            AgentWrapperEvent {
                agent_kind: AgentWrapperKind("codex".to_string()),
                kind: AgentWrapperEventKind::ToolCall,
                channel: Some("tool".to_string()),
                text: None,
                message: None,
                data: Some(tools_facet_data(
                    Some(delta.item_id.as_str()),
                    Some(delta.thread_id.as_str()),
                    Some(delta.turn_id.as_str()),
                    "file_change",
                    ToolPhase::Delta,
                    "running",
                    state.exit_code,
                    bytes,
                )),
            }
        }
        codex::ItemDeltaPayload::McpToolCall(state) => {
            let bytes = ToolBytes {
                stdout: 0,
                stderr: 0,
                diff: 0,
                // Count only MCP tool output (`result`), never `arguments`.
                result: tool_result_bytes(&state.result),
            };
            AgentWrapperEvent {
                agent_kind: AgentWrapperKind("codex".to_string()),
                kind: AgentWrapperEventKind::ToolCall,
                channel: Some("tool".to_string()),
                text: None,
                message: None,
                data: Some(tools_facet_data(
                    Some(delta.item_id.as_str()),
                    Some(delta.thread_id.as_str()),
                    Some(delta.turn_id.as_str()),
                    "mcp_tool_call",
                    ToolPhase::Delta,
                    "running",
                    None,
                    bytes,
                )),
            }
        }
        codex::ItemDeltaPayload::WebSearch(state) => {
            let bytes = ToolBytes {
                stdout: 0,
                stderr: 0,
                diff: 0,
                // Count only web search output (`results`), never `query`.
                result: tool_result_bytes(&state.results),
            };
            AgentWrapperEvent {
                agent_kind: AgentWrapperKind("codex".to_string()),
                kind: AgentWrapperEventKind::ToolCall,
                channel: Some("tool".to_string()),
                text: None,
                message: None,
                data: Some(tools_facet_data(
                    Some(delta.item_id.as_str()),
                    Some(delta.thread_id.as_str()),
                    Some(delta.turn_id.as_str()),
                    "web_search",
                    ToolPhase::Delta,
                    "running",
                    None,
                    bytes,
                )),
            }
        }
        codex::ItemDeltaPayload::TodoList(_) => status_event(None),
        codex::ItemDeltaPayload::Error(err) => error_event(err.message.clone()),
    }
}

fn map_item_failed_event(envelope: &codex::ItemEnvelope<codex::ItemFailure>) -> AgentWrapperEvent {
    // IMPORTANT: `ItemFailure.extra["item_type"]` is populated from a *top-level* `item_type` field
    // on the `item.failed` JSON object, not from a nested `{ "extra": { ... } }` object.
    let item_type = envelope.item.extra.get("item_type").and_then(Value::as_str);
    let Some(item_type) = item_type else {
        return error_event(envelope.item.error.message.clone());
    };
    if !is_toolish_item_type(item_type) {
        return error_event(envelope.item.error.message.clone());
    }

    AgentWrapperEvent {
        agent_kind: AgentWrapperKind("codex".to_string()),
        kind: AgentWrapperEventKind::ToolResult,
        channel: Some("tool".to_string()),
        text: None,
        message: None,
        data: Some(tools_facet_data(
            Some(envelope.item.item_id.as_str()),
            Some(envelope.thread_id.as_str()),
            Some(envelope.turn_id.as_str()),
            // Failed ToolResult kind is derived from the deterministically-attributable item_type.
            item_type,
            ToolPhase::Fail,
            "failed",
            // Failure attribution is metadata-only: no exit_code, and all byte counts are zero.
            None,
            ToolBytes::default(),
        )),
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

fn enforce_final_text_bound(text: Option<String>) -> Option<String> {
    fn utf8_truncate_to_bytes(s: &str, bound_bytes: usize) -> &str {
        if s.len() <= bound_bytes {
            return s;
        }
        let mut end = std::cmp::min(bound_bytes, s.len());
        while end > 0 && !s.is_char_boundary(end) {
            end -= 1;
        }
        &s[..end]
    }

    let text = text?;
    if text.len() <= crate::bounds::TEXT_BOUND_BYTES {
        return Some(text);
    }

    const SUFFIX: &str = "…(truncated)";
    let suffix_bytes = SUFFIX.len();
    let bound = crate::bounds::TEXT_BOUND_BYTES;
    if bound <= suffix_bytes {
        return Some(utf8_truncate_to_bytes("…", bound).to_string());
    }

    let prefix = utf8_truncate_to_bytes(&text, bound - suffix_bytes);
    let mut out = String::with_capacity(bound);
    out.push_str(prefix);
    out.push_str(SUFFIX);
    Some(out)
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
            final_text: enforce_final_text_bound(completion.last_message),
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
mod tests {
    use super::*;
    use crate::{AgentWrapperBackend, AgentWrapperEventKind};

    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };

    fn parse_thread_event(json: &str) -> ThreadEvent {
        serde_json::from_str(json).expect("valid codex::ThreadEvent JSON")
    }

    fn map(json: &str) -> AgentWrapperEvent {
        let event = parse_thread_event(json);
        map_thread_event(&event)
    }

    fn tool_schema(event: &AgentWrapperEvent) -> Option<&str> {
        event
            .data
            .as_ref()
            .and_then(|data| data.get("schema"))
            .and_then(Value::as_str)
    }

    fn tool_field<'a>(event: &'a AgentWrapperEvent, field: &str) -> Option<&'a Value> {
        event
            .data
            .as_ref()
            .and_then(|data| data.get("tool"))
            .and_then(|tool| tool.get(field))
    }

    #[test]
    fn codex_backend_reports_required_capabilities() {
        let backend = CodexBackend::new(CodexBackendConfig::default());
        let capabilities = backend.capabilities();
        assert!(capabilities.contains("agent_api.run"));
        assert!(capabilities.contains("agent_api.events"));
        assert!(capabilities.contains("agent_api.events.live"));
        assert!(capabilities.contains(CAP_TOOLS_STRUCTURED_V1));
        assert!(capabilities.contains(CAP_TOOLS_RESULTS_V1));
        assert!(capabilities.contains(CAP_ARTIFACTS_FINAL_TEXT_V1));
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
    fn item_failed_without_item_type_maps_to_error_with_message() {
        let mapped = map(
            r#"{"type":"item.failed","thread_id":"thread-1","turn_id":"turn-1","item_id":"item-1","error":{"message":"tool failed"}}"#,
        );
        assert_eq!(mapped.kind, AgentWrapperEventKind::Error);
        assert_eq!(mapped.text, None);
        assert!(mapped.message.is_some());
    }

    #[test]
    fn item_failed_with_tool_item_type_maps_to_tool_result_failed() {
        // IMPORTANT: item_type must be a top-level field so it lands in ItemFailure.extra["item_type"].
        let mapped = map(
            r#"{"type":"item.failed","thread_id":"thread-1","turn_id":"turn-1","item_id":"item-1","item_type":"command_execution","error":{"message":"tool failed"}}"#,
        );
        assert_eq!(mapped.kind, AgentWrapperEventKind::ToolResult);
        assert_eq!(tool_schema(&mapped), Some(TOOLS_FACET_SCHEMA));
        assert_eq!(
            tool_field(&mapped, "phase").and_then(Value::as_str),
            Some("fail")
        );
        assert_eq!(
            tool_field(&mapped, "status").and_then(Value::as_str),
            Some("failed")
        );
        assert_eq!(
            tool_field(&mapped, "kind").and_then(Value::as_str),
            Some("command_execution")
        );
        assert_eq!(tool_field(&mapped, "exit_code"), Some(&Value::Null));
        let bytes = tool_field(&mapped, "bytes")
            .and_then(Value::as_object)
            .unwrap();
        assert_eq!(bytes.get("stdout"), Some(&Value::from(0)));
        assert_eq!(bytes.get("stderr"), Some(&Value::from(0)));
        assert_eq!(bytes.get("diff"), Some(&Value::from(0)));
        assert_eq!(bytes.get("result"), Some(&Value::from(0)));
    }

    #[test]
    fn item_failed_with_non_tool_item_type_maps_to_error() {
        // IMPORTANT: item_type must be a top-level field so it lands in ItemFailure.extra["item_type"].
        let mapped = map(
            r#"{"type":"item.failed","thread_id":"thread-1","turn_id":"turn-1","item_id":"item-1","item_type":"agent_message","error":{"message":"not a tool failure"}}"#,
        );
        assert_eq!(mapped.kind, AgentWrapperEventKind::Error);
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
        assert_eq!(tool_schema(&mapped), Some(TOOLS_FACET_SCHEMA));
        assert_eq!(
            tool_field(&mapped, "phase").and_then(Value::as_str),
            Some("start")
        );
    }

    #[test]
    fn command_execution_item_completed_maps_to_tool_result() {
        let mapped = map(
            r#"{"type":"item.completed","thread_id":"thread-1","turn_id":"turn-1","item_id":"item-3","item_type":"command_execution","content":{"command":"echo hi","stdout":"ok","stderr":"warn","exit_code":0}}"#,
        );
        assert_eq!(mapped.kind, AgentWrapperEventKind::ToolResult);
        assert_eq!(tool_schema(&mapped), Some(TOOLS_FACET_SCHEMA));
        assert_eq!(
            tool_field(&mapped, "phase").and_then(Value::as_str),
            Some("complete")
        );
        assert_eq!(
            tool_field(&mapped, "status").and_then(Value::as_str),
            Some("completed")
        );
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

    #[tokio::test]
    async fn codex_backend_enforces_timeout_while_draining_events() {
        let stop = Arc::new(AtomicBool::new(false));

        let events_stop = Arc::clone(&stop);
        let events = futures_util::stream::unfold((), move |_| {
            let events_stop = Arc::clone(&events_stop);
            async move {
                if events_stop.load(Ordering::SeqCst) {
                    None
                } else {
                    tokio::time::sleep(Duration::from_millis(5)).await;
                    Some((
                        Ok(parse_thread_event(
                            r#"{"type":"thread.started","thread_id":"thread-1"}"#,
                        )),
                        (),
                    ))
                }
            }
        });
        let events = Box::pin(events);

        let completion_stop = Arc::clone(&stop);
        let completion = async move {
            tokio::time::sleep(Duration::from_millis(30)).await;
            completion_stop.store(true, Ordering::SeqCst);
            Err(ExecStreamError::Codex(CodexError::Timeout {
                timeout: Duration::from_millis(10),
            }))
        };

        let (tx, mut rx) = mpsc::channel::<AgentWrapperEvent>(16);

        let outcome = tokio::time::timeout(
            Duration::from_millis(250),
            drain_events_while_polling_completion(events, completion, &tx),
        )
        .await
        .expect("should not hang")
        .expect("helper should not fail");

        match outcome {
            Err(ExecStreamError::Codex(CodexError::Timeout { .. })) => {}
            other => panic!("expected codex timeout, got {other:?}"),
        }

        assert!(
            rx.recv().await.is_some(),
            "should forward at least one event while draining"
        );
    }
}
