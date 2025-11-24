//! Launch and interact with `codex mcp-server` over stdio JSON-RPC.
//!
//! The MCP server exposes two tool entrypoints:
//! - `codex/codex`: start a new Codex session with a prompt.
//! - `codex/codex-reply`: continue an existing session by conversation ID.
//!
//! This module spawns the MCP server, sends requests over stdio, and streams
//! `codex/event` notifications (task completion, approvals, cancellations,
//! errors). Requests can be cancelled via JSON-RPC `$ /cancelRequest`.

use std::{
    collections::{BTreeMap, HashMap},
    ffi::OsString,
    io,
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStderr, ChildStdin, ChildStdout, Command},
    sync::{mpsc, oneshot, Mutex},
    task::JoinHandle,
    time,
};
use tracing::{debug, warn};

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

/// Parameters for the initial `initialize` handshake.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InitializeParams {
    pub client: ClientInfo,
    #[serde(default)]
    pub capabilities: Value,
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

/// Client wrapper around the stdio MCP server.
pub struct CodexMcpServer {
    transport: Arc<JsonRpcTransport>,
}

impl CodexMcpServer {
    /// Launch `codex mcp-server`, issue `initialize`, and return a connected handle.
    pub async fn start(config: StdioServerConfig, client: ClientInfo) -> Result<Self, McpError> {
        Self::with_capabilities(config, client, Value::Null).await
    }

    /// Launch with explicit capabilities to send during `initialize`.
    pub async fn with_capabilities(
        config: StdioServerConfig,
        client: ClientInfo,
        capabilities: Value,
    ) -> Result<Self, McpError> {
        let transport = JsonRpcTransport::spawn_mcp(config).await?;
        let params = InitializeParams {
            client,
            capabilities,
        };

        transport
            .initialize(params, transport.startup_timeout())
            .await
            .map_err(|err| McpError::Handshake(err.to_string()))?;

        Ok(Self {
            transport: Arc::new(transport),
        })
    }

    /// Send a new Codex prompt via `codex/codex`.
    pub async fn codex(&self, params: CodexCallParams) -> Result<CodexCallHandle, McpError> {
        self.invoke_codex_call(METHOD_CODEX, serde_json::to_value(params)?)
            .await
    }

    /// Continue an existing conversation via `codex/codex-reply`.
    pub async fn codex_reply(&self, params: CodexReplyParams) -> Result<CodexCallHandle, McpError> {
        self.invoke_codex_call(METHOD_CODEX_REPLY, serde_json::to_value(params)?)
            .await
    }

    /// Send an approval decision back to the MCP server.
    pub async fn send_approval(&self, decision: ApprovalDecision) -> Result<(), McpError> {
        let (_, rx) = self
            .transport
            .request(METHOD_CODEX_APPROVAL, serde_json::to_value(decision)?)
            .await?;

        match rx.await {
            Ok(Ok(_)) => Ok(()),
            Ok(Err(err)) => Err(err),
            Err(_) => Err(McpError::ChannelClosed),
        }
    }

    /// Request cancellation for a pending call.
    pub fn cancel(&self, request_id: RequestId) -> Result<(), McpError> {
        self.transport.cancel(request_id)
    }

    /// Gracefully shut down the MCP server.
    pub async fn shutdown(&self) -> Result<(), McpError> {
        self.transport.shutdown().await
    }

    async fn invoke_codex_call(
        &self,
        method: &str,
        params: Value,
    ) -> Result<CodexCallHandle, McpError> {
        let events = self.transport.register_codex_listener().await;
        let (request_id, raw_response) = self.transport.request(method, params).await?;
        let response = map_response::<CodexCallResult>(raw_response);

        Ok(CodexCallHandle {
            request_id,
            events,
            response,
        })
    }
}

/// Internal transport that handles stdio JSON-RPC.
struct JsonRpcTransport {
    writer: mpsc::UnboundedSender<String>,
    pending: Arc<Mutex<HashMap<RequestId, oneshot::Sender<Result<Value, McpError>>>>>,
    codex_events: Arc<Mutex<Vec<mpsc::UnboundedSender<CodexEvent>>>>,
    next_id: AtomicU64,
    tasks: Vec<JoinHandle<()>>,
    child: Arc<Mutex<Option<Child>>>,
    startup_timeout: Duration,
}

impl JsonRpcTransport {
    async fn spawn_mcp(config: StdioServerConfig) -> Result<Self, McpError> {
        let mut command = Command::new(&config.binary);
        command
            .arg("mcp-server")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);

        if let Some(dir) = &config.current_dir {
            command.current_dir(dir);
        }

        if let Some(code_home) = &config.code_home {
            command.env("CODEX_HOME", code_home);
        }

        for (key, value) in &config.env {
            command.env(key, value);
        }

        let mut child = command.spawn().map_err(|source| McpError::Spawn {
            command: format!("{command:?}"),
            source,
        })?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| McpError::Transport("child stdout unavailable".into()))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| McpError::Transport("child stdin unavailable".into()))?;
        let stderr = child.stderr.take();

        let pending = Arc::new(Mutex::new(HashMap::new()));
        let codex_events = Arc::new(Mutex::new(Vec::new()));
        let (writer_tx, writer_rx) = mpsc::unbounded_channel();

        let writer_handle = tokio::spawn(writer_task(stdin, writer_rx));
        let reader_handle = tokio::spawn(reader_task(
            stdout,
            pending.clone(),
            codex_events.clone(),
            config.mirror_stdio,
        ));

        let stderr_handle =
            stderr.map(|stderr| tokio::spawn(stderr_task(stderr, config.mirror_stdio)));

        let mut tasks = vec![writer_handle, reader_handle];
        if let Some(handle) = stderr_handle {
            tasks.push(handle);
        }

        Ok(Self {
            writer: writer_tx,
            pending,
            codex_events,
            next_id: AtomicU64::new(1),
            tasks,
            child: Arc::new(Mutex::new(Some(child))),
            startup_timeout: config.startup_timeout,
        })
    }

    async fn initialize(
        &self,
        params: InitializeParams,
        timeout: Duration,
    ) -> Result<Value, McpError> {
        let (_, rx) = self
            .request(METHOD_INITIALIZE, serde_json::to_value(params)?)
            .await?;
        recv_with_timeout(rx, timeout).await
    }

    async fn request(
        &self,
        method: &str,
        params: Value,
    ) -> Result<(RequestId, oneshot::Receiver<Result<Value, McpError>>), McpError> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let message = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        });
        let serialized = serde_json::to_string(&message)?;
        let (tx, rx) = oneshot::channel();

        {
            let mut guard = self.pending.lock().await;
            guard.insert(id, tx);
        }

        if self.writer.send(serialized).is_err() {
            let mut guard = self.pending.lock().await;
            guard.remove(&id);
            return Err(McpError::ChannelClosed);
        }

        Ok((id, rx))
    }

    async fn register_codex_listener(&self) -> EventStream<CodexEvent> {
        let (tx, rx) = mpsc::unbounded_channel();
        let mut guard = self.codex_events.lock().await;
        guard.push(tx);
        rx
    }

    fn cancel(&self, request_id: RequestId) -> Result<(), McpError> {
        let message = serde_json::json!({
            "jsonrpc": "2.0",
            "method": METHOD_CANCEL,
            "params": { "id": request_id }
        });
        let serialized = serde_json::to_string(&message)?;
        self.writer
            .send(serialized)
            .map_err(|_| McpError::ChannelClosed)
    }

    async fn shutdown(&self) -> Result<(), McpError> {
        if let Ok((_, rx)) = self.request(METHOD_SHUTDOWN, Value::Null).await {
            let _ = recv_with_timeout(rx, Duration::from_secs(5)).await;
        }

        let exit_message = serde_json::json!({
            "jsonrpc": "2.0",
            "method": METHOD_EXIT,
            "params": Value::Null
        });

        let _ = self
            .writer
            .send(serde_json::to_string(&exit_message).unwrap_or_default());

        Ok(())
    }

    fn startup_timeout(&self) -> Duration {
        self.startup_timeout
    }
}

impl Drop for JsonRpcTransport {
    fn drop(&mut self) {
        for handle in &self.tasks {
            handle.abort();
        }

        if let Ok(mut child_guard) = self.child.try_lock() {
            if let Some(mut child) = child_guard.take() {
                let _ = child.start_kill();
            }
        }
    }
}

#[derive(Debug, Deserialize)]
struct RpcResponse {
    id: Value,
    result: Option<Value>,
    error: Option<RpcError>,
}

#[derive(Debug, Deserialize)]
struct RpcNotification {
    method: String,
    params: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct RpcError {
    code: i64,
    message: String,
    data: Option<Value>,
}

async fn writer_task(mut stdin: ChildStdin, mut rx: mpsc::UnboundedReceiver<String>) {
    while let Some(message) = rx.recv().await {
        if stdin.write_all(message.as_bytes()).await.is_err() {
            break;
        }
        if stdin.write_all(b"\n").await.is_err() {
            break;
        }
        let _ = stdin.flush().await;
    }

    let _ = stdin.shutdown().await;
}

async fn reader_task(
    stdout: ChildStdout,
    pending: Arc<Mutex<HashMap<RequestId, oneshot::Sender<Result<Value, McpError>>>>>,
    codex_events: Arc<Mutex<Vec<mpsc::UnboundedSender<CodexEvent>>>>,
    mirror_stdio: bool,
) {
    let mut lines = BufReader::new(stdout).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        if mirror_stdio {
            eprintln!("[mcp stdout] {line}");
        }

        if line.trim().is_empty() {
            continue;
        }

        let value: Value = match serde_json::from_str(&line) {
            Ok(value) => value,
            Err(err) => {
                warn!("failed to parse MCP stdout as JSON: {err}");
                continue;
            }
        };

        match decode_message(value) {
            Some(Incoming::Response(response)) => {
                handle_response(response, &pending).await;
            }
            Some(Incoming::Notification(notification)) => {
                if notification.method == METHOD_CODEX_EVENT {
                    let params = notification.params.unwrap_or(Value::Null);
                    let event = parse_codex_event(&params).unwrap_or(CodexEvent::Raw {
                        method: METHOD_CODEX_EVENT.to_string(),
                        params,
                    });
                    broadcast_codex_event(event, &codex_events).await;
                }
            }
            None => {
                warn!("received malformed MCP message");
            }
        }
    }

    let mut guard = pending.lock().await;
    for (_, tx) in guard.drain() {
        let _ = tx.send(Err(McpError::ChannelClosed));
    }
}

async fn stderr_task(stderr: ChildStderr, mirror_stdio: bool) {
    let mut lines = BufReader::new(stderr).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        if mirror_stdio {
            eprintln!("[mcp stderr] {line}");
        } else {
            debug!("mcp stderr: {line}");
        }
    }
}

fn map_response<T: for<'a> Deserialize<'a> + Send + 'static>(
    rx: oneshot::Receiver<Result<Value, McpError>>,
) -> oneshot::Receiver<Result<T, McpError>> {
    let (tx, mapped_rx) = oneshot::channel();
    tokio::spawn(async move {
        let mapped = match rx.await {
            Ok(Ok(value)) => serde_json::from_value::<T>(value).map_err(McpError::from),
            Ok(Err(err)) => Err(err),
            Err(_) => Err(McpError::ChannelClosed),
        };
        let _ = tx.send(mapped);
    });
    mapped_rx
}

async fn recv_with_timeout(
    rx: oneshot::Receiver<Result<Value, McpError>>,
    timeout: Duration,
) -> Result<Value, McpError> {
    match time::timeout(timeout, rx).await {
        Ok(Ok(Ok(value))) => Ok(value),
        Ok(Ok(Err(err))) => Err(err),
        Ok(Err(_)) => Err(McpError::ChannelClosed),
        Err(_) => Err(McpError::Timeout(timeout)),
    }
}

#[derive(Debug)]
enum Incoming {
    Response(RpcResponse),
    Notification(RpcNotification),
}

fn decode_message(value: Value) -> Option<Incoming> {
    let is_notification = value.get("id").is_none() && value.get("method").is_some();
    if is_notification {
        let notification: RpcNotification = serde_json::from_value(value).ok()?;
        return Some(Incoming::Notification(notification));
    }

    let is_response = value.get("id").is_some();
    if is_response {
        let response: RpcResponse = serde_json::from_value(value).ok()?;
        return Some(Incoming::Response(response));
    }

    None
}

async fn handle_response(
    response: RpcResponse,
    pending: &Arc<Mutex<HashMap<RequestId, oneshot::Sender<Result<Value, McpError>>>>>,
) {
    let Some(id) = parse_request_id(&response.id) else {
        warn!("received response without numeric id");
        return;
    };

    let sender = { pending.lock().await.remove(&id) };
    let Some(tx) = sender else {
        return;
    };

    if let Some(err) = response.error {
        let mapped = if err.code == -32800 {
            McpError::Cancelled
        } else {
            McpError::Rpc {
                code: err.code,
                message: err.message,
                data: err.data,
            }
        };
        let _ = tx.send(Err(mapped));
    } else if let Some(result) = response.result {
        let _ = tx.send(Ok(result));
    } else {
        let _ = tx.send(Err(McpError::Transport(
            "response missing result and error".into(),
        )));
    }
}

fn parse_request_id(value: &Value) -> Option<RequestId> {
    if let Some(num) = value.as_u64() {
        return Some(num);
    }

    value.as_str().and_then(|s| s.parse::<RequestId>().ok())
}

fn parse_codex_event(value: &Value) -> Option<CodexEvent> {
    let event_type = value.get("type")?.as_str()?;
    match event_type {
        "task_complete" => Some(CodexEvent::TaskComplete {
            conversation_id: value
                .get("conversation_id")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            result: value.get("result").cloned().unwrap_or(Value::Null),
        }),
        "approval_required" | "approval" => {
            let approval_id = value
                .get("approval_id")
                .or_else(|| value.get("id"))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let raw_kind = value
                .get("kind")
                .or_else(|| value.get("approval_kind"))
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string();

            let kind = match raw_kind.to_lowercase().as_str() {
                "exec" => ApprovalKind::Exec,
                "apply" => ApprovalKind::Apply,
                other => ApprovalKind::Unknown(other.to_string()),
            };

            Some(CodexEvent::ApprovalRequired(ApprovalRequest {
                approval_id,
                kind,
                payload: value.clone(),
            }))
        }
        "cancelled" | "canceled" => Some(CodexEvent::Cancelled {
            conversation_id: value
                .get("conversation_id")
                .and_then(Value::as_str)
                .map(|s| s.to_string()),
            reason: value
                .get("reason")
                .and_then(Value::as_str)
                .map(|s| s.to_string()),
        }),
        "error" => Some(CodexEvent::Error {
            message: value
                .get("message")
                .or_else(|| value.get("error"))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            data: value.get("data").cloned(),
        }),
        _ => None,
    }
}

async fn broadcast_codex_event(
    event: CodexEvent,
    sinks: &Arc<Mutex<Vec<mpsc::UnboundedSender<CodexEvent>>>>,
) {
    let mut guard = sinks.lock().await;
    guard.retain(|tx| tx.send(event.clone()).is_ok());
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, os::unix::fs::PermissionsExt};

    fn write_fake_mcp_server() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().expect("tempdir");
        let script_path = dir.path().join("fake-codex");
        let script = r#"#!/usr/bin/env python3
import json
import sys
import threading
import time

pending = {}

def send(payload):
    sys.stdout.write(json.dumps(payload) + "\n")
    sys.stdout.flush()

def handle_codex(req_id):
    pending[str(req_id)] = "pending"
    def worker():
        time.sleep(0.05)
        if pending.get(str(req_id)) == "cancelled":
            return
        send({"jsonrpc": "2.0", "method": "codex/event", "params": {"type": "approval_required", "approval_id": "ap-1", "kind": "exec", "payload": {"cmd": "echo hi"}}})
        time.sleep(0.05)
        if pending.get(str(req_id)) == "cancelled":
            return
        send({"jsonrpc": "2.0", "method": "codex/event", "params": {"type": "task_complete", "conversation_id": "conv-1", "result": {"ok": True}}})
        send({"jsonrpc": "2.0", "id": req_id, "result": {"conversation_id": "conv-1", "output": {"ok": True}}})
        pending.pop(str(req_id), None)
    threading.Thread(target=worker, daemon=True).start()

for line in sys.stdin:
    if not line.strip():
        continue
    msg = json.loads(line)
    method = msg.get("method")
    if method == "initialize":
        send({"jsonrpc": "2.0", "id": msg.get("id"), "result": {"ready": True}})
    elif method == "codex/codex" or method == "codex/codex-reply":
        handle_codex(msg.get("id"))
    elif method == "$/cancelRequest":
        target = msg.get("params", {}).get("id")
        pending[str(target)] = "cancelled"
        send({"jsonrpc": "2.0", "id": target, "error": {"code": -32800, "message": "cancelled"}})
    elif method == "shutdown":
        send({"jsonrpc": "2.0", "id": msg.get("id"), "result": {"ok": True}})
        break
    elif method == "exit":
        break
"#;

        fs::write(&script_path, script).expect("write script");
        let mut perms = fs::metadata(&script_path).expect("metadata").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script_path, perms).expect("chmod");
        (dir, script_path)
    }

    fn test_config(binary: PathBuf) -> StdioServerConfig {
        StdioServerConfig {
            binary,
            code_home: None,
            current_dir: None,
            env: Vec::new(),
            mirror_stdio: false,
            startup_timeout: Duration::from_secs(5),
        }
    }

    #[tokio::test]
    async fn codex_flow_streams_events_and_response() {
        let (_dir, script) = write_fake_mcp_server();
        let config = test_config(script);
        let client = ClientInfo {
            name: "tests".to_string(),
            version: "0.0.0".to_string(),
        };

        let server = CodexMcpServer::start(config, client)
            .await
            .expect("spawn server");

        let params = CodexCallParams {
            prompt: "hello".into(),
            model: None,
            cwd: None,
            sandbox: None,
            approval_policy: None,
            config: BTreeMap::new(),
        };

        let mut handle = server.codex(params).await.expect("codex call");

        let first_event = time::timeout(Duration::from_secs(2), handle.events.recv())
            .await
            .expect("event timeout")
            .expect("event value");
        match first_event {
            CodexEvent::ApprovalRequired(req) => {
                assert_eq!(req.approval_id, "ap-1");
                assert_eq!(req.kind, ApprovalKind::Exec);
            }
            other => panic!("unexpected event: {other:?}"),
        }

        let second_event = time::timeout(Duration::from_secs(2), handle.events.recv())
            .await
            .expect("event timeout")
            .expect("event value");
        assert!(
            matches!(second_event, CodexEvent::TaskComplete { conversation_id, .. } if conversation_id == "conv-1")
        );

        let response = time::timeout(Duration::from_secs(2), handle.response)
            .await
            .expect("response timeout")
            .expect("response recv");
        let response = response.expect("response ok");
        assert_eq!(response.conversation_id, "conv-1");
        assert_eq!(response.output, serde_json::json!({ "ok": true }));

        let _ = server.shutdown().await;
    }

    #[tokio::test]
    async fn canceling_request_returns_cancelled_error() {
        let (_dir, script) = write_fake_mcp_server();
        let config = test_config(script);
        let client = ClientInfo {
            name: "tests".to_string(),
            version: "0.0.0".to_string(),
        };

        let server = CodexMcpServer::start(config, client)
            .await
            .expect("spawn server");

        let params = CodexCallParams {
            prompt: "cancel me".into(),
            model: None,
            cwd: None,
            sandbox: None,
            approval_policy: None,
            config: BTreeMap::new(),
        };

        let handle = server.codex(params).await.expect("codex call");
        server.cancel(handle.request_id).expect("cancel send");

        let response = time::timeout(Duration::from_secs(2), handle.response)
            .await
            .expect("response timeout")
            .expect("recv");
        assert!(matches!(response, Err(McpError::Cancelled)));

        let _ = server.shutdown().await;
    }
}
