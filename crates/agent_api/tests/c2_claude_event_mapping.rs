#![cfg(feature = "claude_code")]

use agent_api::{AgentWrapperBackend, AgentWrapperEvent, AgentWrapperEventKind};
use claude_code::{ClaudeStreamJsonEvent, ClaudeStreamJsonParser};

const SYSTEM_INIT: &str =
    include_str!("../../claude_code/tests/fixtures/stream_json/v1/system_init.jsonl");
const SYSTEM_OTHER: &str =
    include_str!("../../claude_code/tests/fixtures/stream_json/v1/system_other.jsonl");
const RESULT_ERROR: &str =
    include_str!("../../claude_code/tests/fixtures/stream_json/v1/result_error.jsonl");
const ASSISTANT_MESSAGE_TEXT: &str =
    include_str!("../../claude_code/tests/fixtures/stream_json/v1/assistant_message_text.jsonl");
const ASSISTANT_MESSAGE_TOOL_USE: &str = include_str!(
    "../../claude_code/tests/fixtures/stream_json/v1/assistant_message_tool_use.jsonl"
);
const ASSISTANT_MESSAGE_TOOL_RESULT: &str = include_str!(
    "../../claude_code/tests/fixtures/stream_json/v1/assistant_message_tool_result.jsonl"
);
const STREAM_EVENT_TEXT_DELTA: &str =
    include_str!("../../claude_code/tests/fixtures/stream_json/v1/stream_event_text_delta.jsonl");
const STREAM_EVENT_INPUT_JSON_DELTA: &str = include_str!(
    "../../claude_code/tests/fixtures/stream_json/v1/stream_event_input_json_delta.jsonl"
);
const STREAM_EVENT_TOOL_USE_START: &str = include_str!(
    "../../claude_code/tests/fixtures/stream_json/v1/stream_event_tool_use_start.jsonl"
);
const STREAM_EVENT_TOOL_RESULT_START: &str = include_str!(
    "../../claude_code/tests/fixtures/stream_json/v1/stream_event_tool_result_start.jsonl"
);
const UNKNOWN_OUTER_TYPE: &str =
    include_str!("../../claude_code/tests/fixtures/stream_json/v1/unknown_outer_type.jsonl");

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
    let mapped = agent_api::backends::claude_code::map_stream_json_event(event);
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
    let backend = agent_api::backends::claude_code::ClaudeCodeBackend::new(
        agent_api::backends::claude_code::ClaudeCodeBackendConfig::default(),
    );
    let capabilities = backend.capabilities();
    assert!(capabilities.contains("agent_api.run"));
    assert!(capabilities.contains("agent_api.events"));
    assert!(!capabilities.contains("agent_api.events.live"));
}

#[test]
fn claude_backend_registers_under_claude_code_kind_id() {
    let backend = agent_api::backends::claude_code::ClaudeCodeBackend::new(
        agent_api::backends::claude_code::ClaudeCodeBackendConfig::default(),
    );
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
