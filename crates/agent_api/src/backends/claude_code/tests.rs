use super::*;
use crate::{AgentWrapperBackend, AgentWrapperEventKind};
use claude_code::{ClaudeStreamJsonEvent, ClaudeStreamJsonParser};

const SYSTEM_INIT: &str =
    include_str!("../../../../claude_code/tests/fixtures/stream_json/v1/system_init.jsonl");
const SYSTEM_OTHER: &str =
    include_str!("../../../../claude_code/tests/fixtures/stream_json/v1/system_other.jsonl");
const RESULT_ERROR: &str =
    include_str!("../../../../claude_code/tests/fixtures/stream_json/v1/result_error.jsonl");
const ASSISTANT_MESSAGE_TEXT: &str = include_str!(
    "../../../../claude_code/tests/fixtures/stream_json/v1/assistant_message_text.jsonl"
);
const ASSISTANT_MESSAGE_TOOL_USE: &str = include_str!(
    "../../../../claude_code/tests/fixtures/stream_json/v1/assistant_message_tool_use.jsonl"
);
const ASSISTANT_MESSAGE_TOOL_RESULT: &str = include_str!(
    "../../../../claude_code/tests/fixtures/stream_json/v1/assistant_message_tool_result.jsonl"
);
const STREAM_EVENT_TEXT_DELTA: &str = include_str!(
    "../../../../claude_code/tests/fixtures/stream_json/v1/stream_event_text_delta.jsonl"
);
const STREAM_EVENT_INPUT_JSON_DELTA: &str = include_str!(
    "../../../../claude_code/tests/fixtures/stream_json/v1/stream_event_input_json_delta.jsonl"
);
const STREAM_EVENT_TOOL_USE_START: &str = include_str!(
    "../../../../claude_code/tests/fixtures/stream_json/v1/stream_event_tool_use_start.jsonl"
);
const STREAM_EVENT_TOOL_RESULT_START: &str = include_str!(
    "../../../../claude_code/tests/fixtures/stream_json/v1/stream_event_tool_result_start.jsonl"
);
const UNKNOWN_OUTER_TYPE: &str =
    include_str!("../../../../claude_code/tests/fixtures/stream_json/v1/unknown_outer_type.jsonl");

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
