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
        if !forward {
            continue;
        }

        let mapped_events = match outcome {
            Ok(ev) => {
                if let claude_code::ClaudeStreamJsonEvent::AssistantMessage { raw, .. } = &ev {
                    if let Some(text) = extract_assistant_message_final_text(raw) {
                        last_assistant_text = Some(text);
                    }
                }
                map_stream_json_event(ev)
            }
            Err(err) => vec![error_event(redact_parse_error(&err))],
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
mod tests {
    use super::*;
    use crate::{AgentWrapperBackend, AgentWrapperEventKind};
    use claude_code::{ClaudeStreamJsonEvent, ClaudeStreamJsonParser};

    const SYSTEM_INIT: &str =
        include_str!("../../../claude_code/tests/fixtures/stream_json/v1/system_init.jsonl");
    const SYSTEM_OTHER: &str =
        include_str!("../../../claude_code/tests/fixtures/stream_json/v1/system_other.jsonl");
    const RESULT_ERROR: &str =
        include_str!("../../../claude_code/tests/fixtures/stream_json/v1/result_error.jsonl");
    const ASSISTANT_MESSAGE_TEXT: &str = include_str!(
        "../../../claude_code/tests/fixtures/stream_json/v1/assistant_message_text.jsonl"
    );
    const ASSISTANT_MESSAGE_TOOL_USE: &str = include_str!(
        "../../../claude_code/tests/fixtures/stream_json/v1/assistant_message_tool_use.jsonl"
    );
    const ASSISTANT_MESSAGE_TOOL_RESULT: &str = include_str!(
        "../../../claude_code/tests/fixtures/stream_json/v1/assistant_message_tool_result.jsonl"
    );
    const STREAM_EVENT_TEXT_DELTA: &str = include_str!(
        "../../../claude_code/tests/fixtures/stream_json/v1/stream_event_text_delta.jsonl"
    );
    const STREAM_EVENT_INPUT_JSON_DELTA: &str = include_str!(
        "../../../claude_code/tests/fixtures/stream_json/v1/stream_event_input_json_delta.jsonl"
    );
    const STREAM_EVENT_TOOL_USE_START: &str = include_str!(
        "../../../claude_code/tests/fixtures/stream_json/v1/stream_event_tool_use_start.jsonl"
    );
    const STREAM_EVENT_TOOL_RESULT_START: &str = include_str!(
        "../../../claude_code/tests/fixtures/stream_json/v1/stream_event_tool_result_start.jsonl"
    );
    const UNKNOWN_OUTER_TYPE: &str =
        include_str!("../../../claude_code/tests/fixtures/stream_json/v1/unknown_outer_type.jsonl");

    fn parse_stream_json_fixture(text: &str) -> ClaudeStreamJsonEvent {
        let line = text
            .lines()
            .find(|line| !line.chars().all(|ch| ch.is_whitespace()))
            .expect("fixture contains a non-empty line");
        let mut parser = ClaudeStreamJsonParser::new();
        parser
            .parse_line(line)
            .expect("fixture parses")
            .expect("fixture yields a typed event")
    }

    fn map_fixture(text: &str) -> AgentWrapperEvent {
        let event = parse_stream_json_fixture(text);
        let mapped = map_stream_json_event(event);
        assert_eq!(
            mapped.len(),
            1,
            "fixture should map to exactly one wrapper event"
        );
        mapped
            .into_iter()
            .next()
            .expect("fixture mapping returns at least one event")
    }

    #[test]
    fn claude_backend_reports_required_capabilities() {
        let backend = ClaudeCodeBackend::new(ClaudeCodeBackendConfig::default());
        let capabilities = backend.capabilities();
        assert!(capabilities.contains("agent_api.run"));
        assert!(capabilities.contains("agent_api.events"));
        assert!(capabilities.contains("agent_api.events.live"));
        assert!(capabilities.contains(CAP_TOOLS_STRUCTURED_V1));
        assert!(capabilities.contains(CAP_TOOLS_RESULTS_V1));
        assert!(capabilities.contains(CAP_ARTIFACTS_FINAL_TEXT_V1));
    }

    #[test]
    fn claude_backend_registers_under_claude_code_kind_id() {
        let backend = ClaudeCodeBackend::new(ClaudeCodeBackendConfig::default());
        assert_eq!(backend.kind().as_str(), "claude_code");
    }

    #[test]
    fn system_init_maps_to_status() {
        let mapped = map_fixture(SYSTEM_INIT);
        assert_eq!(mapped.agent_kind.as_str(), "claude_code");
        assert_eq!(mapped.kind, AgentWrapperEventKind::Status);
        assert_eq!(mapped.text, None);
    }

    #[test]
    fn system_other_maps_to_status() {
        let mapped = map_fixture(SYSTEM_OTHER);
        assert_eq!(mapped.kind, AgentWrapperEventKind::Status);
        assert_eq!(mapped.text, None);
    }

    #[test]
    fn result_error_maps_to_error_with_message() {
        let mapped = map_fixture(RESULT_ERROR);
        assert_eq!(mapped.kind, AgentWrapperEventKind::Error);
        assert_eq!(mapped.text, None);
        assert!(mapped.message.is_some());
    }

    #[test]
    fn assistant_message_text_maps_to_text_output_and_uses_text_field() {
        let mapped = map_fixture(ASSISTANT_MESSAGE_TEXT);
        assert_eq!(mapped.kind, AgentWrapperEventKind::TextOutput);
        assert_eq!(mapped.text.as_deref(), Some("hello"));
        assert_eq!(mapped.message, None);
    }

    #[test]
    fn assistant_message_tool_use_maps_to_tool_call() {
        let mapped = map_fixture(ASSISTANT_MESSAGE_TOOL_USE);
        assert_eq!(mapped.kind, AgentWrapperEventKind::ToolCall);
        assert_eq!(mapped.text, None);
        assert_eq!(mapped.message, None);
        assert_eq!(mapped.channel.as_deref(), Some(CHANNEL_TOOL));
        assert!(mapped.data.is_some());
        assert_eq!(
            mapped
                .data
                .as_ref()
                .and_then(|v| v.get("schema"))
                .and_then(|v| v.as_str()),
            Some(CAP_TOOLS_STRUCTURED_V1)
        );
        assert_eq!(
            mapped
                .data
                .as_ref()
                .and_then(|v| v.pointer("/tool/kind"))
                .and_then(|v| v.as_str()),
            Some("tool_use")
        );
        assert_eq!(
            mapped
                .data
                .as_ref()
                .and_then(|v| v.pointer("/tool/phase"))
                .and_then(|v| v.as_str()),
            Some("start")
        );
        assert_eq!(
            mapped
                .data
                .as_ref()
                .and_then(|v| v.pointer("/tool/status"))
                .and_then(|v| v.as_str()),
            Some("running")
        );
        assert_eq!(
            mapped
                .data
                .as_ref()
                .and_then(|v| v.pointer("/tool/tool_name"))
                .and_then(|v| v.as_str()),
            Some("bash")
        );
        assert_eq!(
            mapped
                .data
                .as_ref()
                .and_then(|v| v.pointer("/tool/tool_use_id"))
                .and_then(|v| v.as_str()),
            Some("t1")
        );
    }

    #[test]
    fn assistant_message_tool_result_maps_to_tool_result() {
        let mapped = map_fixture(ASSISTANT_MESSAGE_TOOL_RESULT);
        assert_eq!(mapped.kind, AgentWrapperEventKind::ToolResult);
        assert_eq!(mapped.text, None);
        assert_eq!(mapped.message, None);
        assert_eq!(mapped.channel.as_deref(), Some(CHANNEL_TOOL));
        assert!(mapped.data.is_some());
        assert_eq!(
            mapped
                .data
                .as_ref()
                .and_then(|v| v.pointer("/tool/kind"))
                .and_then(|v| v.as_str()),
            Some("tool_result")
        );
        assert_eq!(
            mapped
                .data
                .as_ref()
                .and_then(|v| v.pointer("/tool/phase"))
                .and_then(|v| v.as_str()),
            Some("complete")
        );
        assert_eq!(
            mapped
                .data
                .as_ref()
                .and_then(|v| v.pointer("/tool/status"))
                .and_then(|v| v.as_str()),
            Some("completed")
        );
        assert_eq!(
            mapped
                .data
                .as_ref()
                .and_then(|v| v.pointer("/tool/tool_use_id"))
                .and_then(|v| v.as_str()),
            Some("t1")
        );
    }

    #[test]
    fn stream_event_text_delta_maps_to_text_output_and_uses_text_field() {
        let mapped = map_fixture(STREAM_EVENT_TEXT_DELTA);
        assert_eq!(mapped.kind, AgentWrapperEventKind::TextOutput);
        assert_eq!(mapped.text.as_deref(), Some("hel"));
        assert_eq!(mapped.message, None);
    }

    #[test]
    fn stream_event_input_json_delta_maps_to_tool_call() {
        let mapped = map_fixture(STREAM_EVENT_INPUT_JSON_DELTA);
        assert_eq!(mapped.kind, AgentWrapperEventKind::ToolCall);
        assert_eq!(mapped.text, None);
        assert_eq!(mapped.message, None);
        assert_eq!(mapped.channel.as_deref(), Some(CHANNEL_TOOL));
        assert!(mapped.data.is_some());
        assert_eq!(
            mapped
                .data
                .as_ref()
                .and_then(|v| v.pointer("/tool/kind"))
                .and_then(|v| v.as_str()),
            Some("tool_use")
        );
        assert_eq!(
            mapped
                .data
                .as_ref()
                .and_then(|v| v.pointer("/tool/phase"))
                .and_then(|v| v.as_str()),
            Some("delta")
        );
        assert_eq!(
            mapped
                .data
                .as_ref()
                .and_then(|v| v.pointer("/tool/status"))
                .and_then(|v| v.as_str()),
            Some("running")
        );
        assert!(mapped
            .data
            .as_ref()
            .and_then(|v| v.pointer("/tool/tool_name"))
            .is_some_and(|v| v.is_null()));
        assert!(mapped
            .data
            .as_ref()
            .and_then(|v| v.pointer("/tool/tool_use_id"))
            .is_some_and(|v| v.is_null()));
    }

    #[test]
    fn stream_event_tool_use_start_maps_to_tool_call() {
        let mapped = map_fixture(STREAM_EVENT_TOOL_USE_START);
        assert_eq!(mapped.kind, AgentWrapperEventKind::ToolCall);
        assert_eq!(mapped.channel.as_deref(), Some(CHANNEL_TOOL));
        assert!(mapped.data.is_some());
        assert_eq!(
            mapped
                .data
                .as_ref()
                .and_then(|v| v.pointer("/tool/tool_name"))
                .and_then(|v| v.as_str()),
            Some("bash")
        );
        assert_eq!(
            mapped
                .data
                .as_ref()
                .and_then(|v| v.pointer("/tool/tool_use_id"))
                .and_then(|v| v.as_str()),
            Some("t1")
        );
    }

    #[test]
    fn stream_event_tool_result_start_maps_to_tool_result() {
        let mapped = map_fixture(STREAM_EVENT_TOOL_RESULT_START);
        assert_eq!(mapped.kind, AgentWrapperEventKind::ToolResult);
        assert_eq!(mapped.channel.as_deref(), Some(CHANNEL_TOOL));
        assert!(mapped.data.is_some());
        assert_eq!(
            mapped
                .data
                .as_ref()
                .and_then(|v| v.pointer("/tool/tool_use_id"))
                .and_then(|v| v.as_str()),
            Some("t1")
        );
    }

    #[test]
    fn assistant_message_tool_use_missing_name_and_id_emits_tool_call_with_null_tool_ids() {
        let raw = serde_json::json!({
            "message": {
                "content": [
                    { "type": "tool_use" }
                ]
            }
        });
        let mapped = map_assistant_message(&raw);
        assert_eq!(mapped.len(), 1);
        assert_eq!(mapped[0].kind, AgentWrapperEventKind::ToolCall);
        assert_eq!(mapped[0].channel.as_deref(), Some(CHANNEL_TOOL));
        assert!(mapped[0].data.is_some());
        assert!(mapped[0]
            .data
            .as_ref()
            .and_then(|v| v.pointer("/tool/tool_name"))
            .is_some_and(|v| v.is_null()));
        assert!(mapped[0]
            .data
            .as_ref()
            .and_then(|v| v.pointer("/tool/tool_use_id"))
            .is_some_and(|v| v.is_null()));
    }

    #[test]
    fn stream_event_tool_result_start_with_non_string_tool_use_id_emits_tool_result_with_null_id() {
        let raw = serde_json::json!({
            "type": "content_block_start",
            "content_block": {
                "type": "tool_result",
                "tool_use_id": 123
            }
        });
        let mapped = map_stream_event(&raw);
        assert_eq!(mapped.len(), 1);
        assert_eq!(mapped[0].kind, AgentWrapperEventKind::ToolResult);
        assert_eq!(mapped[0].channel.as_deref(), Some(CHANNEL_TOOL));
        assert!(mapped[0].data.is_some());
        assert!(mapped[0]
            .data
            .as_ref()
            .and_then(|v| v.pointer("/tool/tool_use_id"))
            .is_some_and(|v| v.is_null()));
    }

    #[test]
    fn stream_event_unknown_type_maps_to_unknown() {
        let raw = serde_json::json!({
            "type": "new_stream_event_type",
            "foo": "bar",
        });
        let mapped = map_stream_event(&raw);
        assert_eq!(mapped.len(), 1);
        assert_eq!(mapped[0].kind, AgentWrapperEventKind::Unknown);
    }

    #[test]
    fn stream_event_content_block_start_unknown_block_type_maps_to_unknown() {
        let raw = serde_json::json!({
            "type": "content_block_start",
            "content_block": { "type": "new_block_type" },
        });
        let mapped = map_stream_event(&raw);
        assert_eq!(mapped.len(), 1);
        assert_eq!(mapped[0].kind, AgentWrapperEventKind::Unknown);
    }

    #[test]
    fn unknown_outer_type_maps_to_unknown() {
        let mapped = map_fixture(UNKNOWN_OUTER_TYPE);
        assert_eq!(mapped.kind, AgentWrapperEventKind::Unknown);
        assert_eq!(mapped.text, None);
    }
}
