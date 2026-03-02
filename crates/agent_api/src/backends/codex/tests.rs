use super::*;
use crate::{AgentWrapperBackend, AgentWrapperEventKind};
use codex::ThreadEvent;
use serde_json::{json, Value};

use super::super::session_selectors::EXT_SESSION_FORK_V1;

fn success_exit_status() -> std::process::ExitStatus {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        std::process::ExitStatus::from_raw(0)
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::ExitStatusExt;
        std::process::ExitStatus::from_raw(0)
    }
}

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

fn handle_schema(event: &AgentWrapperEvent) -> Option<&str> {
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
    assert!(capabilities.contains(crate::CAPABILITY_CONTROL_CANCEL_V1));
    assert!(capabilities.contains(CAP_TOOLS_STRUCTURED_V1));
    assert!(capabilities.contains(CAP_TOOLS_RESULTS_V1));
    assert!(capabilities.contains(CAP_ARTIFACTS_FINAL_TEXT_V1));
    assert!(capabilities.contains(CAP_SESSION_HANDLE_V1));
    assert!(capabilities.contains("backend.codex.exec_stream"));
    assert!(capabilities.contains(EXT_NON_INTERACTIVE));
    assert!(capabilities.contains(EXT_CODEX_APPROVAL_POLICY));
    assert!(capabilities.contains(EXT_CODEX_SANDBOX_MODE));
    assert!(capabilities.contains(EXT_SESSION_RESUME_V1));
}

#[test]
fn codex_adapter_implements_backend_harness_adapter_contract() {
    fn assert_impl<T: crate::backend_harness::BackendHarnessAdapter>() {}
    assert_impl::<CodexHarnessAdapter>();
}

#[test]
fn codex_backend_routes_through_harness_and_does_not_reintroduce_orchestration_primitives() {
    const SOURCE: &str = include_str!("../codex.rs");

    assert!(
        SOURCE.contains("run_harnessed_backend("),
        "expected Codex backend to route through the harness entrypoint"
    );
    assert!(
        SOURCE.contains("run_harnessed_backend_control("),
        "expected Codex backend to route cancellation through the harness control entrypoint"
    );
    assert!(
        SOURCE.contains("TerminationState::new"),
        "expected Codex backend control path to register a termination hook"
    );

    assert!(
        !SOURCE.contains("build_gated_run_handle("),
        "expected Codex backend to not bypass harness-owned completion gating"
    );
    assert!(
        !SOURCE.contains("mpsc::channel::<AgentWrapperEvent>(32)"),
        "expected Codex backend to not create a backend-local events channel"
    );
    assert!(
        !SOURCE.contains("tokio::time::timeout("),
        "expected Codex backend to not wrap runs with backend-local timeout orchestration"
    );
}

#[test]
fn redact_exec_stream_error_does_not_leak_raw_jsonl_line() {
    let secret = "RAW-LINE-SECRET-DO-NOT-LEAK";
    let err = ExecStreamError::Normalize {
        line: secret.to_string(),
        message: "missing required context".to_string(),
    };

    let redacted = redact_exec_stream_error(&err);
    assert!(
        !redacted.contains(secret),
        "expected redaction to avoid raw JSONL line content"
    );
    assert!(
        redacted.contains("line_bytes="),
        "expected `line_bytes=<n>` metadata"
    );
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

fn test_adapter() -> CodexHarnessAdapter {
    CodexHarnessAdapter {
        config: CodexBackendConfig::default(),
        run_start_cwd: None,
        termination: None,
        handle_state: std::sync::Arc::new(std::sync::Mutex::new(CodexHandleFacetState::default())),
    }
}

#[test]
fn fork_selector_is_extracted_into_policy_when_validate_and_extract_policy_is_called_directly() {
    let adapter = test_adapter();
    let mut request = AgentWrapperRunRequest {
        prompt: "hello".to_string(),
        ..Default::default()
    };
    request
        .extensions
        .insert(EXT_SESSION_FORK_V1.to_string(), json!({"selector": "last"}));

    let policy = adapter
        .validate_and_extract_policy(&request)
        .expect("expected policy extraction to succeed");
    assert_eq!(policy.fork, Some(SessionSelectorV1::Last));
}

#[test]
fn fork_key_is_supported_via_backend_harness_normalize_request() {
    let adapter = test_adapter();
    let defaults = BackendDefaults::default();
    let mut request = AgentWrapperRunRequest {
        prompt: "hello".to_string(),
        ..Default::default()
    };
    request
        .extensions
        .insert(EXT_SESSION_FORK_V1.to_string(), json!({"selector": "last"}));

    let normalized = crate::backend_harness::normalize_request(&adapter, &defaults, request)
        .expect("expected fork extension key to be supported in S4b");
    assert_eq!(normalized.policy.fork, Some(SessionSelectorV1::Last));
}

#[test]
fn handle_facet_emitted_once_on_thread_started_and_attached_to_completion() {
    let adapter = test_adapter();

    let first = adapter.map_event(CodexBackendEvent::Thread(Box::new(parse_thread_event(
        r#"{"type":"thread.started","thread_id":"thread-1"}"#,
    ))));
    let second = adapter.map_event(CodexBackendEvent::Thread(Box::new(parse_thread_event(
        r#"{"type":"turn.started","thread_id":"thread-1","turn_id":"turn-1"}"#,
    ))));

    let seen: Vec<AgentWrapperEvent> = first.into_iter().chain(second).collect();
    let handle_events: Vec<&AgentWrapperEvent> = seen
        .iter()
        .filter(|ev| ev.kind == AgentWrapperEventKind::Status)
        .filter(|ev| handle_schema(ev) == Some(CAP_SESSION_HANDLE_V1))
        .collect();
    assert_eq!(handle_events.len(), 1);
    assert_eq!(handle_schema(&seen[0]), Some(CAP_SESSION_HANDLE_V1));
    assert_eq!(
        seen[0]
            .data
            .as_ref()
            .and_then(|data| data.get("session"))
            .and_then(|session| session.get("id"))
            .and_then(Value::as_str),
        Some("thread-1")
    );

    let completion = adapter
        .map_completion(CodexBackendCompletion {
            status: success_exit_status(),
            final_text: None,
            selection_failure_message: None,
        })
        .expect("completion maps");
    assert_eq!(
        completion
            .data
            .as_ref()
            .and_then(|data| data.get("schema"))
            .and_then(Value::as_str),
        Some(CAP_SESSION_HANDLE_V1)
    );
    assert_eq!(
        completion
            .data
            .as_ref()
            .and_then(|data| data.get("session"))
            .and_then(|session| session.get("id"))
            .and_then(Value::as_str),
        Some("thread-1")
    );
}

#[test]
fn whitespace_thread_id_is_treated_as_unknown() {
    let adapter = test_adapter();

    let seen = adapter.map_event(CodexBackendEvent::Thread(Box::new(parse_thread_event(
        r#"{"type":"thread.started","thread_id":"   "}"#,
    ))));
    assert!(
        !seen
            .iter()
            .any(|ev| handle_schema(ev) == Some(CAP_SESSION_HANDLE_V1)),
        "expected no handle facet emission for whitespace-only thread ids"
    );

    let completion = adapter
        .map_completion(CodexBackendCompletion {
            status: success_exit_status(),
            final_text: None,
            selection_failure_message: None,
        })
        .expect("completion maps");
    assert_eq!(completion.data, None);
}

#[test]
fn oversize_thread_id_is_omitted_and_warns_once() {
    let adapter = test_adapter();
    let oversize = "a".repeat(SESSION_HANDLE_ID_BOUND_BYTES + 1);
    let json = format!(r#"{{"type":"thread.started","thread_id":"{oversize}"}}"#);

    let first = adapter.map_event(CodexBackendEvent::Thread(Box::new(parse_thread_event(
        &json,
    ))));
    let second = adapter.map_event(CodexBackendEvent::Thread(Box::new(parse_thread_event(
        &json,
    ))));

    let seen: Vec<AgentWrapperEvent> = first.into_iter().chain(second).collect();
    assert!(
        !seen
            .iter()
            .any(|ev| handle_schema(ev) == Some(CAP_SESSION_HANDLE_V1)),
        "expected oversize ids to be treated as unknown (no facet emission)"
    );

    let warnings: Vec<&AgentWrapperEvent> = seen
        .iter()
        .filter(|ev| ev.kind == AgentWrapperEventKind::Status)
        .filter(|ev| {
            ev.message.as_deref().is_some_and(|message| {
                message.contains(SESSION_HANDLE_OVERSIZE_WARNING_MARKER)
                    && message.contains("len_bytes=1025")
                    && !message.contains(&oversize)
            })
        })
        .collect();
    assert_eq!(warnings.len(), 1);

    let completion = adapter
        .map_completion(CodexBackendCompletion {
            status: success_exit_status(),
            final_text: None,
            selection_failure_message: None,
        })
        .expect("completion maps");
    assert_eq!(completion.data, None);
}

#[test]
fn synthetic_status_is_emitted_if_id_first_seen_on_non_status_event() {
    let adapter = test_adapter();

    let seen = adapter.map_event(CodexBackendEvent::Thread(Box::new(parse_thread_event(
        r#"{"type":"item.started","thread_id":"thread-1","turn_id":"turn-1","item_id":"item-1","item_type":"command_execution","content":{"command":"echo hi"}}"#,
    ))));

    assert_eq!(seen.len(), 2);
    assert_eq!(seen[0].kind, AgentWrapperEventKind::ToolCall);
    assert_eq!(tool_schema(&seen[0]), Some(TOOLS_FACET_SCHEMA));
    assert_eq!(seen[1].kind, AgentWrapperEventKind::Status);
    assert_eq!(handle_schema(&seen[1]), Some(CAP_SESSION_HANDLE_V1));
    assert_eq!(
        seen[1]
            .data
            .as_ref()
            .and_then(|data| data.get("session"))
            .and_then(|session| session.get("id"))
            .and_then(Value::as_str),
        Some("thread-1")
    );
}

#[test]
fn app_server_agent_message_delta_maps_to_text_output_without_data() {
    let event =
        fork::map_app_server_notification("agentMessage/delta", &json!("hello")).expect("mapped");
    assert_eq!(event.kind, AgentWrapperEventKind::TextOutput);
    assert_eq!(event.channel.as_deref(), Some("assistant"));
    assert_eq!(event.text.as_deref(), Some("hello"));
    assert_eq!(event.data, None);
}

#[test]
fn app_server_reasoning_delta_prefers_content_text_field_when_present() {
    let params = json!({"content": {"text": "hi"}});
    let event = fork::map_app_server_notification("reasoning/text/delta", &params).expect("mapped");
    assert_eq!(event.kind, AgentWrapperEventKind::TextOutput);
    assert_eq!(event.channel.as_deref(), Some("assistant"));
    assert_eq!(event.text.as_deref(), Some("hi"));
    assert_eq!(event.data, None);
}

#[test]
fn app_server_item_started_maps_to_tool_call_with_metadata_only_facet() {
    let params = json!({
        "item_id": "item-1",
        "thread_id": "thread-1",
        "turn_id": "turn-1",
        "item_type": "command_execution"
    });
    let event = fork::map_app_server_notification("item/started", &params).expect("mapped");
    assert_eq!(event.kind, AgentWrapperEventKind::ToolCall);
    assert_eq!(tool_schema(&event), Some(TOOLS_FACET_SCHEMA));
    assert_eq!(
        tool_field(&event, "phase").and_then(Value::as_str),
        Some("start")
    );
    assert_eq!(
        tool_field(&event, "status").and_then(Value::as_str),
        Some("running")
    );
    assert_eq!(
        tool_field(&event, "kind").and_then(Value::as_str),
        Some("command_execution")
    );
}

#[test]
fn app_server_error_maps_to_error_event_with_safe_message_and_no_data() {
    let secret = "SECRET_SHOULD_NOT_LEAK";
    let params = json!({
        "error": {"message": "boom", "additionalDetails": {"secret": secret}},
        "message": secret
    });
    let event = fork::map_app_server_notification("error", &params).expect("mapped");
    assert_eq!(event.kind, AgentWrapperEventKind::Error);
    assert_eq!(event.message.as_deref(), Some("boom"));
    assert!(!event.message.as_deref().unwrap().contains(secret));
    assert_eq!(event.data, None);
}

#[test]
fn approval_request_detector_matches_direct_and_wrapped_payloads() {
    assert!(fork::is_approval_request_notification(
        "codex/event",
        &json!({"type": "approval_required", "approval_id": "ap-1", "kind": "exec"}),
    ));
    assert!(fork::is_approval_request_notification(
        "codex/event",
        &json!({"msg": {"type": "approval", "id": "ap-1", "approval_kind": "exec"}}),
    ));
    assert!(!fork::is_approval_request_notification(
        "codex/event",
        &json!({"type": "task_complete"}),
    ));
    assert!(!fork::is_approval_request_notification(
        "agentMessage/delta",
        &json!({"type": "approval_required"}),
    ));
}
