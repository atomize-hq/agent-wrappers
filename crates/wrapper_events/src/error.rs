use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum AdapterErrorCode {
    JsonParse,
    Normalize,
    TypedParse,
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CapturedRaw {
    pub line: Option<String>,
    pub json: Option<Value>,
}

#[derive(Debug, Error, Clone)]
pub enum LineRecordError {
    #[error("I/O error while reading wrapper output")]
    Io,
    #[error("invalid UTF-8 in wrapper output")]
    InvalidUtf8,
    #[error("line too long (observed_bytes={observed_bytes}, max_line_bytes={max_line_bytes})")]
    LineTooLong {
        observed_bytes: usize,
        max_line_bytes: usize,
    },
    #[error("adapter parse failure ({code:?}): {summary}")]
    Adapter {
        code: AdapterErrorCode,
        summary: String,
    },
}

#[derive(Debug, Clone)]
pub struct LineRecord<T> {
    pub line_number: usize,
    pub captured_raw: Option<CapturedRaw>,
    pub outcome: Result<T, LineRecordError>,
}

#[derive(Debug, Clone)]
pub struct ErrorDetail {
    pub line_number: usize,
    pub code: AdapterErrorCode,
    pub adapter: &'static str,
    pub details: String,
}

pub trait ErrorDetailSink: Send + 'static {
    fn on_error(&mut self, detail: ErrorDetail);
}
