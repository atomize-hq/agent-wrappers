use std::error::Error;

use serde_json::Value;

use crate::error::AdapterErrorCode;

pub struct LineInput<'a> {
    pub line: &'a str,
    pub json_capture: Option<&'a Value>,
}

pub trait LineParser {
    type Event;
    type Error: ClassifiedParserError;

    fn reset(&mut self);
    fn parse_line(&mut self, input: LineInput<'_>) -> Result<Option<Self::Event>, Self::Error>;
}

pub trait ClassifiedParserError: Error {
    fn code(&self) -> AdapterErrorCode;
    fn redacted_summary(&self) -> String;
    fn full_details(&self) -> String;
}
