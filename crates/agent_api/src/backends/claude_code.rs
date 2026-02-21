use std::{
    collections::{BTreeMap, BTreeSet},
    future::Future,
    path::PathBuf,
    pin::Pin,
    time::Duration,
};

use claude_code::{ClaudeOutputFormat, ClaudePrintRequest};
use futures_util::StreamExt;
use serde_json::Value;
use tokio::sync::{mpsc, oneshot};

use crate::{
    AgentWrapperBackend, AgentWrapperCapabilities, AgentWrapperCompletion, AgentWrapperError,
    AgentWrapperEvent, AgentWrapperEventKind, AgentWrapperKind, AgentWrapperRunHandle,
    AgentWrapperRunRequest,
};

const AGENT_KIND: &str = "claude_code";
const CHANNEL_ASSISTANT: &str = "assistant";
const CHANNEL_TOOL: &str = "tool";

const EXT_NON_INTERACTIVE: &str = "agent_api.exec.non_interactive";

const CAP_TOOLS_STRUCTURED_V1: &str = "agent_api.tools.structured.v1";
const CAP_TOOLS_RESULTS_V1: &str = "agent_api.tools.results.v1";
const CAP_ARTIFACTS_FINAL_TEXT_V1: &str = "agent_api.artifacts.final_text.v1";

fn parse_bool(value: &Value, key: &str) -> Result<bool, AgentWrapperError> {
    value
        .as_bool()
        .ok_or_else(|| AgentWrapperError::InvalidRequest {
            message: format!("{key} must be a boolean"),
        })
}

fn validate_and_extract_non_interactive(
    request: &AgentWrapperRunRequest,
) -> Result<bool, AgentWrapperError> {
    for key in request.extensions.keys() {
        if key != EXT_NON_INTERACTIVE {
            return Err(AgentWrapperError::UnsupportedCapability {
                agent_kind: AGENT_KIND.to_string(),
                capability: key.clone(),
            });
        }
    }

    Ok(request
        .extensions
        .get(EXT_NON_INTERACTIVE)
        .map(|value| parse_bool(value, EXT_NON_INTERACTIVE))
        .transpose()?
        .unwrap_or(true))
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
        ids.insert("agent_api.events.live".to_string());
        ids.insert(CAP_TOOLS_STRUCTURED_V1.to_string());
        ids.insert(CAP_TOOLS_RESULTS_V1.to_string());
        ids.insert(CAP_ARTIFACTS_FINAL_TEXT_V1.to_string());
        ids.insert("backend.claude_code.print_stream_json".to_string());
        ids.insert(EXT_NON_INTERACTIVE.to_string());
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
    config: ClaudeCodeBackendConfig,
    request: AgentWrapperRunRequest,
) -> Result<AgentWrapperRunHandle, AgentWrapperError> {
    if request.prompt.trim().is_empty() {
        return Err(AgentWrapperError::InvalidRequest {
            message: "prompt must not be empty".to_string(),
        });
    }

    let non_interactive = validate_and_extract_non_interactive(&request)?;

    let (tx, rx) = mpsc::channel::<AgentWrapperEvent>(32);
    let (completion_tx, completion_rx) =
        oneshot::channel::<Result<AgentWrapperCompletion, AgentWrapperError>>();

    tokio::spawn(async move {
        let result = run_claude_code(config, request, non_interactive, tx).await;
        let _ = completion_tx.send(result);
    });

    Ok(crate::run_handle_gate::build_gated_run_handle(
        rx,
        completion_rx,
    ))
}

async fn run_claude_code(
    config: ClaudeCodeBackendConfig,
    request: AgentWrapperRunRequest,
    non_interactive: bool,
    tx: mpsc::Sender<AgentWrapperEvent>,
) -> Result<AgentWrapperCompletion, AgentWrapperError> {
    let timeout = request.timeout.or(config.default_timeout);
    if let Some(timeout) = timeout {
        return tokio::time::timeout(
            timeout,
            run_claude_code_inner(config, request, non_interactive, tx),
        )
        .await
        .map_err(|_| AgentWrapperError::Backend {
            message: format!("claude_code exceeded timeout of {timeout:?}"),
        })?;
    }

    run_claude_code_inner(config, request, non_interactive, tx).await
}

async fn run_claude_code_inner(
    config: ClaudeCodeBackendConfig,
    request: AgentWrapperRunRequest,
    non_interactive: bool,
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
    let mut print_req = ClaudePrintRequest::new(request.prompt)
        .output_format(ClaudeOutputFormat::StreamJson)
        .include_partial_messages(true);
    if non_interactive {
        print_req = print_req.permission_mode("bypassPermissions");
    }

    let handle = match client.print_stream_json(print_req).await {
        Ok(handle) => handle,
        Err(err) => {
            for bounded in crate::bounds::enforce_event_bounds(error_event(format!(
                "claude_code error: {err}"
            ))) {
                if tx.send(bounded).await.is_err() {
                    break;
                }
            }
            drop(tx);
            return Err(AgentWrapperError::Backend {
                message: format!("claude_code error: {err}"),
            });
        }
    };

    let mut events = handle.events;
    let completion = handle.completion;

    let mut last_assistant_text: Option<String> = None;

    // If the caller drops the universal events stream, we MUST keep draining the backend stream so
    // the underlying process isn't accidentally cancelled (and so we avoid deadlocks on bounded
    // channels). We simply stop forwarding once the receiver is gone.
    let mut forward = true;
    while let Some(outcome) = events.next().await {
        match outcome {
            Ok(ev) => {
                if let claude_code::ClaudeStreamJsonEvent::AssistantMessage { raw, .. } = &ev {
                    if let Some(text) = extract_assistant_message_final_text(raw) {
                        last_assistant_text = Some(text);
                    }
                }

                if !forward {
                    continue;
                }

                for event in map_stream_json_event(ev) {
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
            Err(err) => {
                if !forward {
                    continue;
                }

                for bounded in crate::bounds::enforce_event_bounds(error_event(redact_parse_error(
                    &err,
                ))) {
                    if tx.send(bounded).await.is_err() {
                        forward = false;
                        break;
                    }
                }
            }
        }
    }

    let status = match completion.await {
        Ok(status) => status,
        Err(err) => {
            for bounded in crate::bounds::enforce_event_bounds(error_event(format!(
                "claude_code error: {err}"
            ))) {
                let _ = tx.send(bounded).await;
            }
            drop(tx);
            return Err(AgentWrapperError::Backend {
                message: format!("claude_code error: {err}"),
            });
        }
    };

    drop(tx);

    Ok(crate::bounds::enforce_completion_bounds(
        AgentWrapperCompletion {
            status,
            final_text: crate::bounds::enforce_final_text_bound(last_assistant_text),
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

#[cfg(test)]
#[path = "claude_code/tests.rs"]
mod tests;
