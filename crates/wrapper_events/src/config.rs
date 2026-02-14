use crate::error::ErrorDetailSink;

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub enum CaptureRaw {
    #[default]
    None,
    Line,
    Json,
    Both,
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub enum ErrorDetailCapture {
    #[default]
    RedactedSummaryOnly,
    FullDetails,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct IngestLimits {
    pub max_line_bytes: usize,
    pub max_raw_bytes_total: Option<usize>,
}

impl Default for IngestLimits {
    fn default() -> Self {
        Self {
            max_line_bytes: 64 * 1024,
            max_raw_bytes_total: None,
        }
    }
}

pub struct IngestConfig {
    pub limits: IngestLimits,
    pub capture_raw: CaptureRaw,
    pub error_detail_capture: ErrorDetailCapture,
    pub error_sink: Option<Box<dyn ErrorDetailSink>>,
}

impl Default for IngestConfig {
    fn default() -> Self {
        Self {
            limits: IngestLimits::default(),
            capture_raw: CaptureRaw::None,
            error_detail_capture: ErrorDetailCapture::RedactedSummaryOnly,
            error_sink: None,
        }
    }
}
