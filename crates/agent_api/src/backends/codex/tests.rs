use super::*;
use crate::{AgentWrapperBackend, AgentWrapperEventKind};
use codex::ThreadEvent;
use serde_json::Value;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;
use tokio::sync::mpsc;

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
