use claude_code::{ClaudeStreamJsonEvent, ClaudeStreamJsonParser};
use serde_json::json;
use std::path::PathBuf;

pub(super) use super::super::harness::ClaudeBackendEvent;
pub(super) use super::super::*;
pub(super) use crate::{
    backend_harness::BackendHarnessAdapter,
    mcp::{AgentWrapperMcpAddRequest, AgentWrapperMcpAddTransport, AgentWrapperMcpRemoveRequest},
    mcp::{
        CAPABILITY_MCP_ADD_V1, CAPABILITY_MCP_GET_V1, CAPABILITY_MCP_LIST_V1,
        CAPABILITY_MCP_REMOVE_V1,
    },
    AgentWrapperBackend, AgentWrapperError, AgentWrapperEvent, AgentWrapperEventKind,
    AgentWrapperRunRequest,
};
pub(super) use serde_json::Value as JsonValue;

pub(super) const SYSTEM_INIT: &str =
    include_str!("../../../../../claude_code/tests/fixtures/stream_json/v1/system_init.jsonl");
pub(super) const SYSTEM_OTHER: &str =
    include_str!("../../../../../claude_code/tests/fixtures/stream_json/v1/system_other.jsonl");
pub(super) const RESULT_ERROR: &str =
    include_str!("../../../../../claude_code/tests/fixtures/stream_json/v1/result_error.jsonl");
pub(super) const ASSISTANT_MESSAGE_TEXT: &str = include_str!(
    "../../../../../claude_code/tests/fixtures/stream_json/v1/assistant_message_text.jsonl"
);
pub(super) const ASSISTANT_MESSAGE_TOOL_USE: &str = include_str!(
    "../../../../../claude_code/tests/fixtures/stream_json/v1/assistant_message_tool_use.jsonl"
);
pub(super) const ASSISTANT_MESSAGE_TOOL_RESULT: &str = include_str!(
    "../../../../../claude_code/tests/fixtures/stream_json/v1/assistant_message_tool_result.jsonl"
);
pub(super) const STREAM_EVENT_TEXT_DELTA: &str = include_str!(
    "../../../../../claude_code/tests/fixtures/stream_json/v1/stream_event_text_delta.jsonl"
);
pub(super) const STREAM_EVENT_INPUT_JSON_DELTA: &str = include_str!(
    "../../../../../claude_code/tests/fixtures/stream_json/v1/stream_event_input_json_delta.jsonl"
);
pub(super) const STREAM_EVENT_TOOL_USE_START: &str = include_str!(
    "../../../../../claude_code/tests/fixtures/stream_json/v1/stream_event_tool_use_start.jsonl"
);
pub(super) const STREAM_EVENT_TOOL_RESULT_START: &str = include_str!(
    "../../../../../claude_code/tests/fixtures/stream_json/v1/stream_event_tool_result_start.jsonl"
);
pub(super) const UNKNOWN_OUTER_TYPE: &str = include_str!(
    "../../../../../claude_code/tests/fixtures/stream_json/v1/unknown_outer_type.jsonl"
);

pub(super) fn parse_stream_json_fixture(text: &str) -> ClaudeStreamJsonEvent {
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

pub(super) fn map_fixture(text: &str) -> AgentWrapperEvent {
    let event = parse_stream_json_fixture(text);
    let mapped = super::super::mapping::map_stream_json_event(event);
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

pub(super) fn success_exit_status() -> std::process::ExitStatus {
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

pub(super) fn exit_status_with_code(code: i32) -> std::process::ExitStatus {
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

pub(super) fn sample_mcp_add_request() -> AgentWrapperMcpAddRequest {
    AgentWrapperMcpAddRequest {
        name: "demo".to_string(),
        transport: AgentWrapperMcpAddTransport::Stdio {
            command: vec!["node".to_string()],
            args: vec!["server.js".to_string()],
            env: std::collections::BTreeMap::from([(
                "SERVER_ONLY".to_string(),
                "server-value".to_string(),
            )]),
        },
        context: Default::default(),
    }
}

pub(super) fn sample_mcp_remove_request() -> AgentWrapperMcpRemoveRequest {
    AgentWrapperMcpRemoveRequest {
        name: "demo".to_string(),
        context: Default::default(),
    }
}

pub(super) fn new_adapter() -> ClaudeHarnessAdapter {
    new_test_adapter(ClaudeCodeBackendConfig::default())
}

pub(super) fn new_adapter_with_config(config: ClaudeCodeBackendConfig) -> ClaudeHarnessAdapter {
    new_test_adapter(config)
}

pub(super) fn new_adapter_with_run_start_cwd(
    run_start_cwd: Option<PathBuf>,
) -> ClaudeHarnessAdapter {
    new_test_adapter_with_run_start_cwd(ClaudeCodeBackendConfig::default(), run_start_cwd)
}

pub(super) fn new_adapter_with_config_and_run_start_cwd(
    config: ClaudeCodeBackendConfig,
    run_start_cwd: Option<PathBuf>,
) -> ClaudeHarnessAdapter {
    new_test_adapter_with_run_start_cwd(config, run_start_cwd)
}

pub(super) fn add_dirs_payload(dirs: &[impl AsRef<str>]) -> JsonValue {
    json!({
        "dirs": dirs.iter().map(|dir| dir.as_ref()).collect::<Vec<_>>()
    })
}

pub(super) fn parse_single_line(line: &str) -> ClaudeStreamJsonEvent {
    let mut parser = ClaudeStreamJsonParser::new();
    parser
        .parse_line(line)
        .expect("line parses")
        .expect("line yields a typed event")
}

pub(super) fn handle_facet_schema(event: &crate::AgentWrapperEvent) -> Option<&str> {
    event
        .data
        .as_ref()
        .and_then(|v| v.get("schema"))
        .and_then(|v| v.as_str())
}
