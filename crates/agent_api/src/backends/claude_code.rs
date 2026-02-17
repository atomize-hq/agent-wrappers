use std::{
    collections::{BTreeMap, BTreeSet},
    future::Future,
    path::PathBuf,
    pin::Pin,
    time::Duration,
};

use claude_code::{ClaudeOutputFormat, ClaudePrintRequest, ClaudeStreamJsonParser};
use tokio::sync::{mpsc, oneshot};

use crate::{
    AgentWrapperBackend, AgentWrapperCapabilities, AgentWrapperCompletion, AgentWrapperError,
    AgentWrapperEvent, AgentWrapperEventKind, AgentWrapperKind, AgentWrapperRunHandle,
    AgentWrapperRunRequest,
};

const AGENT_KIND: &str = "claude_code";
const CHANNEL_ASSISTANT: &str = "assistant";
const CHANNEL_TOOL: &str = "tool";

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
        ids.insert("backend.claude_code.print_stream_json".to_string());
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

    if let Some((capability, _)) = request.extensions.iter().next() {
        return Err(AgentWrapperError::UnsupportedCapability {
            agent_kind: AGENT_KIND.to_string(),
            capability: capability.clone(),
        });
    }

    let (tx, rx) = mpsc::channel::<AgentWrapperEvent>(32);
    let (completion_tx, completion_rx) =
        oneshot::channel::<Result<AgentWrapperCompletion, AgentWrapperError>>();

    tokio::spawn(async move {
        let result = run_claude_code(config, request, tx).await;
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
    tx: mpsc::Sender<AgentWrapperEvent>,
) -> Result<AgentWrapperCompletion, AgentWrapperError> {
    let timeout = request.timeout.or(config.default_timeout);
    if let Some(timeout) = timeout {
        return tokio::time::timeout(timeout, run_claude_code_inner(config, request, tx))
            .await
            .map_err(|_| AgentWrapperError::Backend {
                message: format!("claude_code exceeded timeout of {timeout:?}"),
            })?;
    }

    run_claude_code_inner(config, request, tx).await
}

async fn run_claude_code_inner(
    config: ClaudeCodeBackendConfig,
    request: AgentWrapperRunRequest,
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
    let print_req = ClaudePrintRequest::new(request.prompt)
        .output_format(ClaudeOutputFormat::StreamJson)
        .include_partial_messages(true);

    let res = match client.print(print_req).await {
        Ok(res) => res,
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

    let stdout = String::from_utf8_lossy(&res.output.stdout);
    let mut parser = ClaudeStreamJsonParser::new();

    for line in stdout.lines() {
        match parser.parse_line(line) {
            Ok(None) => {}
            Ok(Some(ev)) => {
                let mapped = map_stream_json_event(ev);
                for event in mapped {
                    for bounded in crate::bounds::enforce_event_bounds(event) {
                        if tx.send(bounded).await.is_err() {
                            break;
                        }
                    }
                }
            }
            Err(err) => {
                for bounded in
                    crate::bounds::enforce_event_bounds(error_event(redact_parse_error(&err)))
                {
                    if tx.send(bounded).await.is_err() {
                        break;
                    }
                }
            }
        }
    }

    drop(tx);

    Ok(crate::bounds::enforce_completion_bounds(
        AgentWrapperCompletion {
            status: res.output.status,
            final_text: None,
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
            "tool_use" => out.push(tool_call_event()),
            "tool_result" => out.push(tool_result_event()),
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
                "tool_use" => vec![tool_call_event()],
                "tool_result" => vec![tool_result_event()],
                _ => vec![status_event(None)],
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
                "input_json_delta" => vec![tool_call_event()],
                _ => vec![unknown_event()],
            }
        }
        _ => vec![status_event(None)],
    }
}

fn tool_call_event() -> AgentWrapperEvent {
    AgentWrapperEvent {
        agent_kind: AgentWrapperKind(AGENT_KIND.to_string()),
        kind: AgentWrapperEventKind::ToolCall,
        channel: Some(CHANNEL_TOOL.to_string()),
        text: None,
        message: None,
        data: None,
    }
}

fn tool_result_event() -> AgentWrapperEvent {
    AgentWrapperEvent {
        agent_kind: AgentWrapperKind(AGENT_KIND.to_string()),
        kind: AgentWrapperEventKind::ToolResult,
        channel: Some(CHANNEL_TOOL.to_string()),
        text: None,
        message: None,
        data: None,
    }
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
        assert!(!capabilities.contains("agent_api.events.live"));
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
    }

    #[test]
    fn assistant_message_tool_result_maps_to_tool_result() {
        let mapped = map_fixture(ASSISTANT_MESSAGE_TOOL_RESULT);
        assert_eq!(mapped.kind, AgentWrapperEventKind::ToolResult);
        assert_eq!(mapped.text, None);
        assert_eq!(mapped.message, None);
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
    }

    #[test]
    fn stream_event_tool_use_start_maps_to_tool_call() {
        let mapped = map_fixture(STREAM_EVENT_TOOL_USE_START);
        assert_eq!(mapped.kind, AgentWrapperEventKind::ToolCall);
    }

    #[test]
    fn stream_event_tool_result_start_maps_to_tool_result() {
        let mapped = map_fixture(STREAM_EVENT_TOOL_RESULT_START);
        assert_eq!(mapped.kind, AgentWrapperEventKind::ToolResult);
    }

    #[test]
    fn unknown_outer_type_maps_to_unknown() {
        let mapped = map_fixture(UNKNOWN_OUTER_TYPE);
        assert_eq!(mapped.kind, AgentWrapperEventKind::Unknown);
        assert_eq!(mapped.text, None);
    }
}
