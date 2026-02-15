#![cfg(feature = "claude_code")]

use std::path::PathBuf;

use claude_code::{ClaudeStreamJsonEvent, ClaudeStreamJsonParser};
use wrapper_events::claude_code_adapter::normalize_claude_code_event;
use wrapper_events::{NormalizationContext, NormalizedEventKind, ValidatedChannelString};

fn fixtures_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../claude_code/tests/fixtures/stream_json/v1")
}

fn read_fixture(name: &str) -> String {
    std::fs::read_to_string(fixtures_root().join(name)).expect("read fixture")
}

fn parse_single(name: &str) -> ClaudeStreamJsonEvent {
    let mut parser = ClaudeStreamJsonParser::new();
    let text = read_fixture(name);
    let line = text
        .lines()
        .find(|l| !l.chars().all(|c| c.is_whitespace()))
        .unwrap();
    parser.parse_line(line).unwrap().unwrap()
}

fn ctx() -> NormalizationContext {
    NormalizationContext {
        agent_id: "agent-1".to_string(),
        backend_id: None,
        orchestration_session_id: None,
        run_id: None,
        world_id: None,
        channel_hint: None,
    }
}

fn expect(name: &str, kind: NormalizedEventKind, channel: Option<&str>) {
    let ev = parse_single(name);
    let out = normalize_claude_code_event(1, ctx(), None, &ev);
    assert_eq!(out.0.len(), 1);
    let one = &out.0[0];
    assert_eq!(one.kind, kind);
    assert_eq!(
        one.channel.as_ref().map(|c| c.as_str()),
        channel
            .and_then(ValidatedChannelString::new)
            .as_ref()
            .map(|c| c.as_str())
    );
}

#[test]
fn normalization_and_channel_follow_contract() {
    expect(
        "system_init.jsonl",
        NormalizedEventKind::Status,
        Some("system"),
    );
    expect(
        "user_message.jsonl",
        NormalizedEventKind::Status,
        Some("user"),
    );
    expect(
        "assistant_message_text.jsonl",
        NormalizedEventKind::TextOutput,
        Some("assistant"),
    );
    expect(
        "assistant_message_tool_use.jsonl",
        NormalizedEventKind::ToolCall,
        Some("tool"),
    );
    expect(
        "assistant_message_tool_result.jsonl",
        NormalizedEventKind::ToolResult,
        Some("tool"),
    );
    expect(
        "result_success.jsonl",
        NormalizedEventKind::Status,
        Some("result"),
    );
    expect(
        "result_error.jsonl",
        NormalizedEventKind::Error,
        Some("error"),
    );
    expect(
        "stream_event_text_delta.jsonl",
        NormalizedEventKind::TextOutput,
        None,
    );
    expect(
        "stream_event_input_json_delta.jsonl",
        NormalizedEventKind::ToolCall,
        Some("tool"),
    );
    expect(
        "stream_event_tool_use_start.jsonl",
        NormalizedEventKind::ToolCall,
        Some("tool"),
    );
    expect(
        "stream_event_tool_result_start.jsonl",
        NormalizedEventKind::ToolResult,
        Some("tool"),
    );
    expect(
        "unknown_outer_type.jsonl",
        NormalizedEventKind::Unknown,
        None,
    );
}
