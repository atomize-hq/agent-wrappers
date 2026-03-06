use super::*;
use crate::{AgentWrapperBackend, AgentWrapperError, AgentWrapperEventKind};
use claude_code::{ClaudeStreamJsonEvent, ClaudeStreamJsonParser};
use serde_json::Value;

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

fn exit_status_with_code(code: i32) -> std::process::ExitStatus {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        std::process::ExitStatus::from_raw(code << 8)
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::ExitStatusExt;
        std::process::ExitStatus::from_raw(code as u32)
    }
}

fn new_adapter() -> ClaudeHarnessAdapter {
    ClaudeHarnessAdapter {
        config: ClaudeCodeBackendConfig::default(),
        termination: None,
        handle_state: std::sync::Arc::new(std::sync::Mutex::new(ClaudeHandleFacetState::default())),
        allow_flag_preflight: std::sync::Arc::new(OnceCell::new()),
    }
}

fn new_adapter_with_config(config: ClaudeCodeBackendConfig) -> ClaudeHarnessAdapter {
    ClaudeHarnessAdapter {
        config,
        termination: None,
        handle_state: std::sync::Arc::new(std::sync::Mutex::new(ClaudeHandleFacetState::default())),
        allow_flag_preflight: std::sync::Arc::new(OnceCell::new()),
    }
}

fn parse_single_line(line: &str) -> ClaudeStreamJsonEvent {
    let mut parser = ClaudeStreamJsonParser::new();
    parser
        .parse_line(line)
        .expect("line parses")
        .expect("line yields a typed event")
}

fn handle_facet_schema(event: &crate::AgentWrapperEvent) -> Option<&str> {
    event
        .data
        .as_ref()
        .and_then(|v| v.get("schema"))
        .and_then(|v| v.as_str())
}

#[test]
fn claude_backend_reports_required_capabilities() {
    let backend = ClaudeCodeBackend::new(ClaudeCodeBackendConfig::default());
    let capabilities = backend.capabilities();
    assert!(capabilities.contains("agent_api.run"));
    assert!(capabilities.contains("agent_api.events"));
    assert!(capabilities.contains("agent_api.events.live"));
    assert!(capabilities.contains(crate::CAPABILITY_CONTROL_CANCEL_V1));
    assert!(capabilities.contains(CAP_TOOLS_STRUCTURED_V1));
    assert!(capabilities.contains(CAP_TOOLS_RESULTS_V1));
    assert!(capabilities.contains(CAP_ARTIFACTS_FINAL_TEXT_V1));
    assert!(capabilities.contains(CAP_SESSION_HANDLE_V1));
    assert!(capabilities.contains(EXT_SESSION_RESUME_V1));
    assert!(capabilities.contains(EXT_SESSION_FORK_V1));
}

#[test]
fn claude_backend_does_not_advertise_external_sandbox_exec_by_default() {
    let backend = ClaudeCodeBackend::new(ClaudeCodeBackendConfig::default());
    let capabilities = backend.capabilities();
    assert!(!capabilities.contains(EXT_EXTERNAL_SANDBOX_V1));
}

#[test]
fn claude_backend_opt_in_advertises_external_sandbox_exec_and_allowlist_accepts_key() {
    let config = ClaudeCodeBackendConfig {
        allow_external_sandbox_exec: true,
        ..Default::default()
    };

    let backend = ClaudeCodeBackend::new(config.clone());
    let capabilities = backend.capabilities();
    assert!(capabilities.contains(EXT_EXTERNAL_SANDBOX_V1));

    let adapter = new_adapter_with_config(config);
    let defaults = crate::backend_harness::BackendDefaults::default();
    let request = AgentWrapperRunRequest {
        prompt: "hello".to_string(),
        extensions: [(EXT_EXTERNAL_SANDBOX_V1.to_string(), Value::Bool(true))]
            .into_iter()
            .collect(),
        ..Default::default()
    };

    crate::backend_harness::normalize_request(&adapter, &defaults, request)
        .expect("external sandbox extension key should be allowlisted when opted-in");
}

#[test]
fn claude_backend_fails_closed_for_external_sandbox_extension_when_opt_in_disabled() {
    let adapter = new_adapter();
    let defaults = crate::backend_harness::BackendDefaults::default();
    let request = AgentWrapperRunRequest {
        prompt: "hello".to_string(),
        extensions: [(EXT_EXTERNAL_SANDBOX_V1.to_string(), Value::Bool(true))]
            .into_iter()
            .collect(),
        ..Default::default()
    };

    let err = match crate::backend_harness::normalize_request(&adapter, &defaults, request) {
        Ok(_) => panic!("expected normalize_request to reject unsupported extension key"),
        Err(err) => err,
    };
    match err {
        AgentWrapperError::UnsupportedCapability { capability, .. } => {
            assert_eq!(capability, EXT_EXTERNAL_SANDBOX_V1);
        }
        other => panic!("expected UnsupportedCapability, got: {other:?}"),
    }
}

#[tokio::test]
async fn allow_flag_preflight_retries_after_failure() {
    let cell = OnceCell::new();

    let result = preflight_allow_flag_support(&cell, || async {
        Ok::<_, claude_code::ClaudeCodeError>(claude_code::CommandOutput {
            status: exit_status_with_code(1),
            stdout: Vec::new(),
            stderr: Vec::new(),
        })
    })
    .await;

    assert!(result.is_err(), "preflight should surface the failure");
    assert!(
        cell.get().is_none(),
        "failed preflight should not initialize the OnceCell"
    );

    let supported = preflight_allow_flag_support(&cell, || async {
        Ok::<_, claude_code::ClaudeCodeError>(claude_code::CommandOutput {
            status: success_exit_status(),
            stdout: b"--allow-dangerously-skip-permissions".to_vec(),
            stderr: Vec::new(),
        })
    })
    .await
    .expect("preflight should succeed");

    assert!(supported);
    assert_eq!(cell.get().copied(), Some(true));

    let called = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let called_clone = std::sync::Arc::clone(&called);
    let supported = preflight_allow_flag_support(&cell, move || {
        let called = std::sync::Arc::clone(&called_clone);
        async move {
            called.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok::<_, claude_code::ClaudeCodeError>(claude_code::CommandOutput {
                status: success_exit_status(),
                stdout: Vec::new(),
                stderr: Vec::new(),
            })
        }
    })
    .await
    .expect("cached preflight should succeed");

    assert!(supported);
    assert_eq!(
        called.load(std::sync::atomic::Ordering::SeqCst),
        0,
        "cached preflight should not re-run help()"
    );
}

#[test]
fn claude_adapter_implements_backend_harness_adapter_contract() {
    fn assert_impl<T: crate::backend_harness::BackendHarnessAdapter>() {}
    assert_impl::<ClaudeHarnessAdapter>();
}

#[test]
fn claude_backend_routes_through_harness_and_does_not_reintroduce_orchestration_primitives() {
    const SOURCE: &str = include_str!("../claude_code.rs");

    assert!(
        SOURCE.contains("run_harnessed_backend("),
        "expected Claude backend to route through the harness entrypoint"
    );
    assert!(
        SOURCE.contains("run_harnessed_backend_control("),
        "expected Claude backend to route cancellation through the harness control entrypoint"
    );
    assert!(
        SOURCE.contains("TerminationState::new"),
        "expected Claude backend control path to register a termination hook"
    );

    assert!(
        !SOURCE.contains("build_gated_run_handle("),
        "expected Claude backend to not bypass harness-owned completion gating"
    );
    assert!(
        !SOURCE.contains("mpsc::channel::<AgentWrapperEvent>(32)"),
        "expected Claude backend to not create a backend-local events channel"
    );
    assert!(
        !SOURCE.contains("tokio::time::timeout("),
        "expected Claude backend to not wrap runs with backend-local timeout orchestration"
    );
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

#[test]
fn claude_emits_handle_facet_once_when_first_event_is_status() {
    let adapter = new_adapter();

    let out1 = adapter.map_event(ClaudeBackendEvent::Stream(parse_stream_json_fixture(
        SYSTEM_INIT,
    )));
    let facet_events_1: Vec<_> = out1
        .iter()
        .filter(|event| handle_facet_schema(event) == Some(CAP_SESSION_HANDLE_V1))
        .collect();
    assert_eq!(facet_events_1.len(), 1);
    assert_eq!(facet_events_1[0].kind, AgentWrapperEventKind::Status);
    assert_eq!(
        facet_events_1[0]
            .data
            .as_ref()
            .and_then(|v| v.pointer("/session/id"))
            .and_then(|v| v.as_str()),
        Some("sess-1")
    );

    let out2 = adapter.map_event(ClaudeBackendEvent::Stream(parse_stream_json_fixture(
        SYSTEM_OTHER,
    )));
    assert!(
        out2.iter()
            .all(|event| handle_facet_schema(event) != Some(CAP_SESSION_HANDLE_V1)),
        "expected subsequent events to not re-emit the handle facet"
    );
}

#[test]
fn claude_emits_synthetic_status_handle_facet_when_first_event_is_not_status() {
    let adapter = new_adapter();

    let out = adapter.map_event(ClaudeBackendEvent::Stream(parse_stream_json_fixture(
        ASSISTANT_MESSAGE_TEXT,
    )));
    assert_eq!(out.len(), 2);
    assert_eq!(out[0].kind, AgentWrapperEventKind::TextOutput);
    assert_eq!(out[0].text.as_deref(), Some("hello"));

    assert_eq!(out[1].kind, AgentWrapperEventKind::Status);
    assert_eq!(handle_facet_schema(&out[1]), Some(CAP_SESSION_HANDLE_V1));
    assert_eq!(
        out[1]
            .data
            .as_ref()
            .and_then(|v| v.pointer("/session/id"))
            .and_then(|v| v.as_str()),
        Some("sess-1")
    );
}

#[test]
fn claude_completion_attaches_handle_facet_when_id_is_known() {
    let adapter = new_adapter();

    let _events = adapter.map_event(ClaudeBackendEvent::Stream(parse_stream_json_fixture(
        SYSTEM_INIT,
    )));

    let completion = adapter
        .map_completion(ClaudeBackendCompletion {
            status: success_exit_status(),
            final_text: None,
            selection_failure_message: None,
        })
        .expect("completion maps");

    assert_eq!(
        completion
            .data
            .as_ref()
            .and_then(|v| v.get("schema"))
            .and_then(|v| v.as_str()),
        Some(CAP_SESSION_HANDLE_V1)
    );
    assert_eq!(
        completion
            .data
            .as_ref()
            .and_then(|v| v.pointer("/session/id"))
            .and_then(|v| v.as_str()),
        Some("sess-1")
    );
}

#[test]
fn claude_whitespace_session_id_is_treated_as_unknown() {
    let adapter = new_adapter();

    let out = adapter.map_event(ClaudeBackendEvent::Stream(parse_single_line(
        r#"{"type":"system","subtype":"init","session_id":"   "}"#,
    )));
    assert!(
        out.iter()
            .all(|event| handle_facet_schema(event) != Some(CAP_SESSION_HANDLE_V1)),
        "expected whitespace-only session_id to not produce a handle facet"
    );

    let completion = adapter
        .map_completion(ClaudeBackendCompletion {
            status: success_exit_status(),
            final_text: None,
            selection_failure_message: None,
        })
        .expect("completion maps");
    assert!(completion.data.is_none());
}

#[test]
fn claude_oversize_session_id_is_omitted_and_warns_once() {
    let adapter = new_adapter();

    let oversize = "a".repeat(SESSION_HANDLE_ID_BOUND_BYTES + 1);
    let line = format!(r#"{{"type":"system","subtype":"init","session_id":"{oversize}"}}"#);
    let out1 = adapter.map_event(ClaudeBackendEvent::Stream(parse_single_line(&line)));

    assert!(
        out1.iter()
            .all(|event| handle_facet_schema(event) != Some(CAP_SESSION_HANDLE_V1)),
        "expected oversize session_id to omit the handle facet"
    );

    let warnings_1: Vec<_> = out1
        .iter()
        .filter(|event| event.message.as_deref() == Some(SESSION_HANDLE_OVERSIZE_WARNING))
        .collect();
    assert_eq!(warnings_1.len(), 1);
    assert_eq!(warnings_1[0].kind, AgentWrapperEventKind::Status);
    assert!(warnings_1[0].data.is_none());

    let out2 = adapter.map_event(ClaudeBackendEvent::Stream(parse_single_line(&line)));
    assert!(
        out2.iter()
            .all(|event| event.message.as_deref() != Some(SESSION_HANDLE_OVERSIZE_WARNING)),
        "expected oversize warning to be emitted at most once"
    );

    let completion = adapter
        .map_completion(ClaudeBackendCompletion {
            status: success_exit_status(),
            final_text: None,
            selection_failure_message: None,
        })
        .expect("completion maps");
    assert!(completion.data.is_none());
}
