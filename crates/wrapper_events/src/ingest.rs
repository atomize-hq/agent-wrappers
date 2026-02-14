use std::io::Read;

use serde_json::Value;

use crate::config::{CaptureRaw, ErrorDetailCapture, IngestConfig};
use crate::error::{AdapterErrorCode, CapturedRaw, ErrorDetail, LineRecord, LineRecordError};
use crate::line_parser::{ClassifiedParserError, LineInput, LineParser};
use crate::reader::{BoundedLine, SyncBoundedLineReader};

#[derive(Debug, Clone, Copy)]
pub struct RawCaptureBudget {
    remaining_bytes: Option<usize>,
}

impl RawCaptureBudget {
    pub fn new(limit: Option<usize>) -> Self {
        Self {
            remaining_bytes: limit,
        }
    }

    fn can_spend(&self, bytes: usize) -> bool {
        match self.remaining_bytes {
            None => true,
            Some(rem) => bytes <= rem,
        }
    }

    fn spend(&mut self, bytes: usize) {
        if let Some(rem) = self.remaining_bytes {
            self.remaining_bytes = Some(rem.saturating_sub(bytes));
        }
    }
}

pub struct LineIngestor<R: Read, P: LineParser> {
    reader: SyncBoundedLineReader<R>,
    parser: P,
    config: IngestConfig,
    budget: RawCaptureBudget,
    adapter_name: &'static str,
}

impl<R: Read, P: LineParser> LineIngestor<R, P> {
    pub fn new(reader: R, parser: P, config: IngestConfig, adapter_name: &'static str) -> Self {
        let budget = RawCaptureBudget::new(config.limits.max_raw_bytes_total);
        Self {
            reader: SyncBoundedLineReader::new(reader, config.limits.max_line_bytes),
            parser,
            config,
            budget,
            adapter_name,
        }
    }

    pub fn into_parser(self) -> P {
        self.parser
    }

    fn record_error<T>(&self, line_number: usize, err: LineRecordError) -> LineRecord<T> {
        LineRecord {
            line_number,
            captured_raw: None,
            outcome: Err(err),
        }
    }

    fn normalize_line(line: &str) -> &str {
        line.strip_suffix('\r').unwrap_or(line)
    }

    fn line_is_blank(line: &str) -> bool {
        line.chars().all(|ch| ch.is_whitespace())
    }

    fn maybe_capture_line(&mut self, line: &str) -> Option<String> {
        if !matches!(self.config.capture_raw, CaptureRaw::Line | CaptureRaw::Both) {
            return None;
        }
        let bytes = line.as_bytes().len();
        if !self.budget.can_spend(bytes) {
            return None;
        }
        self.budget.spend(bytes);
        Some(line.to_string())
    }

    fn maybe_capture_json(&mut self, line: &str) -> Option<Value> {
        if !matches!(self.config.capture_raw, CaptureRaw::Json | CaptureRaw::Both) {
            return None;
        }
        let value: Value = serde_json::from_str(line).ok()?;
        let bytes = serde_json::to_vec(&value).ok()?.len();
        if !self.budget.can_spend(bytes) {
            return None;
        }
        self.budget.spend(bytes);
        Some(value)
    }

    fn capture_raw(&mut self, line: &str) -> Option<CapturedRaw> {
        match self.config.capture_raw {
            CaptureRaw::None => None,
            CaptureRaw::Line => self.maybe_capture_line(line).map(|line| CapturedRaw {
                line: Some(line),
                json: None,
            }),
            CaptureRaw::Json => self.maybe_capture_json(line).map(|json| CapturedRaw {
                line: None,
                json: Some(json),
            }),
            CaptureRaw::Both => {
                let line_cap = self.maybe_capture_line(line);
                let json_cap = self.maybe_capture_json(line);
                if line_cap.is_none() && json_cap.is_none() {
                    None
                } else {
                    Some(CapturedRaw {
                        line: line_cap,
                        json: json_cap,
                    })
                }
            }
        }
    }

    fn adapter_error_record<T>(
        &mut self,
        line_number: usize,
        code: AdapterErrorCode,
        summary: String,
        full_details: String,
    ) -> LineRecord<T> {
        if self.config.error_detail_capture == ErrorDetailCapture::FullDetails {
            if let Some(sink) = self.config.error_sink.as_mut() {
                sink.on_error(ErrorDetail {
                    line_number,
                    code,
                    adapter: self.adapter_name,
                    details: full_details,
                });
            }
        }
        self.record_error(line_number, LineRecordError::Adapter { code, summary })
    }
}

impl<R: Read, P: LineParser> Iterator for LineIngestor<R, P> {
    type Item = LineRecord<P::Event>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let next = self.reader.next()?;
            match next {
                BoundedLine::IoError { line_number } => {
                    return Some(self.record_error(line_number, LineRecordError::Io));
                }
                BoundedLine::LineTooLong {
                    line_number,
                    observed_bytes,
                    max_line_bytes,
                } => {
                    return Some(self.record_error(
                        line_number,
                        LineRecordError::LineTooLong {
                            observed_bytes,
                            max_line_bytes,
                        },
                    ));
                }
                BoundedLine::Line { line_number, bytes } => {
                    let Ok(raw_line) = String::from_utf8(bytes) else {
                        return Some(self.record_error(line_number, LineRecordError::InvalidUtf8));
                    };
                    let line = Self::normalize_line(&raw_line);
                    if Self::line_is_blank(line) {
                        continue;
                    }

                    let captured_raw = self.capture_raw(line);
                    let json_capture = captured_raw.as_ref().and_then(|raw| raw.json.as_ref());
                    let input = LineInput { line, json_capture };

                    match self.parser.parse_line(input) {
                        Ok(None) => continue,
                        Ok(Some(event)) => {
                            return Some(LineRecord {
                                line_number,
                                captured_raw,
                                outcome: Ok(event),
                            });
                        }
                        Err(err) => {
                            return Some(self.adapter_error_record(
                                line_number,
                                err.code(),
                                err.redacted_summary(),
                                err.full_details(),
                            ));
                        }
                    }
                }
            }
        }
    }
}

#[cfg(feature = "tokio")]
mod tokio_ingest {
    use serde_json::Value;
    use tokio::io::AsyncRead;

    use crate::config::{CaptureRaw, ErrorDetailCapture, IngestConfig};
    use crate::error::{AdapterErrorCode, CapturedRaw, ErrorDetail, LineRecord, LineRecordError};
    use crate::line_parser::{ClassifiedParserError, LineInput, LineParser};
    use crate::reader::{AsyncBoundedLineReader, AsyncBoundedLineResult};
    use crate::RawCaptureBudget;

    pub struct AsyncLineIngestor<R: AsyncRead + Unpin, P: LineParser> {
        reader: AsyncBoundedLineReader<R>,
        parser: P,
        config: IngestConfig,
        budget: RawCaptureBudget,
        adapter_name: &'static str,
    }

    impl<R: AsyncRead + Unpin, P: LineParser> AsyncLineIngestor<R, P> {
        pub fn new(reader: R, parser: P, config: IngestConfig, adapter_name: &'static str) -> Self {
            let budget = RawCaptureBudget::new(config.limits.max_raw_bytes_total);
            Self {
                reader: AsyncBoundedLineReader::new(reader, config.limits.max_line_bytes),
                parser,
                config,
                budget,
                adapter_name,
            }
        }

        fn record_error<T>(&self, line_number: usize, err: LineRecordError) -> LineRecord<T> {
            LineRecord {
                line_number,
                captured_raw: None,
                outcome: Err(err),
            }
        }

        fn normalize_line(line: &str) -> &str {
            line.strip_suffix('\r').unwrap_or(line)
        }

        fn line_is_blank(line: &str) -> bool {
            line.chars().all(|ch| ch.is_whitespace())
        }

        fn maybe_capture_line(&mut self, line: &str) -> Option<String> {
            if !matches!(self.config.capture_raw, CaptureRaw::Line | CaptureRaw::Both) {
                return None;
            }
            let bytes = line.as_bytes().len();
            if !self.budget.can_spend(bytes) {
                return None;
            }
            self.budget.spend(bytes);
            Some(line.to_string())
        }

        fn maybe_capture_json(&mut self, line: &str) -> Option<Value> {
            if !matches!(self.config.capture_raw, CaptureRaw::Json | CaptureRaw::Both) {
                return None;
            }
            let value: Value = serde_json::from_str(line).ok()?;
            let bytes = serde_json::to_vec(&value).ok()?.len();
            if !self.budget.can_spend(bytes) {
                return None;
            }
            self.budget.spend(bytes);
            Some(value)
        }

        fn capture_raw(&mut self, line: &str) -> Option<CapturedRaw> {
            match self.config.capture_raw {
                CaptureRaw::None => None,
                CaptureRaw::Line => self.maybe_capture_line(line).map(|line| CapturedRaw {
                    line: Some(line),
                    json: None,
                }),
                CaptureRaw::Json => self.maybe_capture_json(line).map(|json| CapturedRaw {
                    line: None,
                    json: Some(json),
                }),
                CaptureRaw::Both => {
                    let line_cap = self.maybe_capture_line(line);
                    let json_cap = self.maybe_capture_json(line);
                    if line_cap.is_none() && json_cap.is_none() {
                        None
                    } else {
                        Some(CapturedRaw {
                            line: line_cap,
                            json: json_cap,
                        })
                    }
                }
            }
        }

        fn adapter_error_record<T>(
            &mut self,
            line_number: usize,
            code: AdapterErrorCode,
            summary: String,
            full_details: String,
        ) -> LineRecord<T> {
            if self.config.error_detail_capture == ErrorDetailCapture::FullDetails {
                if let Some(sink) = self.config.error_sink.as_mut() {
                    sink.on_error(ErrorDetail {
                        line_number,
                        code,
                        adapter: self.adapter_name,
                        details: full_details,
                    });
                }
            }
            self.record_error(line_number, LineRecordError::Adapter { code, summary })
        }

        pub async fn next_record(&mut self) -> Option<LineRecord<P::Event>> {
            loop {
                let next = self.reader.next_line().await?;
                match next {
                    AsyncBoundedLineResult::IoError { line_number } => {
                        return Some(self.record_error(line_number, LineRecordError::Io));
                    }
                    AsyncBoundedLineResult::LineTooLong {
                        line_number,
                        observed_bytes,
                        max_line_bytes,
                    } => {
                        return Some(self.record_error(
                            line_number,
                            LineRecordError::LineTooLong {
                                observed_bytes,
                                max_line_bytes,
                            },
                        ));
                    }
                    AsyncBoundedLineResult::Line { line_number, bytes } => {
                        let Ok(raw_line) = String::from_utf8(bytes) else {
                            return Some(
                                self.record_error(line_number, LineRecordError::InvalidUtf8),
                            );
                        };
                        let line = Self::normalize_line(&raw_line);
                        if Self::line_is_blank(line) {
                            continue;
                        }

                        let captured_raw = self.capture_raw(line);
                        let json_capture = captured_raw.as_ref().and_then(|raw| raw.json.as_ref());
                        let input = LineInput { line, json_capture };

                        match self.parser.parse_line(input) {
                            Ok(None) => continue,
                            Ok(Some(event)) => {
                                return Some(LineRecord {
                                    line_number,
                                    captured_raw,
                                    outcome: Ok(event),
                                });
                            }
                            Err(err) => {
                                return Some(self.adapter_error_record(
                                    line_number,
                                    err.code(),
                                    err.redacted_summary(),
                                    err.full_details(),
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[derive(Default)]
        struct TestParser;

        #[derive(Debug, thiserror::Error)]
        #[error("boom")]
        struct TestErr;

        impl crate::line_parser::ClassifiedParserError for TestErr {
            fn code(&self) -> AdapterErrorCode {
                AdapterErrorCode::Unknown
            }

            fn redacted_summary(&self) -> String {
                "boom".to_string()
            }

            fn full_details(&self) -> String {
                "boom details".to_string()
            }
        }

        impl crate::LineParser for TestParser {
            type Event = String;
            type Error = TestErr;

            fn reset(&mut self) {}

            fn parse_line(
                &mut self,
                input: crate::LineInput<'_>,
            ) -> Result<Option<Self::Event>, Self::Error> {
                Ok(Some(input.line.to_string()))
            }
        }

        #[tokio::test]
        async fn budget_skips_capture_deterministically() {
            let data = b"{\"k\":1}\n";
            let mut config = IngestConfig::default();
            config.capture_raw = CaptureRaw::Both;
            config.limits.max_raw_bytes_total = Some(2);

            let mut ingestor = AsyncLineIngestor::new(
                std::io::Cursor::new(data),
                TestParser::default(),
                config,
                "test",
            );

            let rec = ingestor.next_record().await.unwrap();
            assert!(rec.captured_raw.is_none());
            assert!(rec.outcome.is_ok());
        }
    }
}

#[cfg(feature = "tokio")]
pub use tokio_ingest::AsyncLineIngestor;

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct TestParser;

    #[derive(Debug, thiserror::Error)]
    #[error("boom")]
    struct TestErr;

    impl crate::line_parser::ClassifiedParserError for TestErr {
        fn code(&self) -> AdapterErrorCode {
            AdapterErrorCode::Unknown
        }

        fn redacted_summary(&self) -> String {
            "boom".to_string()
        }

        fn full_details(&self) -> String {
            "boom details".to_string()
        }
    }

    impl LineParser for TestParser {
        type Event = String;
        type Error = TestErr;

        fn reset(&mut self) {}

        fn parse_line(&mut self, input: LineInput<'_>) -> Result<Option<Self::Event>, Self::Error> {
            Ok(Some(input.line.to_string()))
        }
    }

    #[test]
    fn captures_line_before_parsing() {
        let data = b"hello\n";
        let mut config = IngestConfig::default();
        config.capture_raw = CaptureRaw::Line;
        config.limits.max_raw_bytes_total = Some(32);

        let mut ingestor = LineIngestor::new(
            std::io::Cursor::new(data),
            TestParser::default(),
            config,
            "test",
        );
        let rec = ingestor.next().unwrap();
        assert_eq!(
            rec.captured_raw.as_ref().and_then(|r| r.line.as_deref()),
            Some("hello")
        );
        assert!(rec.outcome.is_ok());
    }
}
