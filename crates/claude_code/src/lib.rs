#![forbid(unsafe_code)]
//! Async helper around the Claude Code CLI (`claude`) focused on the headless `--print` flow.
//!
//! This crate intentionally does **not** attempt to wrap interactive default mode (no `--print`)
//! as a parity target. It shells out to a locally installed/pinned `claude` binary.

mod builder;
mod cli;
mod client;
mod commands;
mod error;
mod process;
mod stream_json;
pub mod wrapper_coverage_manifest;

pub use builder::ClaudeClientBuilder;
pub use client::ClaudeClient;
pub use error::{ClaudeCodeError, StreamJsonLineError};
pub use commands::command::ClaudeCommandRequest;
pub use commands::mcp::{
    McpAddJsonRequest, McpAddRequest, McpGetRequest, McpRemoveRequest, McpScope, McpTransport,
};
pub use commands::print::{ClaudeInputFormat, ClaudeOutputFormat, ClaudePrintRequest};
pub use stream_json::{parse_stream_json_lines, StreamJsonLine, StreamJsonLineOutcome};

pub use process::CommandOutput;
