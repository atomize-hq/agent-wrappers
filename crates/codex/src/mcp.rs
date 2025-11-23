//! Design sketch for Workstream E (MCP + App Server).
//!
//! This module outlines the API surface we plan to ship for `codex mcp-server`
//! and `codex app-server`, both of which speak JSON-RPC 2.0 over stdio. The
//! actual process spawning will rely on Workstream A's env-prep helper to
//! resolve the bundled Codex binary and app-scoped `CODEX_HOME`, so the
//! structs here focus on request/response shapes and lifecycle hooks.
//!
//! ## Transport and lifecycle
//! - We spawn `codex mcp-server` or `codex app-server` with `kill_on_drop` and
//!   wire stdin/stdout to a JSON-RPC loop. The transport owns a reader task
//!   that demuxes responses vs. notifications and a writer task that serializes
//!   requests; callers get `RequestId`s back for cancellation.
//! - Immediately after spawn we send `initialize` (with client info +
//!   capabilities) and wait for success before exposing the handle. On shutdown
//!   we emit `shutdown` followed by `exit`, then kill the child if it lingers.
//! - Cancellation uses JSON-RPC's `$/cancelRequest` for in-flight calls; dropping
//!   the server handle or transport tears down the child process.
//!
//! ## codex mcp-server
//! - Tool entrypoints are `codex/codex` (new session) and `codex/codex-reply`
//!   (continue by `conversation_id`). Params mirror the CLI: `prompt`,
//!   optional `model`, `cwd`, `sandbox`, `approval_policy`, and a free-form
//!   `config` map to forward structured config entries.
//! - Notifications arrive via `codex/event` and are normalized into
//!   [`CodexEvent`]. We expect at least `task_complete`, approvals (`exec` /
//!   `apply`), cancellations, and error emissions. Unknown notifications stay
//!   available via the `Raw` variant.
//! - Each call returns a [`CallHandle`] with a `response` channel (final
//!   `CodexCallResult`) and a live `events` stream for `codex/event` payloads.
//!   Approval prompts surface as [`ApprovalRequest`] values; callers respond via
//!   a follow-up request (likely `codex/approval`) carrying an
//!   [`ApprovalDecision`].
//!
//! ## codex app-server
//! - After `initialize`, callers drive threads and turns with
//!   `thread/start`, `thread/resume`, `turn/start`, and `turn/interrupt`. The
//!   params are intentionally thin wrappers around the JSON-RPC payloads so we
//!   can pass through any future keys unchanged.
//! - Notifications include `task_complete` and `item` events; we keep them
//!   generic (`serde_json::Value`) for now but will project them onto the
//!   Workstream D thread/item types once they land. Raw notifications stay
//!   available for forward compatibility.
//! - Each thread/turn call returns a [`CallHandle`] with an [`AppNotification`]
//!   stream; the caller can cancel by dropping the handle or issuing an
//!   explicit cancel against the recorded `RequestId`.
//!
//! ## Error model
//! - [`McpError`] captures spawn/handshake failures, JSON-RPC errors from the
//!   server, serialization problems, channel drops, and timeouts. We keep the
//!   final error type explicit so downstream code can distinguish protocol
//!   errors from local IO failures.

use std::{collections::BTreeMap, ffi::OsString, io, path::PathBuf, time::Duration};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use tokio::sync::{mpsc, oneshot};

/// JSON-RPC method name used to initialize MCP servers.
pub const METHOD_INITIALIZE: &str = "initialize";
/// JSON-RPC method name used to shut down MCP servers.
pub const METHOD_SHUTDOWN: &str = "shutdown";
/// JSON-RPC method name used after shutdown to signal exit.
pub const METHOD_EXIT: &str = "exit";
/// JSON-RPC cancellation method per the spec.
pub const METHOD_CANCEL: &str = "$/cancelRequest";

/// Method names exposed by `codex mcp-server`.
pub const METHOD_CODEX: &str = "codex/codex";
/// Method names exposed by `codex mcp-server` for follow-up prompts.
pub const METHOD_CODEX_REPLY: &str = "codex/codex-reply";
/// Notification channel emitted by `codex mcp-server`.
pub const METHOD_CODEX_EVENT: &str = "codex/event";
/// Expected approval response hook (server-specific; confirmed during E2).
pub const METHOD_CODEX_APPROVAL: &str = "codex/approval";

/// Method names exposed by `codex app-server`.
pub const METHOD_THREAD_START: &str = "thread/start";
/// Resume an existing thread.
pub const METHOD_THREAD_RESUME: &str = "thread/resume";
/// Start a new turn on a thread.
pub const METHOD_TURN_START: &str = "turn/start";
/// Interrupt an active turn.
pub const METHOD_TURN_INTERRUPT: &str = "turn/interrupt";

/// Unique identifier for JSON-RPC calls.
pub type RequestId = u64;

/// Stream of notifications surfaced alongside a JSON-RPC response.
pub type EventStream<T> = mpsc::UnboundedReceiver<T>;

/// Shared launch configuration for stdio MCP/app-server processes.
///
/// The Workstream A env-prep helper should populate `binary`, `code_home`, and
/// baseline environment entries. Callers can layer additional `env` entries for
/// per-call overrides (e.g., `RUST_LOG`). `mirror_stdio` controls whether raw
/// stdout/stderr should be mirrored to the host console in addition to being
/// parsed as JSON-RPC.
#[derive(Clone, Debug)]
pub struct StdioServerConfig {
    pub binary: PathBuf,
    pub code_home: Option<PathBuf>,
    pub current_dir: Option<PathBuf>,
    pub env: Vec<(OsString, OsString)>,
    pub mirror_stdio: bool,
    pub startup_timeout: Duration,
}

/// Client metadata attached to the `initialize` request.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClientInfo {
    pub name: String,
    pub version: String,
}

/// Parameters for `codex/codex` (new session).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CodexCallParams {
    pub prompt: String,
    pub model: Option<String>,
    pub cwd: Option<PathBuf>,
    pub sandbox: Option<String>,
    pub approval_policy: Option<String>,
    #[serde(default)]
    pub config: BTreeMap<String, Value>,
}

/// Parameters for `codex/codex-reply` (continue an existing conversation).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CodexReplyParams {
    pub conversation_id: String,
    pub prompt: String,
    pub model: Option<String>,
    pub cwd: Option<PathBuf>,
    pub sandbox: Option<String>,
    pub approval_policy: Option<String>,
    #[serde(default)]
    pub config: BTreeMap<String, Value>,
}

/// Classification for approval prompts surfaced by the MCP server.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ApprovalKind {
    Exec,
    Apply,
    Unknown(String),
}

/// Approval request emitted as part of a `codex/event` notification.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub approval_id: String,
    pub kind: ApprovalKind,
    /// Full payload from the server so callers can render UI or inspect diffs/commands.
    pub payload: Value,
}

/// Decision payload sent back to the MCP server in response to an approval prompt.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ApprovalDecision {
    Approve {
        approval_id: String,
    },
    Reject {
        approval_id: String,
        reason: Option<String>,
    },
}

/// Notification emitted by `codex/event`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CodexEvent {
    TaskComplete {
        conversation_id: String,
        result: Value,
    },
    ApprovalRequired(ApprovalRequest),
    Cancelled {
        conversation_id: Option<String>,
        reason: Option<String>,
    },
    Error {
        message: String,
        data: Option<Value>,
    },
    Raw {
        method: String,
        params: Value,
    },
}

/// Final response payload for `codex/codex` or `codex/codex-reply`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CodexCallResult {
    pub conversation_id: String,
    pub output: Value,
}

/// Handle returned for each codex call, bundling response and notifications.
#[derive(Debug)]
pub struct CodexCallHandle {
    pub request_id: RequestId,
    pub events: EventStream<CodexEvent>,
    pub response: oneshot::Receiver<Result<CodexCallResult, McpError>>,
}

/// Parameters for `thread/start`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ThreadStartParams {
    pub thread_id: Option<String>,
    #[serde(default)]
    pub metadata: Value,
}

/// Parameters for `thread/resume`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ThreadResumeParams {
    pub thread_id: String,
}

/// Parameters for `turn/start`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TurnStartParams {
    pub thread_id: String,
    pub prompt: String,
    pub model: Option<String>,
    #[serde(default)]
    pub config: BTreeMap<String, Value>,
}

/// Parameters for `turn/interrupt`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TurnInterruptParams {
    pub thread_id: Option<String>,
    pub turn_id: String,
}

/// Notification emitted by the app-server.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AppNotification {
    TaskComplete {
        thread_id: String,
        turn_id: Option<String>,
        result: Value,
    },
    Item {
        thread_id: String,
        turn_id: Option<String>,
        item: Value,
    },
    Error {
        message: String,
        data: Option<Value>,
    },
    Raw {
        method: String,
        params: Value,
    },
}

/// Handle returned for each app-server call, bundling response and notifications.
#[derive(Debug)]
pub struct AppCallHandle {
    pub request_id: RequestId,
    pub events: EventStream<AppNotification>,
    pub response: oneshot::Receiver<Result<Value, McpError>>,
}

/// Errors surfaced while managing MCP/app-server transports.
#[derive(Debug, Error)]
pub enum McpError {
    #[error("failed to spawn `{command}`: {source}")]
    Spawn {
        command: String,
        #[source]
        source: io::Error,
    },
    #[error("server did not respond to initialize: {0}")]
    Handshake(String),
    #[error("transport task failed: {0}")]
    Transport(String),
    #[error("server returned JSON-RPC error {code}: {message}")]
    Rpc {
        code: i64,
        message: String,
        data: Option<Value>,
    },
    #[error("server reported an error: {0}")]
    Server(String),
    #[error("request was cancelled")]
    Cancelled,
    #[error("timed out after {0:?}")]
    Timeout(Duration),
    #[error("serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("transport channel closed unexpectedly")]
    ChannelClosed,
}
