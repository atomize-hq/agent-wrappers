use claude_code::{
    ClaudeStreamJsonErrorCode, ClaudeStreamJsonEvent, ClaudeStreamJsonParseError,
    ClaudeStreamJsonParser,
};

use crate::error::{AdapterErrorCode, CapturedRaw};
use crate::line_parser::{ClassifiedParserError, LineInput, LineParser};
use crate::normalized::{
    NormalizationContext, NormalizedEventKind, NormalizedEvents, NormalizedWrapperEvent,
    WrapperAgentKind,
};
use crate::ValidatedChannelString;

#[derive(Debug, Clone, Default)]
pub struct ClaudeCodeLineParser {
    parser: ClaudeStreamJsonParser,
}

impl ClaudeCodeLineParser {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Debug, thiserror::Error)]
#[error("{redacted}")]
pub struct ClaudeCodeLineParserError {
    code: AdapterErrorCode,
    redacted: String,
    details: String,
}

impl ClassifiedParserError for ClaudeCodeLineParserError {
    fn code(&self) -> AdapterErrorCode {
        self.code
    }

    fn redacted_summary(&self) -> String {
        self.redacted.clone()
    }

    fn full_details(&self) -> String {
        self.details.clone()
    }
}

impl LineParser for ClaudeCodeLineParser {
    type Event = ClaudeStreamJsonEvent;
    type Error = ClaudeCodeLineParserError;

    fn reset(&mut self) {
        self.parser.reset();
    }

    fn parse_line(&mut self, input: LineInput<'_>) -> Result<Option<Self::Event>, Self::Error> {
        self.parser
            .parse_line(input.line)
            .map_err(|err| claude_err(err))
    }
}

fn claude_err(err: ClaudeStreamJsonParseError) -> ClaudeCodeLineParserError {
    ClaudeCodeLineParserError {
        code: map_code(err.code),
        redacted: err.message.clone(),
        details: err.details,
    }
}

fn map_code(code: ClaudeStreamJsonErrorCode) -> AdapterErrorCode {
    match code {
        ClaudeStreamJsonErrorCode::JsonParse => AdapterErrorCode::JsonParse,
        ClaudeStreamJsonErrorCode::TypedParse => AdapterErrorCode::TypedParse,
        ClaudeStreamJsonErrorCode::Normalize => AdapterErrorCode::Normalize,
        ClaudeStreamJsonErrorCode::Unknown => AdapterErrorCode::Unknown,
    }
}

pub fn normalize_claude_code_event(
    line_number: usize,
    context: NormalizationContext,
    captured_raw: Option<CapturedRaw>,
    event: &ClaudeStreamJsonEvent,
) -> NormalizedEvents {
    let kind = classify(event);
    let producer = producer(event);
    let channel = channel_for(kind, producer);
    NormalizedEvents(vec![NormalizedWrapperEvent {
        line_number,
        agent_kind: WrapperAgentKind::ClaudeCode,
        kind,
        context,
        channel,
        captured_raw,
    }])
}

fn producer(event: &ClaudeStreamJsonEvent) -> Option<&'static str> {
    match event {
        ClaudeStreamJsonEvent::SystemInit { .. } | ClaudeStreamJsonEvent::SystemOther { .. } => {
            Some("system")
        }
        ClaudeStreamJsonEvent::UserMessage { .. } => Some("user"),
        ClaudeStreamJsonEvent::AssistantMessage { .. } => Some("assistant"),
        ClaudeStreamJsonEvent::ResultSuccess { .. } | ClaudeStreamJsonEvent::ResultError { .. } => {
            Some("result")
        }
        ClaudeStreamJsonEvent::StreamEvent { .. } | ClaudeStreamJsonEvent::Unknown { .. } => None,
    }
}

fn channel_for(
    kind: NormalizedEventKind,
    producer: Option<&'static str>,
) -> Option<ValidatedChannelString> {
    let raw = match kind {
        NormalizedEventKind::ToolCall | NormalizedEventKind::ToolResult => Some("tool"),
        NormalizedEventKind::Error => Some("error"),
        _ => producer,
    }?;
    ValidatedChannelString::new(raw)
}

fn classify(event: &ClaudeStreamJsonEvent) -> NormalizedEventKind {
    match event {
        ClaudeStreamJsonEvent::SystemInit { .. } | ClaudeStreamJsonEvent::SystemOther { .. } => {
            NormalizedEventKind::Status
        }
        ClaudeStreamJsonEvent::UserMessage { .. } => NormalizedEventKind::Status,
        ClaudeStreamJsonEvent::ResultSuccess { .. } => NormalizedEventKind::Status,
        ClaudeStreamJsonEvent::ResultError { .. } => NormalizedEventKind::Error,
        ClaudeStreamJsonEvent::Unknown { .. } => NormalizedEventKind::Unknown,
        ClaudeStreamJsonEvent::AssistantMessage { raw, .. } => classify_assistant(raw),
        ClaudeStreamJsonEvent::StreamEvent { raw, .. } => classify_stream_event(raw),
    }
}

fn classify_assistant(raw: &serde_json::Value) -> NormalizedEventKind {
    let Some(content) = raw
        .get("message")
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_array())
    else {
        return NormalizedEventKind::TextOutput;
    };

    let mut saw_tool_result = false;
    for item in content {
        let Some(ty) = item.get("type").and_then(|t| t.as_str()) else {
            continue;
        };
        if ty == "tool_use" {
            return NormalizedEventKind::ToolCall;
        }
        if ty == "tool_result" {
            saw_tool_result = true;
        }
    }
    if saw_tool_result {
        NormalizedEventKind::ToolResult
    } else {
        NormalizedEventKind::TextOutput
    }
}

fn classify_stream_event(raw: &serde_json::Value) -> NormalizedEventKind {
    let Some(event) = raw.get("event") else {
        return NormalizedEventKind::Status;
    };
    let Some(event_type) = event.get("type").and_then(|t| t.as_str()) else {
        return NormalizedEventKind::Status;
    };

    match event_type {
        "error" => NormalizedEventKind::Error,
        "content_block_start" => {
            let block_type = event
                .get("content_block")
                .and_then(|b| b.get("type"))
                .and_then(|t| t.as_str());
            match block_type {
                Some("tool_use") => NormalizedEventKind::ToolCall,
                Some("tool_result") => NormalizedEventKind::ToolResult,
                _ => NormalizedEventKind::Status,
            }
        }
        "content_block_delta" => {
            let delta_type = event
                .get("delta")
                .and_then(|d| d.get("type"))
                .and_then(|t| t.as_str());
            match delta_type {
                Some("text_delta") => NormalizedEventKind::TextOutput,
                Some("input_json_delta") => NormalizedEventKind::ToolCall,
                _ => NormalizedEventKind::Status,
            }
        }
        _ => NormalizedEventKind::Status,
    }
}
