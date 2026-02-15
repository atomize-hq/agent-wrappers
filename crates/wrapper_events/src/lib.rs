#![forbid(unsafe_code)]
//! Shared ingestion primitives for wrapper JSONL/NDJSON outputs.
//!
//! This crate is intentionally **not** a Substrate envelope. It provides:
//! - A bounded-memory, line-oriented ingestion loop (sync + optional tokio).
//! - Adapter plumbing (feature-gated) for wrapper-specific parsers.
//! - A minimal normalized event shape for consumers that want a unified view.

mod channel;
mod config;
mod error;
mod ingest;
mod line_parser;
mod normalized;
mod reader;

#[cfg(feature = "codex")]
pub mod codex_adapter;

#[cfg(feature = "claude_code")]
pub mod claude_code_adapter;

pub use channel::ValidatedChannelString;
pub use config::{CaptureRaw, ErrorDetailCapture, IngestConfig, IngestLimits};
pub use error::{
    AdapterErrorCode, CapturedRaw, ErrorDetail, ErrorDetailSink, LineRecord, LineRecordError,
};
pub use ingest::{LineIngestor, RawCaptureBudget};
pub use line_parser::{ClassifiedParserError, LineInput, LineParser};
pub use normalized::{
    NormalizationContext, NormalizedEventKind, NormalizedEvents, NormalizedWrapperEvent,
    WrapperAgentKind,
};

#[cfg(feature = "tokio")]
pub use ingest::AsyncLineIngestor;
