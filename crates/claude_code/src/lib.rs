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
pub use client::ClaudeSetupTokenSession;
pub use commands::command::ClaudeCommandRequest;
pub use commands::doctor::ClaudeDoctorRequest;
pub use commands::mcp::{
    McpAddFromClaudeDesktopRequest, McpAddJsonRequest, McpAddRequest, McpGetRequest,
    McpRemoveRequest, McpScope, McpServeRequest, McpTransport,
};
pub use commands::plugin::{
    PluginDisableRequest, PluginEnableRequest, PluginInstallRequest, PluginListRequest,
    PluginManifestMarketplaceRequest, PluginManifestRequest, PluginMarketplaceAddRequest,
    PluginMarketplaceListRequest, PluginMarketplaceRemoveRequest, PluginMarketplaceRepoRequest,
    PluginMarketplaceRequest, PluginMarketplaceUpdateRequest, PluginRequest,
    PluginUninstallRequest, PluginUpdateRequest, PluginValidateRequest,
};
pub use commands::print::{
    ClaudeChromeMode, ClaudeInputFormat, ClaudeOutputFormat, ClaudePrintRequest,
};
pub use commands::setup_token::ClaudeSetupTokenRequest;
pub use commands::update::ClaudeUpdateRequest;
pub use error::{ClaudeCodeError, StreamJsonLineError};
pub use stream_json::{parse_stream_json_lines, StreamJsonLine, StreamJsonLineOutcome};
pub use stream_json::{
    ClaudeStreamEvent, ClaudeStreamJsonErrorCode, ClaudeStreamJsonEvent,
    ClaudeStreamJsonParseError, ClaudeStreamJsonParser,
};

pub use process::CommandOutput;
