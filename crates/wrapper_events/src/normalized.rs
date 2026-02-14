use crate::{CapturedRaw, ValidatedChannelString};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum WrapperAgentKind {
    Codex,
    ClaudeCode,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum NormalizedEventKind {
    TextOutput,
    ToolCall,
    ToolResult,
    Status,
    Error,
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NormalizationContext {
    pub agent_id: String,
    pub backend_id: Option<String>,
    pub orchestration_session_id: Option<String>,
    pub run_id: Option<String>,
    pub world_id: Option<String>,
    pub channel_hint: Option<ValidatedChannelString>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NormalizedWrapperEvent {
    pub line_number: usize,
    pub agent_kind: WrapperAgentKind,
    pub kind: NormalizedEventKind,
    pub context: NormalizationContext,
    pub channel: Option<ValidatedChannelString>,
    pub captured_raw: Option<CapturedRaw>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct NormalizedEvents(pub Vec<NormalizedWrapperEvent>);
