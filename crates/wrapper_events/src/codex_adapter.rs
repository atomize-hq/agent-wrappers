use codex::{ExecStreamError, JsonlThreadEventParser, ThreadEvent};

use crate::error::AdapterErrorCode;
use crate::line_parser::{ClassifiedParserError, LineInput, LineParser};
use crate::normalized::{
    NormalizationContext, NormalizedEventKind, NormalizedEvents, NormalizedWrapperEvent,
    WrapperAgentKind,
};
use crate::CapturedRaw;

#[derive(Debug, Clone)]
pub struct CodexLineParser {
    parser: JsonlThreadEventParser,
}

impl Default for CodexLineParser {
    fn default() -> Self {
        Self {
            parser: JsonlThreadEventParser::new(),
        }
    }
}

impl CodexLineParser {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Debug, thiserror::Error)]
#[error("{redacted}")]
pub struct CodexLineParserError {
    code: AdapterErrorCode,
    redacted: String,
    details: String,
}

impl ClassifiedParserError for CodexLineParserError {
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

impl LineParser for CodexLineParser {
    type Event = ThreadEvent;
    type Error = CodexLineParserError;

    fn reset(&mut self) {
        self.parser.reset();
    }

    fn parse_line(&mut self, input: LineInput<'_>) -> Result<Option<Self::Event>, Self::Error> {
        self.parser
            .parse_line(input.line)
            .map_err(|err| codex_err(err))
    }
}

fn codex_err(err: ExecStreamError) -> CodexLineParserError {
    let (code, redacted) = match &err {
        ExecStreamError::Parse { source, .. } => (
            AdapterErrorCode::JsonParse,
            format!("parse error: {source}"),
        ),
        ExecStreamError::Normalize { message, .. } => (
            AdapterErrorCode::Normalize,
            format!("normalize error: {message}"),
        ),
        ExecStreamError::Codex(source) => {
            (AdapterErrorCode::Unknown, format!("codex error: {source}"))
        }
        ExecStreamError::IdleTimeout { .. } => (
            AdapterErrorCode::Unknown,
            "idle timeout while reading codex stream".to_string(),
        ),
        ExecStreamError::ChannelClosed => (
            AdapterErrorCode::Unknown,
            "codex channel closed".to_string(),
        ),
    };

    CodexLineParserError {
        code,
        redacted,
        details: err.to_string(),
    }
}

pub fn normalize_codex_event(
    line_number: usize,
    context: NormalizationContext,
    captured_raw: Option<CapturedRaw>,
    event: &ThreadEvent,
) -> NormalizedEvents {
    let kind = classify_codex_event(event);
    NormalizedEvents(vec![NormalizedWrapperEvent {
        line_number,
        agent_kind: WrapperAgentKind::Codex,
        kind,
        context,
        channel: None,
        captured_raw,
    }])
}

fn classify_codex_event(event: &ThreadEvent) -> NormalizedEventKind {
    match event {
        ThreadEvent::ThreadStarted(_)
        | ThreadEvent::TurnStarted(_)
        | ThreadEvent::TurnCompleted(_)
        | ThreadEvent::TurnFailed(_) => NormalizedEventKind::Status,
        ThreadEvent::Error(_) => NormalizedEventKind::Error,
        ThreadEvent::ItemFailed(_) => NormalizedEventKind::Error,
        ThreadEvent::ItemStarted(env) | ThreadEvent::ItemCompleted(env) => {
            classify_item_payload(&env.item.payload)
        }
        ThreadEvent::ItemDelta(delta) => classify_item_delta(&delta.delta),
    }
}

fn classify_item_payload(payload: &codex::ItemPayload) -> NormalizedEventKind {
    match payload {
        codex::ItemPayload::AgentMessage(_) | codex::ItemPayload::Reasoning(_) => {
            NormalizedEventKind::TextOutput
        }
        codex::ItemPayload::CommandExecution(_)
        | codex::ItemPayload::FileChange(_)
        | codex::ItemPayload::McpToolCall(_)
        | codex::ItemPayload::WebSearch(_) => NormalizedEventKind::ToolCall,
        codex::ItemPayload::TodoList(_) => NormalizedEventKind::Status,
        codex::ItemPayload::Error(_) => NormalizedEventKind::Error,
    }
}

fn classify_item_delta(payload: &codex::ItemDeltaPayload) -> NormalizedEventKind {
    match payload {
        codex::ItemDeltaPayload::AgentMessage(_) | codex::ItemDeltaPayload::Reasoning(_) => {
            NormalizedEventKind::TextOutput
        }
        codex::ItemDeltaPayload::CommandExecution(_)
        | codex::ItemDeltaPayload::FileChange(_)
        | codex::ItemDeltaPayload::McpToolCall(_)
        | codex::ItemDeltaPayload::WebSearch(_) => NormalizedEventKind::ToolCall,
        codex::ItemDeltaPayload::TodoList(_) => NormalizedEventKind::Status,
        codex::ItemDeltaPayload::Error(_) => NormalizedEventKind::Error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CaptureRaw, IngestConfig, IngestLimits, LineIngestor, LineRecordError};

    #[test]
    fn redacted_summary_never_includes_raw_line() {
        let data = b"{not-json}\n";
        let config = IngestConfig {
            limits: IngestLimits {
                max_line_bytes: 1024,
                max_raw_bytes_total: None,
            },
            capture_raw: CaptureRaw::None,
            ..IngestConfig::default()
        };

        let mut ingestor = LineIngestor::new(
            std::io::Cursor::new(data),
            CodexLineParser::new(),
            config,
            "codex",
        );
        let rec = ingestor.next().unwrap();
        match rec.outcome {
            Err(LineRecordError::Adapter { summary, .. }) => {
                assert!(!summary.contains("{not-json}"));
            }
            other => panic!("expected adapter error, got {other:?}"),
        }
    }
}
