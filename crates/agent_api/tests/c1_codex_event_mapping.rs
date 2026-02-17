#![cfg(feature = "codex")]

use agent_api::{AgentWrapperBackend, AgentWrapperEvent, AgentWrapperEventKind};

fn parse_thread_event(json: &str) -> codex::ThreadEvent {
    serde_json::from_str(json).expect("valid codex::ThreadEvent JSON")
}

fn map(json: &str) -> AgentWrapperEvent {
    let event = parse_thread_event(json);
    agent_api::backends::codex::map_thread_event(&event)
}

#[test]
fn codex_backend_reports_required_capabilities() {
    let backend = agent_api::backends::codex::CodexBackend::new(
        agent_api::backends::codex::CodexBackendConfig::default(),
    );
    let capabilities = backend.capabilities();
    assert!(capabilities.contains("agent_api.run"));
    assert!(capabilities.contains("agent_api.events"));
    assert!(capabilities.contains("agent_api.events.live"));
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
fn item_failed_maps_to_error_with_message() {
    let mapped = map(
        r#"{"type":"item.failed","thread_id":"thread-1","turn_id":"turn-1","item_id":"item-1","error":{"message":"tool failed"}}"#,
    );
    assert_eq!(mapped.kind, AgentWrapperEventKind::Error);
    assert_eq!(mapped.text, None);
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
