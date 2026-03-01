use std::{collections::HashSet, io, path::PathBuf, sync::Arc, time::Duration};

use serde_json::{json, Value};
use thiserror::Error;

use super::{
    AppCallHandle, ApprovalDecision, ClientInfo, CodexCallHandle, CodexCallParams, CodexCallResult,
    CodexReplyParams, InitializeParams, RequestId, StdioServerConfig, METHOD_CODEX,
    METHOD_CODEX_APPROVAL, METHOD_THREAD_FORK, METHOD_THREAD_LIST, METHOD_THREAD_RESUME,
    METHOD_THREAD_START, METHOD_TURN_INTERRUPT, METHOD_TURN_START,
};

use super::jsonrpc::{map_response, JsonRpcTransport};

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
        Self::with_capabilities(config, client, Value::Object(Default::default())).await
    }

    /// Launch with explicit capabilities to send during `initialize`.
    pub async fn with_capabilities(
        config: StdioServerConfig,
        client: ClientInfo,
        capabilities: Value,
    ) -> Result<Self, McpError> {
        let capabilities = match capabilities {
            Value::Null => Value::Object(Default::default()),
            other => other,
        };
        let transport = JsonRpcTransport::spawn_mcp(config).await?;
        let params = InitializeParams {
            client,
            protocol_version: "2024-11-05".to_string(),
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
        self.invoke_tool_call("codex", serde_json::to_value(params)?)
            .await
    }

    /// Continue an existing conversation via `codex/codex-reply`.
    pub async fn codex_reply(&self, params: CodexReplyParams) -> Result<CodexCallHandle, McpError> {
        self.invoke_tool_call("codex-reply", serde_json::to_value(params)?)
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

    async fn invoke_tool_call(
        &self,
        tool_name: &str,
        arguments: Value,
    ) -> Result<CodexCallHandle, McpError> {
        let events = self.transport.register_codex_listener().await;
        let request = json!({
            "name": tool_name,
            "arguments": arguments,
        });
        let (request_id, raw_response) = self.transport.request(METHOD_CODEX, request).await?;
        let response = map_response::<CodexCallResult>(raw_response);

        Ok(CodexCallHandle {
            request_id,
            events,
            response,
        })
    }
}

/// Client wrapper around the stdio app-server.
pub struct CodexAppServer {
    transport: Arc<JsonRpcTransport>,
}

impl CodexAppServer {
    /// Launch `codex app-server`, issue `initialize`, and return a connected handle.
    pub async fn start(config: StdioServerConfig, client: ClientInfo) -> Result<Self, McpError> {
        Self::with_capabilities(config, client, Value::Object(Default::default())).await
    }

    /// Launch with `capabilities.experimentalApi=true` in the `initialize` handshake.
    pub async fn start_experimental(
        config: StdioServerConfig,
        client: ClientInfo,
    ) -> Result<Self, McpError> {
        Self::with_capabilities(config, client, json!({ "experimentalApi": true })).await
    }

    /// Launch with explicit capabilities to send during `initialize`.
    pub async fn with_capabilities(
        config: StdioServerConfig,
        client: ClientInfo,
        capabilities: Value,
    ) -> Result<Self, McpError> {
        let capabilities = match capabilities {
            Value::Null => Value::Object(Default::default()),
            other => other,
        };
        let transport = JsonRpcTransport::spawn_app(config).await?;
        let params = InitializeParams {
            client,
            protocol_version: "2024-11-05".to_string(),
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

    /// Start a new thread (or use a provided ID) via `thread/start`.
    pub async fn thread_start(
        &self,
        params: super::ThreadStartParams,
    ) -> Result<AppCallHandle, McpError> {
        self.invoke_app_call(METHOD_THREAD_START, serde_json::to_value(params)?)
            .await
    }

    /// Resume an existing thread via `thread/resume`.
    pub async fn thread_resume(
        &self,
        params: super::ThreadResumeParams,
    ) -> Result<AppCallHandle, McpError> {
        self.invoke_app_call(METHOD_THREAD_RESUME, serde_json::to_value(params)?)
            .await
    }

    /// List threads via `thread/list`.
    pub async fn thread_list(
        &self,
        params: super::ThreadListParams,
    ) -> Result<super::ThreadListResponse, McpError> {
        let (_, rx) = self
            .transport
            .request(METHOD_THREAD_LIST, serde_json::to_value(params)?)
            .await?;
        let mapped = map_response::<super::ThreadListResponse>(rx);
        match mapped.await {
            Ok(result) => result,
            Err(_) => Err(McpError::ChannelClosed),
        }
    }

    /// Fork an existing thread via `thread/fork`.
    pub async fn thread_fork(
        &self,
        params: super::ThreadForkParams,
    ) -> Result<super::ThreadForkResponse, McpError> {
        let (_, rx) = self
            .transport
            .request(METHOD_THREAD_FORK, serde_json::to_value(params)?)
            .await?;
        let mapped = map_response::<super::ThreadForkResponse>(rx);
        match mapped.await {
            Ok(result) => result,
            Err(_) => Err(McpError::ChannelClosed),
        }
    }

    /// Start a new turn on a thread via `turn/start`.
    pub async fn turn_start(
        &self,
        params: super::TurnStartParams,
    ) -> Result<AppCallHandle, McpError> {
        self.invoke_app_call(METHOD_TURN_START, serde_json::to_value(params)?)
            .await
    }

    /// Start a new turn on a thread via `turn/start` (pinned fork flow subset).
    pub async fn turn_start_v2(
        &self,
        params: super::TurnStartParamsV2,
    ) -> Result<AppCallHandle, McpError> {
        self.invoke_app_call(METHOD_TURN_START, serde_json::to_value(params)?)
            .await
    }

    /// Select the deterministic "last" thread id using `thread/list` paging and tuple ordering.
    pub async fn select_last_thread_id(&self, cwd: PathBuf) -> Result<Option<String>, McpError> {
        let mut cursor: Option<String> = None;
        let mut seen_cursors: HashSet<String> = HashSet::new();
        let mut best: Option<(i64, i64, String)> = None;

        loop {
            let page = self
                .thread_list(super::ThreadListParams {
                    cwd: Some(cwd.clone()),
                    cursor: cursor.clone(),
                    limit: Some(100),
                    sort_key: Some(super::ThreadListSortKey::UpdatedAt),
                    archived: None,
                    model_providers: None,
                    source_kinds: None,
                })
                .await?;

            for thread in page.data {
                let candidate = (thread.updated_at, thread.created_at, thread.id);
                let should_replace = match best.as_ref() {
                    None => true,
                    Some(current) => (candidate.0, candidate.1, &candidate.2)
                        > (current.0, current.1, &current.2),
                };

                if should_replace {
                    best = Some(candidate);
                }
            }

            let Some(next_cursor) = page.next_cursor else {
                break;
            };

            if !seen_cursors.insert(next_cursor.clone()) {
                return Err(McpError::Transport(format!(
                    "thread/list pagination cursor repeated: {next_cursor}"
                )));
            }
            cursor = Some(next_cursor);
        }

        Ok(best.map(|(_, _, id)| id))
    }

    /// Interrupt an active turn via `turn/interrupt`.
    pub async fn turn_interrupt(
        &self,
        params: super::TurnInterruptParams,
    ) -> Result<AppCallHandle, McpError> {
        self.invoke_app_call(METHOD_TURN_INTERRUPT, serde_json::to_value(params)?)
            .await
    }

    /// Request cancellation for a pending call.
    pub fn cancel(&self, request_id: RequestId) -> Result<(), McpError> {
        self.transport.cancel(request_id)
    }

    /// Gracefully shut down the app-server.
    pub async fn shutdown(&self) -> Result<(), McpError> {
        self.transport.shutdown().await
    }

    async fn invoke_app_call(
        &self,
        method: &str,
        params: Value,
    ) -> Result<AppCallHandle, McpError> {
        let events = self.transport.register_app_listener().await;
        let (request_id, raw_response) = self.transport.request(method, params).await?;
        let response = map_response::<Value>(raw_response);

        Ok(AppCallHandle {
            request_id,
            events,
            response,
        })
    }
}
