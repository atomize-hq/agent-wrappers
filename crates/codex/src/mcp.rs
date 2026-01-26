//! Launch and interact with Codex MCP + app servers using stored runtime definitions.
//!
//! - Spawn `codex mcp-server`, call `codex/codex` or `codex/codex-reply`, and stream
//!   `codex/event` notifications (task completion, approvals, cancellations, errors).
//! - Start `codex app-server` threads/turns and surface item/task_complete notifications.
//! - Manage `[mcp_servers]` and `[app_runtimes]` config entries, resolve them into launch-ready
//!   runtimes, and expose read-only APIs (including pooled app runtimes) without mutating stored
//!   config or thread metadata.
//! - Requests may be cancelled via the JSON-RPC `$ /cancelRequest` flow.
//!
//! The MCP server exposes two tool entrypoints:
//! - `codex/codex`: start a new Codex session with a prompt.
//! - `codex/codex-reply`: continue an existing session by conversation ID.
//!
//! The app-server supports `thread/start`, `thread/resume`, `turn/start`, and `turn/interrupt`
//! requests. Runtime and pool helpers keep resume hints/metadata intact while starting,
//! reusing, and stopping app-server instances.

use std::{
    collections::{BTreeMap, HashMap},
    env,
    ffi::OsString,
    fmt, fs, io,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStderr, ChildStdin, ChildStdout, Command},
    sync::{mpsc, oneshot, Mutex},
    task::JoinHandle,
    time,
};
use toml::{value::Table as TomlTable, Value as TomlValue};
use tracing::{debug, warn};

/// JSON-RPC method name used to initialize MCP servers.
pub const METHOD_INITIALIZE: &str = "initialize";
/// JSON-RPC method name used to shut down MCP servers.
pub const METHOD_SHUTDOWN: &str = "shutdown";
/// JSON-RPC method name used after shutdown to signal exit.
pub const METHOD_EXIT: &str = "exit";
/// JSON-RPC cancellation method per the spec.
pub const METHOD_CANCEL: &str = "$/cancelRequest";

/// Tool-call method exposed by the MCP server.
pub const METHOD_CODEX: &str = "tools/call";
/// Tool-call method for follow-up prompts (codex-reply).
pub const METHOD_CODEX_REPLY: &str = "tools/call";
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

/// Default config filename placed under CODEX_HOME.
pub const DEFAULT_CONFIG_FILE: &str = "config.toml";
const MCP_SERVERS_KEY: &str = "mcp_servers";
const APP_RUNTIMES_KEY: &str = "app_runtimes";

/// MCP server definition coupled with its name.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpServerEntry {
    pub name: String,
    pub definition: McpServerDefinition,
}

/// App runtime definition coupled with its name.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AppRuntimeEntry {
    pub name: String,
    pub definition: AppRuntimeDefinition,
}

/// JSON-serializable MCP server configuration stored under `[mcp_servers]`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpServerDefinition {
    pub transport: McpTransport,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<McpToolConfig>,
}

/// Supported transport definitions for MCP servers.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "transport", rename_all = "snake_case")]
pub enum McpTransport {
    Stdio(StdioServerDefinition),
    StreamableHttp(StreamableHttpDefinition),
}

/// Stdio transport configuration for an MCP server.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct StdioServerDefinition {
    pub command: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
}

/// HTTP transport configuration that supports streaming responses.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct StreamableHttpDefinition {
    pub url: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub headers: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bearer_env_var: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connect_timeout_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_timeout_ms: Option<u64>,
}

/// Tool allow/deny lists for a given MCP server.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpToolConfig {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub enabled: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub disabled: Vec<String>,
}

/// Stored definition for launching an app-server runtime.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AppRuntimeDefinition {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code_home: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_dir: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mirror_stdio: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub startup_timeout_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub binary: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub metadata: Value,
}

/// Input for adding or updating an app runtime entry.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AddAppRuntimeRequest {
    pub name: String,
    pub definition: AppRuntimeDefinition,
    #[serde(default)]
    pub overwrite: bool,
}

/// Resolved runtime configuration for an MCP server, ready for spawning or connecting.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpRuntimeServer {
    pub name: String,
    pub transport: McpRuntimeTransport,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<McpToolConfig>,
}

/// Transport-specific runtime configuration.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "transport", rename_all = "snake_case")]
pub enum McpRuntimeTransport {
    Stdio(StdioServerDefinition),
    StreamableHttp(ResolvedStreamableHttpDefinition),
}

/// HTTP runtime config with bearer tokens resolved from the environment.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResolvedStreamableHttpDefinition {
    pub url: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub headers: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bearer_env_var: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bearer_token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connect_timeout_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_timeout_ms: Option<u64>,
}

/// Launcher/connector wrapper around a resolved MCP runtime server.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpServerLauncher {
    pub name: String,
    pub transport: McpServerLauncherTransport,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<McpToolConfig>,
}

/// Transport-specific launcher/connector.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum McpServerLauncherTransport {
    Stdio(StdioLauncher),
    StreamableHttp(StreamableHttpConnector),
}

/// Prepared stdio launcher with merged env and startup timeout.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct StdioLauncher {
    pub command: PathBuf,
    pub args: Vec<String>,
    pub env: Vec<(OsString, OsString)>,
    pub current_dir: Option<PathBuf>,
    pub timeout: Duration,
    pub mirror_stdio: bool,
}

/// Prepared HTTP connector with resolved headers and timeouts.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct StreamableHttpConnector {
    pub url: String,
    pub headers: BTreeMap<String, String>,
    pub bearer_env_var: Option<String>,
    pub bearer_token: Option<String>,
    pub connect_timeout: Option<Duration>,
    pub request_timeout: Option<Duration>,
}

/// Input for adding or updating an MCP server entry.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AddServerRequest {
    pub name: String,
    pub definition: McpServerDefinition,
    #[serde(default)]
    pub overwrite: bool,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    #[serde(default)]
    pub bearer_token: Option<String>,
}

/// Result of logging into a server (auth token set in env var).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpLoginResult {
    pub server: String,
    pub env_var: Option<String>,
}

/// Result of clearing a stored auth token.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpLogoutResult {
    pub server: String,
    pub env_var: Option<String>,
    pub cleared: bool,
}

/// Errors surfaced while managing MCP config entries.
#[derive(Debug, Error)]
pub enum McpConfigError {
    #[error("failed to read {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to write {path}: {source}")]
    Write {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to create directory {path}: {source}")]
    CreateDir {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to parse {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
    #[error("config root at {path} must be a table")]
    InvalidRoot { path: PathBuf },
    #[error("`mcp_servers` must be a table in {path}")]
    InvalidServers { path: PathBuf },
    #[error("failed to decode mcp_servers: {source}")]
    DecodeServers {
        #[source]
        source: toml::de::Error,
    },
    #[error("`app_runtimes` must be a table in {path}")]
    InvalidAppRuntimes { path: PathBuf },
    #[error("failed to decode app_runtimes: {source}")]
    DecodeAppRuntimes {
        #[source]
        source: toml::de::Error,
    },
    #[error("failed to serialize config: {source}")]
    Serialize {
        #[source]
        source: toml::ser::Error,
    },
    #[error("server `{0}` already exists")]
    ServerAlreadyExists(String),
    #[error("server `{0}` not found")]
    ServerNotFound(String),
    #[error("server name may not be empty")]
    InvalidServerName,
    #[error("app runtime `{0}` already exists")]
    AppRuntimeAlreadyExists(String),
    #[error("app runtime `{0}` not found")]
    AppRuntimeNotFound(String),
    #[error("app runtime name may not be empty")]
    InvalidAppRuntimeName,
    #[error("invalid env var name `{name}`")]
    InvalidEnvVarName { name: String },
    #[error("server `{server}` missing bearer_env_var for auth token")]
    MissingBearerEnvVar { server: String },
    #[error("server `{server}` transport does not support login/logout")]
    UnsupportedAuthTransport { server: String },
}

impl From<McpServerEntry> for McpRuntimeServer {
    fn from(entry: McpServerEntry) -> Self {
        let McpServerEntry { name, definition } = entry;
        McpRuntimeServer::from_definition(name, definition)
    }
}

impl McpRuntimeServer {
    /// Builds a runtime config from a stored server definition.
    pub fn from_definition(name: impl Into<String>, definition: McpServerDefinition) -> Self {
        let McpServerDefinition {
            transport,
            description,
            tags,
            tools,
        } = definition;

        Self {
            name: name.into(),
            transport: McpRuntimeTransport::from_transport(transport),
            description,
            tags,
            tools,
        }
    }
}

impl McpRuntimeTransport {
    fn from_transport(transport: McpTransport) -> Self {
        match transport {
            McpTransport::Stdio(definition) => McpRuntimeTransport::Stdio(definition),
            McpTransport::StreamableHttp(definition) => {
                McpRuntimeTransport::StreamableHttp(resolve_streamable_http(definition))
            }
        }
    }
}

fn resolve_streamable_http(
    definition: StreamableHttpDefinition,
) -> ResolvedStreamableHttpDefinition {
    let StreamableHttpDefinition {
        url,
        headers,
        bearer_env_var,
        connect_timeout_ms,
        request_timeout_ms,
    } = definition;

    let mut headers = headers;
    let mut bearer_token = None;
    if let Some(env_var) = bearer_env_var.as_deref() {
        if let Ok(token) = env::var(env_var) {
            if !token.is_empty() {
                let has_auth_header = headers
                    .keys()
                    .any(|key| key.eq_ignore_ascii_case("authorization"));
                if !has_auth_header {
                    headers.insert("Authorization".into(), format!("Bearer {token}"));
                }
                bearer_token = Some(token);
            }
        }
    }

    ResolvedStreamableHttpDefinition {
        url,
        headers,
        bearer_env_var,
        bearer_token,
        connect_timeout_ms,
        request_timeout_ms,
    }
}

impl McpRuntimeServer {
    /// Converts a runtime server into a launcher/connector, merging stdio defaults.
    pub fn into_launcher(self, defaults: &StdioServerConfig) -> McpServerLauncher {
        let McpRuntimeServer {
            name,
            transport,
            description,
            tags,
            tools,
        } = self;

        let transport = match transport {
            McpRuntimeTransport::Stdio(def) => {
                McpServerLauncherTransport::Stdio(StdioLauncher::from_runtime(def, defaults))
            }
            McpRuntimeTransport::StreamableHttp(def) => {
                McpServerLauncherTransport::StreamableHttp(def.into())
            }
        };

        McpServerLauncher {
            name,
            transport,
            description,
            tags,
            tools,
        }
    }

    /// Convenience clone-preserving conversion to a launcher/connector.
    pub fn to_launcher(&self, defaults: &StdioServerConfig) -> McpServerLauncher {
        self.clone().into_launcher(defaults)
    }
}

impl StdioLauncher {
    fn from_runtime(definition: StdioServerDefinition, defaults: &StdioServerConfig) -> Self {
        let env = merge_stdio_env(
            defaults.code_home.as_deref(),
            &defaults.env,
            &definition.env,
        );

        Self {
            command: PathBuf::from(definition.command),
            args: definition.args,
            env,
            current_dir: defaults.current_dir.clone(),
            timeout: definition
                .timeout_ms
                .map(Duration::from_millis)
                .unwrap_or(defaults.startup_timeout),
            mirror_stdio: defaults.mirror_stdio,
        }
    }

    /// Builds a `tokio::process::Command` with merged env/dirs applied.
    pub fn command(&self) -> Command {
        let mut command = Command::new(&self.command);
        command
            .args(&self.args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);

        if let Some(dir) = &self.current_dir {
            command.current_dir(dir);
        }

        for (key, value) in &self.env {
            command.env(key, value);
        }

        command
    }
}

impl From<ResolvedStreamableHttpDefinition> for StreamableHttpConnector {
    fn from(definition: ResolvedStreamableHttpDefinition) -> Self {
        let ResolvedStreamableHttpDefinition {
            url,
            headers,
            bearer_env_var,
            bearer_token,
            connect_timeout_ms,
            request_timeout_ms,
        } = definition;

        Self {
            url,
            headers,
            bearer_env_var,
            bearer_token,
            connect_timeout: connect_timeout_ms.map(Duration::from_millis),
            request_timeout: request_timeout_ms.map(Duration::from_millis),
        }
    }
}

fn merge_stdio_env(
    code_home: Option<&Path>,
    base_env: &[(OsString, OsString)],
    runtime_env: &BTreeMap<String, String>,
) -> Vec<(OsString, OsString)> {
    let mut merged: HashMap<OsString, OsString> = HashMap::new();

    if let Some(code_home) = code_home {
        merged.insert(
            OsString::from("CODEX_HOME"),
            code_home.as_os_str().to_os_string(),
        );
    }

    for (key, value) in base_env {
        merged.insert(key.clone(), value.clone());
    }

    for (key, value) in runtime_env {
        merged.insert(OsString::from(key), OsString::from(value));
    }

    merged.into_iter().collect()
}

/// Summarized runtime metadata for listing available MCP servers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct McpRuntimeSummary {
    pub name: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub tools: Option<McpToolConfig>,
    pub transport: McpRuntimeSummaryTransport,
}

/// Transport kind used by a runtime.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum McpRuntimeSummaryTransport {
    Stdio,
    StreamableHttp,
}

impl From<&McpServerLauncher> for McpRuntimeSummary {
    fn from(launcher: &McpServerLauncher) -> Self {
        let transport = match launcher.transport {
            McpServerLauncherTransport::Stdio(_) => McpRuntimeSummaryTransport::Stdio,
            McpServerLauncherTransport::StreamableHttp(_) => {
                McpRuntimeSummaryTransport::StreamableHttp
            }
        };

        Self {
            name: launcher.name.clone(),
            description: launcher.description.clone(),
            tags: launcher.tags.clone(),
            tools: launcher.tools.clone(),
            transport,
        }
    }
}

/// Stored app runtime converted into launch-ready config with metadata intact.
#[derive(Clone, Debug, PartialEq)]
pub struct AppRuntime {
    pub name: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub metadata: Value,
    pub env: BTreeMap<String, String>,
    pub code_home: Option<PathBuf>,
    pub current_dir: Option<PathBuf>,
    pub mirror_stdio: Option<bool>,
    pub startup_timeout_ms: Option<u64>,
    pub binary: Option<PathBuf>,
}

impl From<AppRuntimeEntry> for AppRuntime {
    fn from(entry: AppRuntimeEntry) -> Self {
        let AppRuntimeEntry { name, definition } = entry;
        let AppRuntimeDefinition {
            description,
            tags,
            env,
            code_home,
            current_dir,
            mirror_stdio,
            startup_timeout_ms,
            binary,
            metadata,
        } = definition;

        Self {
            name,
            description,
            tags,
            metadata,
            env,
            code_home,
            current_dir,
            mirror_stdio,
            startup_timeout_ms,
            binary,
        }
    }
}

impl AppRuntime {
    /// Converts an app runtime into a launch-ready config using provided defaults.
    pub fn into_launcher(self, defaults: &StdioServerConfig) -> AppRuntimeLauncher {
        let code_home = self
            .code_home
            .clone()
            .or_else(|| defaults.code_home.clone());
        let env = merge_stdio_env(code_home.as_deref(), &defaults.env, &self.env);

        let config = StdioServerConfig {
            binary: self
                .binary
                .clone()
                .unwrap_or_else(|| defaults.binary.clone()),
            code_home,
            current_dir: self
                .current_dir
                .clone()
                .or_else(|| defaults.current_dir.clone()),
            env,
            mirror_stdio: self.mirror_stdio.unwrap_or(defaults.mirror_stdio),
            startup_timeout: self
                .startup_timeout_ms
                .map(Duration::from_millis)
                .unwrap_or(defaults.startup_timeout),
        };

        AppRuntimeLauncher {
            name: self.name,
            description: self.description,
            tags: self.tags,
            metadata: self.metadata,
            config,
        }
    }

    /// Convenience clone-preserving conversion to a launcher.
    pub fn to_launcher(&self, defaults: &StdioServerConfig) -> AppRuntimeLauncher {
        self.clone().into_launcher(defaults)
    }
}

/// Launch-ready stdio config bundled with app metadata.
#[derive(Clone, Debug)]
pub struct AppRuntimeLauncher {
    pub name: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub metadata: Value,
    pub config: StdioServerConfig,
}

/// Summarized app runtime metadata for listing.
#[derive(Clone, Debug, PartialEq)]
pub struct AppRuntimeSummary {
    pub name: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub metadata: Value,
}

impl From<&AppRuntimeLauncher> for AppRuntimeSummary {
    fn from(launcher: &AppRuntimeLauncher) -> Self {
        Self {
            name: launcher.name.clone(),
            description: launcher.description.clone(),
            tags: launcher.tags.clone(),
            metadata: launcher.metadata.clone(),
        }
    }
}

/// Errors surfaced while starting or stopping MCP runtimes.
#[derive(Debug, Error)]
pub enum McpRuntimeError {
    #[error("runtime `{0}` not found")]
    NotFound(String),
    #[error("runtime `{name}` uses `{actual}` transport (expected {expected})")]
    UnsupportedTransport {
        name: String,
        expected: &'static str,
        actual: &'static str,
    },
    #[error("failed to spawn `{command:?}`: {source}")]
    Spawn {
        command: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("stdio pipes unavailable for `{name}`")]
    MissingPipes { name: String },
    #[error("failed to stop `{name}`: {source}")]
    Stop {
        name: String,
        #[source]
        source: io::Error,
    },
    #[error("timed out stopping `{name}` after {timeout:?}")]
    StopTimeout { name: String, timeout: Duration },
}

/// Lightweight runtime manager that owns resolved launchers/connectors.
///
/// The manager is non-destructive: launchers remain available after `prepare`
/// is called so callers can reuse connectors or restart stdio servers as
/// needed.
#[derive(Clone, Debug)]
pub struct McpRuntimeManager {
    launchers: BTreeMap<String, McpServerLauncher>,
}

impl McpRuntimeManager {
    /// Construct a runtime manager from resolved launchers.
    pub fn new(launchers: Vec<McpServerLauncher>) -> Self {
        let mut map = BTreeMap::new();
        for launcher in launchers {
            map.insert(launcher.name.clone(), launcher);
        }
        Self { launchers: map }
    }

    /// Returns the available runtimes with tool hints intact.
    pub fn available(&self) -> Vec<McpRuntimeSummary> {
        self.launchers
            .values()
            .map(McpRuntimeSummary::from)
            .collect()
    }

    /// Returns a cloned launcher/connector by name without mutating storage.
    pub fn launcher(&self, name: &str) -> Option<McpServerLauncher> {
        self.launchers.get(name).cloned()
    }

    /// Start a stdio runtime or hand back HTTP connector metadata.
    pub fn prepare(&self, name: &str) -> Result<McpRuntimeHandle, McpRuntimeError> {
        let Some(launcher) = self.launcher(name) else {
            return Err(McpRuntimeError::NotFound(name.to_string()));
        };

        let tools = launcher.tools.clone();
        match launcher.transport {
            McpServerLauncherTransport::Stdio(launch) => {
                let mut command = launch.command();
                let spawn_target = launch.command.clone();
                let mut child = command.spawn().map_err(|source| McpRuntimeError::Spawn {
                    command: spawn_target,
                    source,
                })?;

                let stdout = child.stdout.take();
                let stdin = child.stdin.take();
                if let (Some(stdout), Some(stdin)) = (stdout, stdin) {
                    let stderr = child.stderr.take();
                    Ok(McpRuntimeHandle::Stdio(ManagedStdioRuntime {
                        name: launcher.name,
                        tools,
                        child,
                        stdin,
                        stdout,
                        stderr,
                        timeout: launch.timeout,
                    }))
                } else {
                    let _ = child.start_kill();
                    Err(McpRuntimeError::MissingPipes {
                        name: launcher.name,
                    })
                }
            }
            McpServerLauncherTransport::StreamableHttp(connector) => {
                Ok(McpRuntimeHandle::StreamableHttp(ManagedHttpRuntime {
                    name: launcher.name,
                    connector,
                    tools,
                }))
            }
        }
    }
}

/// Read-only helpers around [`McpRuntimeManager`] backed by stored config.
#[derive(Clone, Debug)]
pub struct McpRuntimeApi {
    manager: McpRuntimeManager,
}

impl McpRuntimeApi {
    /// Build a runtime API from already prepared launchers/connectors.
    pub fn new(launchers: Vec<McpServerLauncher>) -> Self {
        Self {
            manager: McpRuntimeManager::new(launchers),
        }
    }

    /// Load runtime launchers from disk and merge Workstream A stdio defaults.
    ///
    /// This is non-destructive: stored definitions are read, resolved, and left untouched.
    pub fn from_config(
        config: &McpConfigManager,
        defaults: &StdioServerConfig,
    ) -> Result<Self, McpConfigError> {
        let launchers = config.runtime_launchers(defaults)?;
        Ok(Self::new(launchers))
    }

    /// List available runtimes along with tool hints.
    pub fn available(&self) -> Vec<McpRuntimeSummary> {
        self.manager.available()
    }

    /// Returns a launch-ready config for the given runtime.
    pub fn launcher(&self, name: &str) -> Result<McpServerLauncher, McpRuntimeError> {
        self.manager
            .launcher(name)
            .ok_or_else(|| McpRuntimeError::NotFound(name.to_string()))
    }

    /// Returns the stdio launcher for a runtime, erroring if it uses HTTP.
    pub fn stdio_launcher(&self, name: &str) -> Result<StdioLauncher, McpRuntimeError> {
        let launcher = self.launcher(name)?;
        match launcher.transport {
            McpServerLauncherTransport::Stdio(launch) => Ok(launch),
            McpServerLauncherTransport::StreamableHttp(_) => {
                Err(McpRuntimeError::UnsupportedTransport {
                    name: launcher.name,
                    expected: "stdio",
                    actual: "streamable_http",
                })
            }
        }
    }

    /// Returns the HTTP connector for a runtime, erroring if it uses stdio.
    pub fn http_connector(&self, name: &str) -> Result<StreamableHttpConnector, McpRuntimeError> {
        let launcher = self.launcher(name)?;
        match launcher.transport {
            McpServerLauncherTransport::StreamableHttp(connector) => Ok(connector),
            McpServerLauncherTransport::Stdio(_) => Err(McpRuntimeError::UnsupportedTransport {
                name: launcher.name,
                expected: "streamable_http",
                actual: "stdio",
            }),
        }
    }

    /// Prepare a runtime handle for connection or spawn.
    pub fn prepare(&self, name: &str) -> Result<McpRuntimeHandle, McpRuntimeError> {
        self.manager.prepare(name)
    }
}

/// Handle returned by [`McpRuntimeManager::prepare`] for either transport.
#[derive(Debug)]
pub enum McpRuntimeHandle {
    Stdio(ManagedStdioRuntime),
    StreamableHttp(ManagedHttpRuntime),
}

impl McpRuntimeHandle {
    /// Returns tool hints when present.
    pub fn tools(&self) -> Option<&McpToolConfig> {
        match self {
            McpRuntimeHandle::Stdio(handle) => handle.tools.as_ref(),
            McpRuntimeHandle::StreamableHttp(handle) => handle.tools.as_ref(),
        }
    }
}

/// Running stdio MCP server along with its pipes.
#[derive(Debug)]
pub struct ManagedStdioRuntime {
    name: String,
    tools: Option<McpToolConfig>,
    child: Child,
    stdin: ChildStdin,
    stdout: ChildStdout,
    stderr: Option<ChildStderr>,
    timeout: Duration,
}

impl ManagedStdioRuntime {
    /// Name of the runtime.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Tool allow/deny hints if provided.
    pub fn tools(&self) -> Option<&McpToolConfig> {
        self.tools.as_ref()
    }

    /// Writable pipe to the server.
    pub fn stdin_mut(&mut self) -> &mut ChildStdin {
        &mut self.stdin
    }

    /// Readable pipe from the server.
    pub fn stdout_mut(&mut self) -> &mut ChildStdout {
        &mut self.stdout
    }

    /// Optional stderr pipe from the server.
    pub fn stderr_mut(&mut self) -> Option<&mut ChildStderr> {
        self.stderr.as_mut()
    }

    /// Terminate the process and wait for exit (best-effort).
    pub async fn stop(&mut self) -> Result<(), McpRuntimeError> {
        if let Ok(Some(_)) = self.child.try_wait() {
            return Ok(());
        }

        let _ = self.child.start_kill();
        match time::timeout(self.timeout, self.child.wait()).await {
            Ok(Ok(_)) => Ok(()),
            Ok(Err(source)) => Err(McpRuntimeError::Stop {
                name: self.name.clone(),
                source,
            }),
            Err(_) => Err(McpRuntimeError::StopTimeout {
                name: self.name.clone(),
                timeout: self.timeout,
            }),
        }
    }
}

impl Drop for ManagedStdioRuntime {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
    }
}

/// HTTP runtime connector with tool hints preserved.
#[derive(Clone, Debug)]
pub struct ManagedHttpRuntime {
    pub name: String,
    pub connector: StreamableHttpConnector,
    pub tools: Option<McpToolConfig>,
}

/// Errors surfaced while reading app runtimes.
#[derive(Debug, Error)]
pub enum AppRuntimeError {
    #[error("runtime `{0}` not found")]
    NotFound(String),
    #[error("failed to start runtime `{name}`: {source}")]
    Start {
        name: String,
        #[source]
        source: McpError,
    },
    #[error("failed to stop runtime `{name}`: {source}")]
    Stop {
        name: String,
        #[source]
        source: McpError,
    },
}

/// Prepared app runtime with merged stdio config and metadata.
#[derive(Clone, Debug)]
pub struct AppRuntimeHandle {
    pub name: String,
    pub metadata: Value,
    pub config: StdioServerConfig,
}

/// Running app-server instance with metadata preserved.
pub struct ManagedAppRuntime {
    pub name: String,
    pub metadata: Value,
    pub config: StdioServerConfig,
    pub server: CodexAppServer,
}

impl fmt::Debug for ManagedAppRuntime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ManagedAppRuntime")
            .field("name", &self.name)
            .field("metadata", &self.metadata)
            .field("config", &self.config)
            .finish()
    }
}

impl ManagedAppRuntime {
    /// Gracefully shut down the app-server.
    pub async fn stop(&self) -> Result<(), McpError> {
        self.server.shutdown().await
    }
}

impl AppRuntimeHandle {
    /// Launch the app-server using the prepared stdio config.
    pub async fn start(self, client: ClientInfo) -> Result<ManagedAppRuntime, McpError> {
        let AppRuntimeHandle {
            name,
            metadata,
            config,
        } = self;

        let server = CodexAppServer::start(config.clone(), client).await?;

        Ok(ManagedAppRuntime {
            name,
            metadata,
            config,
            server,
        })
    }
}

/// Non-destructive manager for app runtimes backed by launch-ready configs.
#[derive(Clone, Debug)]
pub struct AppRuntimeManager {
    launchers: BTreeMap<String, AppRuntimeLauncher>,
}

impl AppRuntimeManager {
    /// Construct a runtime manager from prepared launchers.
    pub fn new(launchers: Vec<AppRuntimeLauncher>) -> Self {
        let mut map = BTreeMap::new();
        for launcher in launchers {
            map.insert(launcher.name.clone(), launcher);
        }
        Self { launchers: map }
    }

    /// Returns the available app runtimes with metadata intact.
    pub fn available(&self) -> Vec<AppRuntimeSummary> {
        self.launchers
            .values()
            .map(AppRuntimeSummary::from)
            .collect()
    }

    /// Returns a cloned launcher by name without mutating storage.
    pub fn launcher(&self, name: &str) -> Option<AppRuntimeLauncher> {
        self.launchers.get(name).cloned()
    }

    /// Returns a prepared config + metadata for launching the app server.
    pub fn prepare(&self, name: &str) -> Result<AppRuntimeHandle, AppRuntimeError> {
        let Some(launcher) = self.launcher(name) else {
            return Err(AppRuntimeError::NotFound(name.to_string()));
        };

        Ok(AppRuntimeHandle {
            name: launcher.name,
            metadata: launcher.metadata,
            config: launcher.config,
        })
    }

    /// Start an app-server runtime using the prepared config and metadata.
    pub async fn start(
        &self,
        name: &str,
        client: ClientInfo,
    ) -> Result<ManagedAppRuntime, AppRuntimeError> {
        let handle = self.prepare(name)?;
        handle
            .start(client)
            .await
            .map_err(|source| AppRuntimeError::Start {
                name: name.to_string(),
                source,
            })
    }
}

/// Read-only helpers around [`AppRuntimeManager`] backed by stored config.
#[derive(Clone, Debug)]
pub struct AppRuntimeApi {
    manager: AppRuntimeManager,
}

impl AppRuntimeApi {
    /// Build an API from already prepared launchers.
    pub fn new(launchers: Vec<AppRuntimeLauncher>) -> Self {
        Self {
            manager: AppRuntimeManager::new(launchers),
        }
    }

    /// Load app runtimes from disk and merge Workstream A stdio defaults.
    pub fn from_config(
        config: &McpConfigManager,
        defaults: &StdioServerConfig,
    ) -> Result<Self, McpConfigError> {
        let launchers = config.app_runtime_launchers(defaults)?;
        Ok(Self::new(launchers))
    }

    /// List available runtimes and metadata.
    pub fn available(&self) -> Vec<AppRuntimeSummary> {
        self.manager.available()
    }

    /// Returns the launch-ready config bundle for the given runtime.
    pub fn launcher(&self, name: &str) -> Result<AppRuntimeLauncher, AppRuntimeError> {
        self.manager
            .launcher(name)
            .ok_or_else(|| AppRuntimeError::NotFound(name.to_string()))
    }

    /// Prepare a stdio config + metadata for a runtime.
    pub fn prepare(&self, name: &str) -> Result<AppRuntimeHandle, AppRuntimeError> {
        self.manager.prepare(name)
    }

    /// Start an app runtime and return a managed handle.
    pub async fn start(
        &self,
        name: &str,
        client: ClientInfo,
    ) -> Result<ManagedAppRuntime, AppRuntimeError> {
        self.manager.start(name, client).await
    }

    /// Convenience accessor for the merged stdio config.
    pub fn stdio_config(&self, name: &str) -> Result<StdioServerConfig, AppRuntimeError> {
        self.prepare(name).map(|handle| handle.config)
    }

    /// Build a pooled lifecycle manager that can reuse running runtimes.
    pub fn pool(&self) -> AppRuntimePool {
        AppRuntimePool::new(self.manager.clone())
    }

    /// Consume the API and return a pooled lifecycle manager.
    pub fn into_pool(self) -> AppRuntimePool {
        AppRuntimePool::new(self.manager)
    }

    /// Build a pooled lifecycle API that can reuse running runtimes.
    pub fn pool_api(&self) -> AppRuntimePoolApi {
        AppRuntimePoolApi::from_manager(self.manager.clone())
    }

    /// Consume the API and return a pooled lifecycle API.
    pub fn into_pool_api(self) -> AppRuntimePoolApi {
        AppRuntimePoolApi::from_manager(self.manager)
    }
}

/// Async pool that starts, reuses, and stops app runtimes without mutating config.
///
/// Runtime metadata and resume hints remain intact when runtimes are reused or restarted.
#[derive(Clone, Debug)]
pub struct AppRuntimePool {
    manager: AppRuntimeManager,
    running: Arc<Mutex<HashMap<String, Arc<ManagedAppRuntime>>>>,
}

impl AppRuntimePool {
    /// Create a new pool backed by launch-ready runtime configs.
    pub fn new(manager: AppRuntimeManager) -> Self {
        Self {
            manager,
            running: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// List available runtimes and metadata without touching stored definitions.
    pub fn available(&self) -> Vec<AppRuntimeSummary> {
        self.manager.available()
    }

    /// List currently running runtimes with metadata/resume hints preserved.
    pub async fn running(&self) -> Vec<AppRuntimeSummary> {
        let mut names: Vec<String> = {
            let guard = self.running.lock().await;
            guard.keys().cloned().collect()
        };

        names.sort();

        names
            .into_iter()
            .filter_map(|name| self.manager.launcher(&name))
            .map(|launcher| AppRuntimeSummary::from(&launcher))
            .collect()
    }

    /// Returns a launch-ready config bundle for the given runtime.
    pub fn launcher(&self, name: &str) -> Option<AppRuntimeLauncher> {
        self.manager.launcher(name)
    }

    /// Prepare a stdio config + metadata for a runtime without starting it.
    pub fn prepare(&self, name: &str) -> Result<AppRuntimeHandle, AppRuntimeError> {
        self.manager.prepare(name)
    }

    /// Start (or reuse) an app runtime. Subsequent calls reuse an existing instance.
    pub async fn start(
        &self,
        name: &str,
        client: ClientInfo,
    ) -> Result<Arc<ManagedAppRuntime>, AppRuntimeError> {
        {
            let guard = self.running.lock().await;
            if let Some(existing) = guard.get(name) {
                return Ok(existing.clone());
            }
        }

        let runtime = Arc::new(self.manager.start(name, client).await?);

        let mut guard = self.running.lock().await;
        if let Some(existing) = guard.get(name) {
            runtime
                .stop()
                .await
                .map_err(|source| AppRuntimeError::Stop {
                    name: name.to_string(),
                    source,
                })?;
            return Ok(existing.clone());
        }

        guard.insert(name.to_string(), runtime.clone());
        Ok(runtime)
    }

    /// Stop a running runtime and remove it from the pool.
    pub async fn stop(&self, name: &str) -> Result<(), AppRuntimeError> {
        let runtime = {
            let mut guard = self.running.lock().await;
            guard.remove(name)
        };

        match runtime {
            Some(runtime) => runtime
                .stop()
                .await
                .map_err(|source| AppRuntimeError::Stop {
                    name: name.to_string(),
                    source,
                }),
            None => Err(AppRuntimeError::NotFound(name.to_string())),
        }
    }

    /// Stop all running runtimes (best-effort) and clear the pool.
    pub async fn stop_all(&self) -> Result<(), AppRuntimeError> {
        let runtimes: Vec<(String, Arc<ManagedAppRuntime>)> = {
            let mut guard = self.running.lock().await;
            guard.drain().collect()
        };

        let mut first_error: Option<AppRuntimeError> = None;

        for (name, runtime) in runtimes {
            if let Err(source) = runtime.stop().await {
                if first_error.is_none() {
                    first_error = Some(AppRuntimeError::Stop { name, source });
                }
            }
        }

        if let Some(err) = first_error {
            return Err(err);
        }

        Ok(())
    }
}

/// Public API around [`AppRuntimePool`] that exposes pooled lifecycle helpers.
///
/// Operations reuse running runtimes when available while preserving stored definitions
/// and app metadata/resume hints.
#[derive(Clone, Debug)]
pub struct AppRuntimePoolApi {
    pool: AppRuntimePool,
}

impl AppRuntimePoolApi {
    /// Build a pooled API from prepared launchers.
    pub fn new(launchers: Vec<AppRuntimeLauncher>) -> Self {
        Self::from_manager(AppRuntimeManager::new(launchers))
    }

    /// Load app runtimes from disk and merge Workstream A stdio defaults.
    pub fn from_config(
        config: &McpConfigManager,
        defaults: &StdioServerConfig,
    ) -> Result<Self, McpConfigError> {
        let launchers = config.app_runtime_launchers(defaults)?;
        Ok(Self::new(launchers))
    }

    /// Build a pooled API from a runtime manager.
    pub fn from_manager(manager: AppRuntimeManager) -> Self {
        Self::from_pool(AppRuntimePool::new(manager))
    }

    /// Wrap an existing pool in the API surface.
    pub fn from_pool(pool: AppRuntimePool) -> Self {
        Self { pool }
    }

    /// List available runtimes and metadata.
    pub fn available(&self) -> Vec<AppRuntimeSummary> {
        self.pool.available()
    }

    /// List running runtimes with metadata intact.
    pub async fn running(&self) -> Vec<AppRuntimeSummary> {
        self.pool.running().await
    }

    /// Returns the launch-ready config bundle for the given runtime.
    pub fn launcher(&self, name: &str) -> Result<AppRuntimeLauncher, AppRuntimeError> {
        self.pool
            .launcher(name)
            .ok_or_else(|| AppRuntimeError::NotFound(name.to_string()))
    }

    /// Prepare a stdio config + metadata for a runtime.
    pub fn prepare(&self, name: &str) -> Result<AppRuntimeHandle, AppRuntimeError> {
        self.pool.prepare(name)
    }

    /// Start (or reuse) an app runtime.
    pub async fn start(
        &self,
        name: &str,
        client: ClientInfo,
    ) -> Result<Arc<ManagedAppRuntime>, AppRuntimeError> {
        self.pool.start(name, client).await
    }

    /// Stop a running runtime and remove it from the pool.
    pub async fn stop(&self, name: &str) -> Result<(), AppRuntimeError> {
        self.pool.stop(name).await
    }

    /// Stop all running runtimes (best-effort) and clear the pool.
    pub async fn stop_all(&self) -> Result<(), AppRuntimeError> {
        self.pool.stop_all().await
    }

    /// Convenience accessor for the merged stdio config.
    pub fn stdio_config(&self, name: &str) -> Result<StdioServerConfig, AppRuntimeError> {
        self.prepare(name).map(|handle| handle.config)
    }
}

/// Helper to load and mutate MCP + app runtime config stored under `[mcp_servers]` and
/// `[app_runtimes]`.
///
/// Runtime, API, and pool helpers consume this manager in a read-only fashion so stored
/// definitions, auth hints, and metadata are left untouched while preparing launch configs.
pub struct McpConfigManager {
    config_path: PathBuf,
}

impl McpConfigManager {
    /// Create a manager that reads/writes the given config path.
    pub fn new(config_path: impl Into<PathBuf>) -> Self {
        Self {
            config_path: config_path.into(),
        }
    }

    /// Convenience constructor for a CODEX_HOME directory.
    pub fn from_code_home(code_home: impl AsRef<Path>) -> Self {
        Self::new(code_home.as_ref().join(DEFAULT_CONFIG_FILE))
    }

    /// Returns the underlying config path.
    pub fn config_path(&self) -> &Path {
        &self.config_path
    }

    /// Returns all configured MCP servers.
    pub fn list_servers(&self) -> Result<Vec<McpServerEntry>, McpConfigError> {
        let servers = self.read_servers()?;
        Ok(servers
            .into_iter()
            .map(|(name, definition)| McpServerEntry { name, definition })
            .collect())
    }

    /// Returns a single MCP server by name.
    pub fn get_server(&self, name: &str) -> Result<McpServerEntry, McpConfigError> {
        let servers = self.read_servers()?;
        let Some(definition) = servers.get(name).cloned() else {
            return Err(McpConfigError::ServerNotFound(name.to_string()));
        };

        Ok(McpServerEntry {
            name: name.to_string(),
            definition,
        })
    }

    /// Returns all configured app runtimes.
    pub fn list_app_runtimes(&self) -> Result<Vec<AppRuntimeEntry>, McpConfigError> {
        let runtimes = self.read_app_runtimes()?;
        Ok(runtimes
            .into_iter()
            .map(|(name, definition)| AppRuntimeEntry { name, definition })
            .collect())
    }

    /// Returns a single app runtime by name.
    pub fn get_app_runtime(&self, name: &str) -> Result<AppRuntimeEntry, McpConfigError> {
        let runtimes = self.read_app_runtimes()?;
        let Some(definition) = runtimes.get(name).cloned() else {
            return Err(McpConfigError::AppRuntimeNotFound(name.to_string()));
        };

        Ok(AppRuntimeEntry {
            name: name.to_string(),
            definition,
        })
    }

    /// Returns runtime-ready app configs with metadata preserved.
    pub fn app_runtimes(&self) -> Result<Vec<AppRuntime>, McpConfigError> {
        Ok(self
            .list_app_runtimes()?
            .into_iter()
            .map(AppRuntime::from)
            .collect())
    }

    /// Returns a runtime-ready app config for a single entry.
    pub fn app_runtime(&self, name: &str) -> Result<AppRuntime, McpConfigError> {
        self.get_app_runtime(name).map(AppRuntime::from)
    }

    /// Returns prepared launchers for all app runtimes.
    pub fn app_runtime_launchers(
        &self,
        defaults: &StdioServerConfig,
    ) -> Result<Vec<AppRuntimeLauncher>, McpConfigError> {
        self.app_runtimes().map(|runtimes| {
            runtimes
                .into_iter()
                .map(|runtime| runtime.into_launcher(defaults))
                .collect()
        })
    }

    /// Returns a prepared launcher for an app runtime by name.
    pub fn app_runtime_launcher(
        &self,
        name: &str,
        defaults: &StdioServerConfig,
    ) -> Result<AppRuntimeLauncher, McpConfigError> {
        self.app_runtime(name)
            .map(|runtime| runtime.into_launcher(defaults))
    }

    /// Returns runtime-ready configs for all servers, resolving bearer tokens from the environment.
    pub fn runtime_servers(&self) -> Result<Vec<McpRuntimeServer>, McpConfigError> {
        Ok(self
            .list_servers()?
            .into_iter()
            .map(McpRuntimeServer::from)
            .collect())
    }

    /// Returns a runtime-ready config for a single server by name.
    pub fn runtime_server(&self, name: &str) -> Result<McpRuntimeServer, McpConfigError> {
        self.get_server(name).map(McpRuntimeServer::from)
    }

    /// Returns prepared launchers/connectors for all runtime servers.
    pub fn runtime_launchers(
        &self,
        defaults: &StdioServerConfig,
    ) -> Result<Vec<McpServerLauncher>, McpConfigError> {
        self.runtime_servers().map(|servers| {
            servers
                .into_iter()
                .map(|server| server.into_launcher(defaults))
                .collect()
        })
    }

    /// Returns a prepared launcher/connector for a single runtime server by name.
    pub fn runtime_launcher(
        &self,
        name: &str,
        defaults: &StdioServerConfig,
    ) -> Result<McpServerLauncher, McpConfigError> {
        self.runtime_server(name)
            .map(|server| server.into_launcher(defaults))
    }

    /// Adds or updates an app runtime definition.
    pub fn add_app_runtime(
        &self,
        request: AddAppRuntimeRequest,
    ) -> Result<AppRuntimeEntry, McpConfigError> {
        let AddAppRuntimeRequest {
            name,
            definition,
            overwrite,
        } = request;

        if name.trim().is_empty() {
            return Err(McpConfigError::InvalidAppRuntimeName);
        }

        let (table, mut runtimes) = self.read_table_and_app_runtimes()?;
        if !overwrite && runtimes.contains_key(&name) {
            return Err(McpConfigError::AppRuntimeAlreadyExists(name));
        }

        runtimes.insert(name.clone(), definition.clone());
        self.persist_app_runtimes(table, &runtimes)?;

        Ok(AppRuntimeEntry { name, definition })
    }

    /// Adds or updates a server definition and injects any provided env vars.
    pub fn add_server(
        &self,
        mut request: AddServerRequest,
    ) -> Result<McpServerEntry, McpConfigError> {
        if request.name.trim().is_empty() {
            return Err(McpConfigError::InvalidServerName);
        }

        let mut env_injections = request.env.clone();
        if let Some(token) = request.bearer_token.take() {
            let var = Self::bearer_env_var(&request.name, &request.definition)?;
            env_injections.entry(var).or_insert(token);
        }

        if let McpTransport::Stdio(transport) = &mut request.definition.transport {
            for (key, value) in &env_injections {
                transport.env.entry(key.clone()).or_insert(value.clone());
            }
        }

        self.set_env_vars(&env_injections)?;

        let (table, mut servers) = self.read_table_and_servers()?;
        if !request.overwrite && servers.contains_key(&request.name) {
            return Err(McpConfigError::ServerAlreadyExists(request.name));
        }

        servers.insert(request.name.clone(), request.definition.clone());
        self.persist_servers(table, &servers)?;

        Ok(McpServerEntry {
            name: request.name,
            definition: request.definition,
        })
    }

    /// Removes a server definition. Returns the removed entry if it existed.
    pub fn remove_server(&self, name: &str) -> Result<Option<McpServerEntry>, McpConfigError> {
        let (table, mut servers) = self.read_table_and_servers()?;
        let removed = servers.remove(name).map(|definition| McpServerEntry {
            name: name.to_string(),
            definition,
        });

        if removed.is_some() {
            self.persist_servers(table, &servers)?;
        }

        Ok(removed)
    }

    /// Writes the provided token into the server's bearer env var.
    pub fn login(
        &self,
        name: &str,
        token: impl AsRef<str>,
    ) -> Result<McpLoginResult, McpConfigError> {
        let servers = self.read_servers()?;
        let definition = servers
            .get(name)
            .ok_or_else(|| McpConfigError::ServerNotFound(name.to_string()))?;
        let env_var = Self::bearer_env_var(name, definition)?;
        self.validate_env_key(&env_var)?;
        env::set_var(&env_var, token.as_ref());
        Ok(McpLoginResult {
            server: name.to_string(),
            env_var: Some(env_var),
        })
    }

    /// Clears the bearer env var used for the given server.
    pub fn logout(&self, name: &str) -> Result<McpLogoutResult, McpConfigError> {
        let servers = self.read_servers()?;
        let definition = servers
            .get(name)
            .ok_or_else(|| McpConfigError::ServerNotFound(name.to_string()))?;
        let env_var = Self::bearer_env_var(name, definition)?;
        let cleared = env::var(&env_var).is_ok();
        env::remove_var(&env_var);
        Ok(McpLogoutResult {
            server: name.to_string(),
            env_var: Some(env_var),
            cleared,
        })
    }

    fn bearer_env_var(
        name: &str,
        definition: &McpServerDefinition,
    ) -> Result<String, McpConfigError> {
        match &definition.transport {
            McpTransport::StreamableHttp(http) => {
                http.bearer_env_var
                    .clone()
                    .ok_or_else(|| McpConfigError::MissingBearerEnvVar {
                        server: name.to_string(),
                    })
            }
            McpTransport::Stdio(_) => Err(McpConfigError::UnsupportedAuthTransport {
                server: name.to_string(),
            }),
        }
    }

    fn read_servers(&self) -> Result<BTreeMap<String, McpServerDefinition>, McpConfigError> {
        let table = self.load_table()?;
        self.parse_servers(table.get(MCP_SERVERS_KEY))
    }

    fn read_table_and_servers(
        &self,
    ) -> Result<(TomlTable, BTreeMap<String, McpServerDefinition>), McpConfigError> {
        let table = self.load_table()?;
        let servers = self.parse_servers(table.get(MCP_SERVERS_KEY))?;
        Ok((table, servers))
    }

    fn parse_servers(
        &self,
        value: Option<&TomlValue>,
    ) -> Result<BTreeMap<String, McpServerDefinition>, McpConfigError> {
        let Some(value) = value else {
            return Ok(BTreeMap::new());
        };

        let table = value
            .as_table()
            .ok_or_else(|| McpConfigError::InvalidServers {
                path: self.config_path.clone(),
            })?;
        let cloned = TomlValue::Table(table.clone());
        cloned
            .try_into()
            .map_err(|source| McpConfigError::DecodeServers { source })
    }

    fn persist_servers(
        &self,
        mut table: TomlTable,
        servers: &BTreeMap<String, McpServerDefinition>,
    ) -> Result<(), McpConfigError> {
        if servers.is_empty() {
            table.remove(MCP_SERVERS_KEY);
        } else {
            let value = TomlValue::try_from(servers.clone())
                .map_err(|source| McpConfigError::Serialize { source })?;
            table.insert(MCP_SERVERS_KEY.to_string(), value);
        }

        self.write_table(table)
    }

    fn read_app_runtimes(&self) -> Result<BTreeMap<String, AppRuntimeDefinition>, McpConfigError> {
        let table = self.load_table()?;
        self.parse_app_runtimes(table.get(APP_RUNTIMES_KEY))
    }

    fn read_table_and_app_runtimes(
        &self,
    ) -> Result<(TomlTable, BTreeMap<String, AppRuntimeDefinition>), McpConfigError> {
        let table = self.load_table()?;
        let runtimes = self.parse_app_runtimes(table.get(APP_RUNTIMES_KEY))?;
        Ok((table, runtimes))
    }

    fn parse_app_runtimes(
        &self,
        value: Option<&TomlValue>,
    ) -> Result<BTreeMap<String, AppRuntimeDefinition>, McpConfigError> {
        let Some(value) = value else {
            return Ok(BTreeMap::new());
        };

        let table = value
            .as_table()
            .ok_or_else(|| McpConfigError::InvalidAppRuntimes {
                path: self.config_path.clone(),
            })?;
        let cloned = TomlValue::Table(table.clone());
        cloned
            .try_into()
            .map_err(|source| McpConfigError::DecodeAppRuntimes { source })
    }

    fn persist_app_runtimes(
        &self,
        mut table: TomlTable,
        runtimes: &BTreeMap<String, AppRuntimeDefinition>,
    ) -> Result<(), McpConfigError> {
        if runtimes.is_empty() {
            table.remove(APP_RUNTIMES_KEY);
        } else {
            let value = TomlValue::try_from(runtimes.clone())
                .map_err(|source| McpConfigError::Serialize { source })?;
            table.insert(APP_RUNTIMES_KEY.to_string(), value);
        }

        self.write_table(table)
    }

    fn load_table(&self) -> Result<TomlTable, McpConfigError> {
        if !self.config_path.exists() {
            return Ok(TomlTable::new());
        }

        let contents =
            fs::read_to_string(&self.config_path).map_err(|source| McpConfigError::Read {
                path: self.config_path.clone(),
                source,
            })?;

        if contents.trim().is_empty() {
            return Ok(TomlTable::new());
        }

        let value: TomlValue = contents.parse().map_err(|source| McpConfigError::Parse {
            path: self.config_path.clone(),
            source,
        })?;

        value
            .as_table()
            .cloned()
            .ok_or_else(|| McpConfigError::InvalidRoot {
                path: self.config_path.clone(),
            })
    }

    fn write_table(&self, table: TomlTable) -> Result<(), McpConfigError> {
        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent).map_err(|source| McpConfigError::CreateDir {
                path: parent.to_path_buf(),
                source,
            })?;
        }

        let serialized = toml::to_string_pretty(&TomlValue::Table(table))
            .map_err(|source| McpConfigError::Serialize { source })?;

        fs::write(&self.config_path, serialized).map_err(|source| McpConfigError::Write {
            path: self.config_path.clone(),
            source,
        })
    }

    fn set_env_vars(&self, vars: &BTreeMap<String, String>) -> Result<(), McpConfigError> {
        for (key, value) in vars {
            self.validate_env_key(key)?;
            env::set_var(key, value);
        }
        Ok(())
    }

    fn validate_env_key(&self, key: &str) -> Result<(), McpConfigError> {
        let invalid = key.is_empty() || key.contains('=') || key.contains('\0');
        if invalid {
            return Err(McpConfigError::InvalidEnvVarName {
                name: key.to_string(),
            });
        }
        Ok(())
    }
}

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
    #[serde(rename = "clientInfo")]
    pub client: ClientInfo,
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    #[serde(default)]
    pub capabilities: Value,
}

/// Parameters for `codex/codex` (new session).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CodexCallParams {
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sandbox: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approval_policy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub config: BTreeMap<String, Value>,
}

/// Parameters for `codex/codex-reply` (continue an existing conversation).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CodexReplyParams {
    #[serde(rename = "conversationId")]
    pub conversation_id: String,
    pub prompt: String,
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
    #[serde(default, rename = "conversationId", alias = "conversation_id")]
    pub conversation_id: Option<String>,
    #[serde(default, rename = "content", alias = "output")]
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
#[serde(rename_all = "camelCase")]
pub struct ThreadResumeParams {
    pub thread_id: String,
}

/// Parameters for `turn/start`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnStartParams {
    pub thread_id: String,
    #[serde(rename = "input", alias = "prompt")]
    pub input: Vec<TurnInput>,
    pub model: Option<String>,
    #[serde(default)]
    pub config: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TurnInput {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// Parameters for `turn/interrupt`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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

#[derive(Clone)]
enum NotificationHook {
    Codex {
        sinks: Arc<Mutex<Vec<mpsc::UnboundedSender<CodexEvent>>>>,
    },
    App {
        sinks: Arc<Mutex<Vec<mpsc::UnboundedSender<AppNotification>>>>,
    },
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
    pub async fn thread_start(&self, params: ThreadStartParams) -> Result<AppCallHandle, McpError> {
        self.invoke_app_call(METHOD_THREAD_START, serde_json::to_value(params)?)
            .await
    }

    /// Resume an existing thread via `thread/resume`.
    pub async fn thread_resume(
        &self,
        params: ThreadResumeParams,
    ) -> Result<AppCallHandle, McpError> {
        self.invoke_app_call(METHOD_THREAD_RESUME, serde_json::to_value(params)?)
            .await
    }

    /// Start a new turn on a thread via `turn/start`.
    pub async fn turn_start(&self, params: TurnStartParams) -> Result<AppCallHandle, McpError> {
        self.invoke_app_call(METHOD_TURN_START, serde_json::to_value(params)?)
            .await
    }

    /// Interrupt an active turn via `turn/interrupt`.
    pub async fn turn_interrupt(
        &self,
        params: TurnInterruptParams,
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

type PendingRequests = Arc<Mutex<HashMap<RequestId, oneshot::Sender<Result<Value, McpError>>>>>;

/// Internal transport that handles stdio JSON-RPC.
struct JsonRpcTransport {
    writer: mpsc::UnboundedSender<String>,
    pending: PendingRequests,
    notification_hook: NotificationHook,
    next_id: AtomicU64,
    tasks: Vec<JoinHandle<()>>,
    child: Arc<Mutex<Option<Child>>>,
    startup_timeout: Duration,
}

impl JsonRpcTransport {
    async fn spawn_mcp(config: StdioServerConfig) -> Result<Self, McpError> {
        let hook = NotificationHook::Codex {
            sinks: Arc::new(Mutex::new(Vec::new())),
        };
        Self::spawn_with_subcommand(config, "mcp-server", hook).await
    }

    async fn spawn_app(config: StdioServerConfig) -> Result<Self, McpError> {
        let hook = NotificationHook::App {
            sinks: Arc::new(Mutex::new(Vec::new())),
        };
        Self::spawn_with_subcommand(config, "app-server", hook).await
    }

    async fn spawn_with_subcommand(
        config: StdioServerConfig,
        subcommand: &str,
        notification_hook: NotificationHook,
    ) -> Result<Self, McpError> {
        let mut command = Command::new(&config.binary);
        command
            .arg(subcommand)
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

        let command_debug = format!("{command:?}");
        let mut backoff = Duration::from_millis(2);
        let mut child = {
            let mut child = None;
            for attempt in 0..5 {
                match command.spawn() {
                    Ok(spawned) => {
                        child = Some(spawned);
                        break;
                    }
                    Err(source) => {
                        let is_busy =
                            matches!(source.kind(), std::io::ErrorKind::ExecutableFileBusy)
                                || source.raw_os_error() == Some(26);
                        if is_busy && attempt < 4 {
                            tokio::time::sleep(backoff).await;
                            backoff = std::cmp::min(backoff * 2, Duration::from_millis(50));
                            continue;
                        }
                        return Err(McpError::Spawn {
                            command: command_debug,
                            source,
                        });
                    }
                }
            }
            child.expect("spawn loop should return or set child")
        };

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
        let (writer_tx, writer_rx) = mpsc::unbounded_channel();

        let writer_handle = tokio::spawn(writer_task(stdin, writer_rx));
        let reader_handle = tokio::spawn(reader_task(
            stdout,
            pending.clone(),
            notification_hook.clone(),
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
            notification_hook,
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
        match &self.notification_hook {
            NotificationHook::Codex { sinks } => {
                let (tx, rx) = mpsc::unbounded_channel();
                let mut guard = sinks.lock().await;
                guard.push(tx);
                rx
            }
            _ => {
                let (_tx, rx) = mpsc::unbounded_channel();
                rx
            }
        }
    }

    async fn register_app_listener(&self) -> EventStream<AppNotification> {
        match &self.notification_hook {
            NotificationHook::App { sinks } => {
                let (tx, rx) = mpsc::unbounded_channel();
                let mut guard = sinks.lock().await;
                guard.push(tx);
                rx
            }
            _ => {
                let (_tx, rx) = mpsc::unbounded_channel();
                rx
            }
        }
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
    pending: PendingRequests,
    notification_hook: NotificationHook,
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
            Some(Incoming::Notification(notification)) => match &notification_hook {
                NotificationHook::Codex { sinks } => {
                    if notification.method == METHOD_CODEX_EVENT {
                        let params = notification.params.unwrap_or(Value::Null);
                        let event = parse_codex_event(&params).unwrap_or(CodexEvent::Raw {
                            method: METHOD_CODEX_EVENT.to_string(),
                            params,
                        });
                        broadcast_codex_event(event, sinks).await;
                    }
                }
                NotificationHook::App { sinks } => {
                    let params = notification.params.unwrap_or(Value::Null);
                    let event = parse_app_notification(&notification.method, &params);
                    broadcast_app_event(event, sinks).await;
                }
            },
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

async fn handle_response(response: RpcResponse, pending: &PendingRequests) {
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
    let payload = value.get("msg").unwrap_or(value);
    let event_type = payload.get("type")?.as_str()?;
    let conversation_id = payload
        .get("thread_id")
        .or_else(|| payload.get("threadId"))
        .or_else(|| payload.get("session_id"))
        .or_else(|| payload.get("conversation_id"))
        .or_else(|| payload.get("conversationId"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();

    match event_type {
        "task_complete" => Some(CodexEvent::TaskComplete {
            conversation_id,
            result: payload
                .get("result")
                .cloned()
                .unwrap_or_else(|| payload.clone()),
        }),
        "approval_required" | "approval" => {
            let approval_id = payload
                .get("approval_id")
                .or_else(|| payload.get("id"))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let raw_kind = payload
                .get("kind")
                .or_else(|| payload.get("approval_kind"))
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
                payload: payload.clone(),
            }))
        }
        "cancelled" | "canceled" => Some(CodexEvent::Cancelled {
            conversation_id: payload
                .get("conversation_id")
                .and_then(Value::as_str)
                .map(|s| s.to_string()),
            reason: payload
                .get("reason")
                .and_then(Value::as_str)
                .map(|s| s.to_string()),
        }),
        "error" => Some(CodexEvent::Error {
            message: payload
                .get("message")
                .or_else(|| payload.get("error"))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            data: payload.get("data").cloned(),
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

fn parse_app_notification(method: &str, value: &Value) -> AppNotification {
    let notification_type = value
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_lowercase();

    let thread_id = extract_string(value, &["thread_id", "threadId"]).unwrap_or_default();
    let turn_id = extract_string(value, &["turn_id", "turnId"]);

    match notification_type.as_str() {
        "task_complete" | "taskcomplete" => AppNotification::TaskComplete {
            thread_id,
            turn_id,
            result: value.get("result").cloned().unwrap_or(Value::Null),
        },
        "item" => AppNotification::Item {
            thread_id,
            turn_id,
            item: value.get("item").cloned().unwrap_or_else(|| value.clone()),
        },
        "error" => AppNotification::Error {
            message: value
                .get("message")
                .or_else(|| value.get("error"))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            data: value.get("data").cloned(),
        },
        _ => AppNotification::Raw {
            method: method.to_string(),
            params: value.clone(),
        },
    }
}

fn extract_string(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| value.get(key).and_then(Value::as_str))
        .map(|s| s.to_string())
}

async fn broadcast_app_event(
    event: AppNotification,
    sinks: &Arc<Mutex<Vec<mpsc::UnboundedSender<AppNotification>>>>,
) {
    let mut guard = sinks.lock().await;
    guard.retain(|tx| tx.send(event.clone()).is_ok());
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        collections::{BTreeMap, HashMap},
        env,
        ffi::OsString,
        fs,
        os::unix::fs::PermissionsExt,
        path::PathBuf,
    };

    fn temp_config_manager() -> (tempfile::TempDir, McpConfigManager) {
        let dir = tempfile::tempdir().expect("tempdir");
        let manager = McpConfigManager::from_code_home(dir.path());
        (dir, manager)
    }

    fn stdio_definition(command: &str) -> McpServerDefinition {
        McpServerDefinition {
            transport: McpTransport::Stdio(StdioServerDefinition {
                command: command.to_string(),
                args: Vec::new(),
                env: BTreeMap::new(),
                timeout_ms: Some(1500),
            }),
            description: None,
            tags: Vec::new(),
            tools: None,
        }
    }

    fn streamable_definition(url: &str, bearer_var: &str) -> McpServerDefinition {
        McpServerDefinition {
            transport: McpTransport::StreamableHttp(StreamableHttpDefinition {
                url: url.to_string(),
                headers: BTreeMap::new(),
                bearer_env_var: Some(bearer_var.to_string()),
                connect_timeout_ms: Some(5000),
                request_timeout_ms: Some(5000),
            }),
            description: None,
            tags: Vec::new(),
            tools: Some(McpToolConfig {
                enabled: vec![],
                disabled: vec![],
            }),
        }
    }

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

def mark_cancelled(target, reason="cancelled"):
    if target is None:
        return
    state = pending.get(str(target)) or {}
    conv_id = state.get("conversation_id")
    pending[str(target)] = {"status": "cancelled", "conversation_id": conv_id}
    if conv_id:
        send({"jsonrpc": "2.0", "method": "codex/event", "params": {"type": "cancelled", "conversation_id": conv_id, "reason": reason}})
    send({"jsonrpc": "2.0", "id": target, "error": {"code": -32800, "message": reason}})

def handle_codex(req_id, params):
    conversation_id = params.get("conversation_id") or params.get("conversationId") or f"conv-{req_id}"
    pending[str(req_id)] = {"status": "pending", "conversation_id": conversation_id}
    def worker():
        time.sleep(0.05)
        state = pending.get(str(req_id))
        if not state or state.get("status") == "cancelled":
            return
        send({"jsonrpc": "2.0", "method": "codex/event", "params": {"type": "approval_required", "approval_id": f"ap-{req_id}", "kind": "exec"}})
        time.sleep(0.05)
        state = pending.get(str(req_id))
        if not state or state.get("status") == "cancelled":
            return
        send({"jsonrpc": "2.0", "method": "codex/event", "params": {"type": "task_complete", "conversation_id": conversation_id, "result": {"ok": True}}})
        send({"jsonrpc": "2.0", "id": req_id, "result": {"conversation_id": conversation_id, "output": {"ok": True}}})
        pending.pop(str(req_id), None)
    threading.Thread(target=worker, daemon=True).start()

for line in sys.stdin:
    if not line.strip():
        continue
    msg = json.loads(line)
    method = msg.get("method")
    if method == "initialize":
        send({"jsonrpc": "2.0", "id": msg.get("id"), "result": {"ready": True}})
    elif method == "tools/call":
        params = msg.get("params", {})
        tool = params.get("name")
        args = params.get("arguments", {})
        if tool in ["codex", "codex-reply"]:
            handle_codex(msg.get("id"), args)
    elif method == "$/cancelRequest":
        target = msg.get("params", {}).get("id")
        mark_cancelled(target, reason="client_cancel")
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

    fn write_fake_app_server() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().expect("tempdir");
        let script_path = dir.path().join("fake-codex-app");
        let script = r#"#!/usr/bin/env python3
import json
import sys
import threading
import time

pending = {}
turn_lookup = {}

def send(payload):
    sys.stdout.write(json.dumps(payload) + "\n")
    sys.stdout.flush()

def mark_cancelled(req_id, reason="cancelled"):
    if req_id is None:
        return
    state = pending.get(str(req_id)) or {}
    thread_id = state.get("thread_id") or "thread-unknown"
    turn_id = state.get("turn_id")
    pending[str(req_id)] = {"status": "cancelled", "thread_id": thread_id, "turn_id": turn_id}
    if turn_id:
        send({"jsonrpc": "2.0", "method": "task/notification", "params": {"type": "task_complete", "thread_id": thread_id, "turn_id": turn_id, "result": {"cancelled": True, "reason": reason}}})
    send({"jsonrpc": "2.0", "id": req_id, "error": {"code": -32800, "message": reason}})

def handle_turn(req_id, params):
    thread_id = params.get("threadId") or params.get("thread_id") or "thread-unknown"
    turn_id = params.get("turnId") or params.get("turn_id") or f"turn-{req_id}"
    pending[str(req_id)] = {"status": "pending", "thread_id": thread_id, "turn_id": turn_id}
    turn_lookup[turn_id] = req_id

    def worker():
        time.sleep(0.05)
        state = pending.get(str(req_id))
        if not state or state.get("status") == "cancelled":
            return
        send({"jsonrpc": "2.0", "method": "task/notification", "params": {"type": "item", "thread_id": thread_id, "turn_id": turn_id, "item": {"message": "processing"}}})
        time.sleep(0.05)
        state = pending.get(str(req_id))
        if not state or state.get("status") == "cancelled":
            return
        send({"jsonrpc": "2.0", "method": "task/notification", "params": {"type": "task_complete", "thread_id": thread_id, "turn_id": turn_id, "result": {"ok": True}}})
        send({"jsonrpc": "2.0", "id": req_id, "result": {"turn_id": turn_id, "accepted": True}})
        pending.pop(str(req_id), None)
        turn_lookup.pop(turn_id, None)

    threading.Thread(target=worker, daemon=True).start()

for line in sys.stdin:
    if not line.strip():
        continue
    msg = json.loads(line)
    method = msg.get("method")
    if method == "initialize":
        send({"jsonrpc": "2.0", "id": msg.get("id"), "result": {"ready": True}})
    elif method == "thread/start":
        params = msg.get("params", {})
        thread_id = params.get("thread_id") or f"thread-{msg.get('id')}"
        send({"jsonrpc": "2.0", "id": msg.get("id"), "result": {"thread_id": thread_id}})
    elif method == "thread/resume":
        params = msg.get("params", {})
        thread_id = params.get("threadId") or params.get("thread_id")
        send({"jsonrpc": "2.0", "id": msg.get("id"), "result": {"thread_id": thread_id, "resumed": True}})
    elif method == "turn/start":
        handle_turn(msg.get("id"), msg.get("params", {}))
    elif method == "turn/interrupt":
        params = msg.get("params", {})
        turn_id = params.get("turnId") or params.get("turn_id")
        req_id = turn_lookup.get(turn_id)
        if req_id:
            mark_cancelled(req_id, reason="interrupted")
            turn_lookup.pop(turn_id, None)
            pending.pop(str(req_id), None)
        send({"jsonrpc": "2.0", "id": msg.get("id"), "result": {"interrupted": True}})
    elif method == "$/cancelRequest":
        target = msg.get("params", {}).get("id")
        mark_cancelled(target, reason="client_cancel")
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

    fn write_env_probe_server(var: &str) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().expect("tempdir");
        let script_path = dir.path().join("env-probe-server");
        let script = format!(
            r#"#!/usr/bin/env python3
import os
import sys
import time

sys.stdout.write(os.environ.get("{var}", "") + "\n")
sys.stdout.flush()
time.sleep(30)
"#
        );

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

    fn test_client() -> ClientInfo {
        ClientInfo {
            name: "tests".to_string(),
            version: "0.0.0".to_string(),
        }
    }

    async fn start_fake_mcp_server() -> (tempfile::TempDir, CodexMcpServer) {
        let (dir, script) = write_fake_mcp_server();
        let config = test_config(script);
        let client = test_client();
        let server = CodexMcpServer::start(config, client)
            .await
            .expect("spawn mcp server");
        (dir, server)
    }

    async fn start_fake_app_server() -> (tempfile::TempDir, CodexAppServer) {
        let (dir, script) = write_fake_app_server();
        let config = test_config(script);
        let client = test_client();
        let server = CodexAppServer::start(config, client)
            .await
            .expect("spawn app server");
        (dir, server)
    }

    #[test]
    fn add_stdio_server_injects_env_and_persists() {
        let (dir, manager) = temp_config_manager();
        let env_key = "MCP_STDIO_TEST_KEY";
        env::remove_var(env_key);

        let mut env_map = BTreeMap::new();
        env_map.insert(env_key.to_string(), "secret".to_string());

        let added = manager
            .add_server(AddServerRequest {
                name: "local".into(),
                definition: stdio_definition("my-mcp"),
                overwrite: false,
                env: env_map,
                bearer_token: None,
            })
            .expect("add server");

        match added.definition.transport {
            McpTransport::Stdio(def) => {
                assert_eq!(def.command, "my-mcp");
                assert_eq!(def.env.get(env_key), Some(&"secret".to_string()));
            }
            _ => panic!("expected stdio transport"),
        }

        let listed = manager.list_servers().expect("list servers");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "local");

        let fetched = manager.get_server("local").expect("get server");
        match fetched.definition.transport {
            McpTransport::Stdio(def) => {
                assert_eq!(def.env.get(env_key), Some(&"secret".to_string()))
            }
            _ => panic!("expected stdio transport"),
        }

        let config_path = dir.path().join(DEFAULT_CONFIG_FILE);
        let serialized = fs::read_to_string(config_path).expect("read config");
        let value: TomlValue = serialized.parse().expect("parse toml");
        let table = value.as_table().expect("table root");
        let servers_table = table.get(MCP_SERVERS_KEY).expect("mcp_servers");
        let decoded: BTreeMap<String, McpServerDefinition> = servers_table
            .clone()
            .try_into()
            .expect("decode mcp_servers");
        let stored = decoded.get("local").expect("stored server");
        match &stored.transport {
            McpTransport::Stdio(def) => {
                assert_eq!(def.env.get(env_key), Some(&"secret".to_string()))
            }
            _ => panic!("expected stdio transport"),
        }

        assert_eq!(env::var(env_key).unwrap(), "secret");
        env::remove_var(env_key);
    }

    #[test]
    fn add_streamable_http_sets_token_and_allows_login_logout() {
        let (_dir, manager) = temp_config_manager();
        let env_var = "MCP_HTTP_TOKEN_E5";
        env::remove_var(env_var);

        let mut definition = streamable_definition("https://example.test/mcp", env_var);
        if let McpTransport::StreamableHttp(ref mut http) = definition.transport {
            http.headers.insert("X-Test".into(), "true".into());
        }

        let _added = manager
            .add_server(AddServerRequest {
                name: "remote".into(),
                definition,
                overwrite: false,
                env: BTreeMap::new(),
                bearer_token: Some("token-a".into()),
            })
            .expect("add server");

        assert_eq!(env::var(env_var).unwrap(), "token-a");

        let logout = manager.logout("remote").expect("logout");
        assert_eq!(logout.env_var.as_deref(), Some(env_var));
        assert!(logout.cleared);
        assert!(env::var(env_var).is_err());

        let login = manager.login("remote", "token-b").expect("login");
        assert_eq!(login.env_var.as_deref(), Some(env_var));
        assert_eq!(env::var(env_var).unwrap(), "token-b");

        env::remove_var(env_var);
    }

    #[test]
    fn remove_server_prunes_config() {
        let (_dir, manager) = temp_config_manager();

        manager
            .add_server(AddServerRequest {
                name: "one".into(),
                definition: stdio_definition("one"),
                overwrite: false,
                env: BTreeMap::new(),
                bearer_token: None,
            })
            .expect("add first");

        manager
            .add_server(AddServerRequest {
                name: "two".into(),
                definition: stdio_definition("two"),
                overwrite: false,
                env: BTreeMap::new(),
                bearer_token: None,
            })
            .expect("add second");

        let removed = manager.remove_server("one").expect("remove");
        assert!(removed.is_some());

        let listed = manager.list_servers().expect("list");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "two");

        let config = fs::read_to_string(manager.config_path()).expect("read config");
        let value: TomlValue = config.parse().expect("parse config");
        let table = value.as_table().expect("table root");
        let servers_value = table.get(MCP_SERVERS_KEY).cloned().expect("servers");
        let servers: BTreeMap<String, McpServerDefinition> =
            servers_value.try_into().expect("decode servers");
        assert!(!servers.contains_key("one"));
        assert!(servers.contains_key("two"));
    }

    #[test]
    fn runtime_stdio_server_resolves_env_and_tools() {
        let (_dir, manager) = temp_config_manager();
        let mut definition = stdio_definition("my-mcp");
        definition.description = Some("local mcp".into());
        definition.tags = vec!["dev".into(), "local".into()];
        definition.tools = Some(McpToolConfig {
            enabled: vec!["tool-a".into()],
            disabled: vec!["tool-b".into()],
        });

        if let McpTransport::Stdio(ref mut stdio) = definition.transport {
            stdio.args = vec!["--flag".into()];
            stdio.env.insert("EXAMPLE".into(), "value".into());
            stdio.timeout_ms = Some(2500);
        }

        let mut injected = BTreeMap::new();
        injected.insert("MCP_STDIO_INJECT_E6".into(), "yes".into());

        manager
            .add_server(AddServerRequest {
                name: "local".into(),
                definition,
                overwrite: false,
                env: injected,
                bearer_token: None,
            })
            .expect("add server");

        let runtime = manager.runtime_server("local").expect("runtime server");
        assert_eq!(runtime.name, "local");
        assert_eq!(runtime.description.as_deref(), Some("local mcp"));
        assert_eq!(runtime.tags, vec!["dev".to_string(), "local".to_string()]);

        let tools = runtime.tools.as_ref().expect("tool hints");
        assert_eq!(tools.enabled, vec!["tool-a".to_string()]);
        assert_eq!(tools.disabled, vec!["tool-b".to_string()]);

        match &runtime.transport {
            McpRuntimeTransport::Stdio(def) => {
                assert_eq!(def.command, "my-mcp");
                assert_eq!(def.args, vec!["--flag".to_string()]);
                assert_eq!(def.timeout_ms, Some(2500));
                assert_eq!(def.env.get("EXAMPLE").map(String::as_str), Some("value"));
                assert_eq!(
                    def.env.get("MCP_STDIO_INJECT_E6").map(String::as_str),
                    Some("yes")
                );
            }
            other => panic!("expected stdio transport, got {other:?}"),
        }

        serde_json::to_string(&runtime).expect("serialize runtime");
        env::remove_var("MCP_STDIO_INJECT_E6");
    }

    #[test]
    fn runtime_http_resolves_bearer_and_sets_header() {
        let (_dir, manager) = temp_config_manager();
        let env_var = "MCP_HTTP_TOKEN_E6";
        env::set_var(env_var, "token-123");

        let mut definition = streamable_definition("https://example.test/mcp", env_var);
        if let McpTransport::StreamableHttp(ref mut http) = definition.transport {
            http.headers.insert("X-Test".into(), "true".into());
            http.connect_timeout_ms = Some(1200);
            http.request_timeout_ms = Some(3400);
        }

        manager
            .add_server(AddServerRequest {
                name: "remote".into(),
                definition,
                overwrite: false,
                env: BTreeMap::new(),
                bearer_token: None,
            })
            .expect("add server");

        let runtime = manager.runtime_server("remote").expect("runtime server");
        match &runtime.transport {
            McpRuntimeTransport::StreamableHttp(def) => {
                assert_eq!(def.url, "https://example.test/mcp");
                assert_eq!(def.bearer_env_var.as_deref(), Some(env_var));
                assert_eq!(def.bearer_token.as_deref(), Some("token-123"));
                assert_eq!(def.headers.get("X-Test").map(String::as_str), Some("true"));
                assert_eq!(
                    def.headers.get("Authorization").map(String::as_str),
                    Some("Bearer token-123")
                );
                assert_eq!(def.connect_timeout_ms, Some(1200));
                assert_eq!(def.request_timeout_ms, Some(3400));
            }
            other => panic!("expected streamable_http transport, got {other:?}"),
        }

        let serialized = serde_json::to_value(&runtime).expect("serialize runtime");
        assert!(serialized.get("transport").is_some());

        env::remove_var(env_var);
    }

    #[test]
    fn runtime_http_preserves_existing_auth_header() {
        let (_dir, manager) = temp_config_manager();
        let env_var = "MCP_HTTP_TOKEN_E6B";
        env::set_var(env_var, "token-override");

        let mut definition = streamable_definition("https://example.test/custom", env_var);
        if let McpTransport::StreamableHttp(ref mut http) = definition.transport {
            http.headers
                .insert("Authorization".into(), "Custom 123".into());
        }

        manager
            .add_server(AddServerRequest {
                name: "remote-custom".into(),
                definition,
                overwrite: false,
                env: BTreeMap::new(),
                bearer_token: None,
            })
            .expect("add server");

        let runtime = manager
            .runtime_server("remote-custom")
            .expect("runtime server");
        match &runtime.transport {
            McpRuntimeTransport::StreamableHttp(def) => {
                assert_eq!(def.bearer_token.as_deref(), Some("token-override"));
                assert_eq!(
                    def.headers.get("Authorization").map(String::as_str),
                    Some("Custom 123")
                );
            }
            other => panic!("expected streamable_http transport, got {other:?}"),
        }

        env::remove_var(env_var);
    }

    #[test]
    fn runtime_stdio_launcher_merges_env_timeout_and_tools() {
        let base_dir = tempfile::tempdir().expect("tempdir");
        let code_home = base_dir.path().join("code_home");

        let defaults = StdioServerConfig {
            binary: PathBuf::from("codex"),
            code_home: Some(code_home.clone()),
            current_dir: Some(base_dir.path().to_path_buf()),
            env: vec![
                (OsString::from("BASE_ONLY"), OsString::from("base")),
                (OsString::from("OVERRIDE_ME"), OsString::from("base")),
            ],
            mirror_stdio: true,
            startup_timeout: Duration::from_secs(5),
        };

        let mut definition = StdioServerDefinition {
            command: "my-mcp".into(),
            args: vec!["--flag".into()],
            env: BTreeMap::new(),
            timeout_ms: Some(1500),
        };
        definition
            .env
            .insert("OVERRIDE_ME".into(), "runtime".into());
        definition
            .env
            .insert("RUNTIME_ONLY".into(), "runtime-env".into());

        let runtime = McpRuntimeServer {
            name: "local".into(),
            transport: McpRuntimeTransport::Stdio(definition),
            description: Some("example".into()),
            tags: vec!["dev".into()],
            tools: Some(McpToolConfig {
                enabled: vec!["tool-1".into()],
                disabled: vec!["tool-2".into()],
            }),
        };

        let launcher = runtime.into_launcher(&defaults);
        assert_eq!(launcher.name, "local");
        assert_eq!(launcher.description.as_deref(), Some("example"));
        assert_eq!(launcher.tags, vec!["dev".to_string()]);

        let tools = launcher.tools.clone().expect("tool hints");
        assert_eq!(tools.enabled, vec!["tool-1".to_string()]);
        assert_eq!(tools.disabled, vec!["tool-2".to_string()]);

        match launcher.transport {
            McpServerLauncherTransport::Stdio(launch) => {
                assert_eq!(launch.command, PathBuf::from("my-mcp"));
                assert_eq!(launch.args, vec!["--flag".to_string()]);
                assert_eq!(launch.current_dir.as_ref(), defaults.current_dir.as_ref());
                assert_eq!(launch.timeout, Duration::from_millis(1500));
                assert!(launch.mirror_stdio);

                let env_map: HashMap<OsString, OsString> = launch.env.into_iter().collect();
                assert_eq!(
                    env_map.get(&OsString::from("BASE_ONLY")),
                    Some(&OsString::from("base"))
                );
                assert_eq!(
                    env_map.get(&OsString::from("OVERRIDE_ME")),
                    Some(&OsString::from("runtime"))
                );
                assert_eq!(
                    env_map.get(&OsString::from("RUNTIME_ONLY")),
                    Some(&OsString::from("runtime-env"))
                );
                assert_eq!(
                    env_map.get(&OsString::from("CODEX_HOME")),
                    Some(&code_home.as_os_str().to_os_string())
                );
            }
            other => panic!("expected stdio launcher, got {other:?}"),
        }
    }

    #[test]
    fn streamable_http_connector_converts_timeouts_and_headers() {
        let env_var = "MCP_HTTP_TOKEN_E7";
        env::set_var(env_var, "token-launcher");

        let mut definition = StreamableHttpDefinition {
            url: "https://example.test/stream".into(),
            headers: BTreeMap::new(),
            bearer_env_var: Some(env_var.to_string()),
            connect_timeout_ms: Some(1200),
            request_timeout_ms: Some(3400),
        };
        definition.headers.insert("X-Test".into(), "true".into());

        let runtime = McpRuntimeServer {
            name: "remote".into(),
            transport: McpRuntimeTransport::StreamableHttp(resolve_streamable_http(definition)),
            description: None,
            tags: vec!["http".into()],
            tools: Some(McpToolConfig {
                enabled: vec!["tool-a".into()],
                disabled: vec![],
            }),
        };

        let defaults = StdioServerConfig {
            binary: PathBuf::from("codex"),
            code_home: None,
            current_dir: None,
            env: Vec::new(),
            mirror_stdio: false,
            startup_timeout: Duration::from_secs(2),
        };

        let launcher = runtime.into_launcher(&defaults);
        match launcher.transport {
            McpServerLauncherTransport::StreamableHttp(connector) => {
                assert_eq!(connector.url, "https://example.test/stream");
                assert_eq!(
                    connector.headers.get("X-Test").map(String::as_str),
                    Some("true")
                );
                assert_eq!(
                    connector.headers.get("Authorization").map(String::as_str),
                    Some("Bearer token-launcher")
                );
                assert_eq!(connector.connect_timeout, Some(Duration::from_millis(1200)));
                assert_eq!(connector.request_timeout, Some(Duration::from_millis(3400)));
                assert_eq!(connector.bearer_env_var.as_deref(), Some(env_var));
                assert_eq!(connector.bearer_token.as_deref(), Some("token-launcher"));

                let tools = launcher.tools.as_ref().expect("tool hints present");
                assert_eq!(tools.enabled, vec!["tool-a".to_string()]);
                assert!(tools.disabled.is_empty());
            }
            other => panic!("expected http connector, got {other:?}"),
        }

        env::remove_var(env_var);
    }

    #[tokio::test]
    async fn codex_flow_streams_events_and_response() {
        let (_dir, server) = start_fake_mcp_server().await;

        let params = CodexCallParams {
            prompt: "hello".into(),
            model: None,
            cwd: None,
            sandbox: None,
            approval_policy: None,
            profile: None,
            config: BTreeMap::new(),
        };

        let mut handle = server.codex(params).await.expect("codex call");

        let first_event = time::timeout(Duration::from_secs(2), handle.events.recv())
            .await
            .expect("event timeout")
            .expect("event value");
        match first_event {
            CodexEvent::ApprovalRequired(req) => {
                assert!(req.approval_id.starts_with("ap-"));
                assert_eq!(req.kind, ApprovalKind::Exec);
            }
            other => panic!("unexpected event: {other:?}"),
        }

        let second_event = time::timeout(Duration::from_secs(2), handle.events.recv())
            .await
            .expect("event timeout")
            .expect("event value");
        let event_conversation = match second_event {
            CodexEvent::TaskComplete {
                conversation_id, ..
            } => {
                assert!(!conversation_id.is_empty());
                conversation_id
            }
            other => panic!("unexpected event: {other:?}"),
        };

        let response = time::timeout(Duration::from_secs(2), handle.response)
            .await
            .expect("response timeout")
            .expect("response recv");
        let response = response.expect("response ok");
        assert_eq!(
            response.conversation_id.as_deref(),
            Some(event_conversation.as_str())
        );
        assert_eq!(response.output, serde_json::json!({ "ok": true }));

        let _ = server.shutdown().await;
    }

    #[tokio::test]
    async fn canceling_request_returns_cancelled_error() {
        let (_dir, server) = start_fake_mcp_server().await;

        let params = CodexCallParams {
            prompt: "cancel me".into(),
            model: None,
            cwd: None,
            sandbox: None,
            approval_policy: None,
            profile: None,
            config: BTreeMap::new(),
        };

        let mut handle = server.codex(params).await.expect("codex call");
        server.cancel(handle.request_id).expect("cancel send");

        let expected_conversation = format!("conv-{}", handle.request_id);
        let cancel_event = time::timeout(Duration::from_secs(2), handle.events.recv())
            .await
            .expect("event timeout")
            .expect("cancel notification");
        match cancel_event {
            CodexEvent::Cancelled {
                conversation_id,
                reason,
            } => {
                assert_eq!(
                    conversation_id.as_deref(),
                    Some(expected_conversation.as_str())
                );
                assert_eq!(reason.as_deref(), Some("client_cancel"));
            }
            other => panic!("expected cancellation event, got {other:?}"),
        }

        let response = time::timeout(Duration::from_secs(2), handle.response)
            .await
            .expect("response timeout")
            .expect("recv");
        assert!(matches!(response, Err(McpError::Cancelled)));

        let _ = server.shutdown().await;
    }

    #[tokio::test]
    async fn codex_reply_streams_follow_up_notifications() {
        let (_dir, server) = start_fake_mcp_server().await;

        let params = CodexCallParams {
            prompt: "hello".into(),
            model: None,
            cwd: None,
            sandbox: None,
            approval_policy: None,
            profile: None,
            config: BTreeMap::new(),
        };
        let first = server.codex(params).await.expect("start codex");
        let first_response = time::timeout(Duration::from_secs(2), first.response)
            .await
            .expect("response timeout")
            .expect("recv")
            .expect("ok");
        let conversation_id = first_response.conversation_id.expect("conversation id set");
        assert!(!conversation_id.is_empty());

        let reply_params = CodexReplyParams {
            conversation_id: conversation_id.clone(),
            prompt: "follow up".into(),
        };
        let mut reply = server.codex_reply(reply_params).await.expect("codex reply");

        let expected_approval = format!("ap-{}", reply.request_id);
        let approval = time::timeout(Duration::from_secs(2), reply.events.recv())
            .await
            .expect("event timeout")
            .expect("approval");
        match approval {
            CodexEvent::ApprovalRequired(req) => {
                assert_eq!(req.approval_id, expected_approval);
                assert_eq!(req.kind, ApprovalKind::Exec);
            }
            other => panic!("unexpected event: {other:?}"),
        }

        let complete = time::timeout(Duration::from_secs(2), reply.events.recv())
            .await
            .expect("event timeout")
            .expect("task completion");
        match complete {
            CodexEvent::TaskComplete {
                conversation_id: event_conv,
                ..
            } => assert_eq!(event_conv, conversation_id),
            other => panic!("unexpected event: {other:?}"),
        }

        let reply_response = time::timeout(Duration::from_secs(2), reply.response)
            .await
            .expect("response timeout")
            .expect("recv")
            .expect("ok");
        assert_eq!(
            reply_response.conversation_id.as_deref(),
            Some(conversation_id.as_str())
        );
        assert_eq!(reply_response.output, serde_json::json!({ "ok": true }));

        let _ = server.shutdown().await;
    }

    #[tokio::test]
    async fn app_flow_streams_notifications_and_response() {
        let (_dir, server) = start_fake_app_server().await;

        let thread_params = ThreadStartParams {
            thread_id: None,
            metadata: Value::Null,
        };
        let thread_handle = server
            .thread_start(thread_params)
            .await
            .expect("thread start");
        let thread_response = time::timeout(Duration::from_secs(2), thread_handle.response)
            .await
            .expect("thread response timeout")
            .expect("thread response recv")
            .expect("thread response ok");
        let thread_id = thread_response
            .get("thread_id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        assert!(!thread_id.is_empty());

        let params = TurnStartParams {
            thread_id: thread_id.clone(),
            input: vec![TurnInput {
                kind: "text".to_string(),
                text: Some("hi".to_string()),
            }],
            model: None,
            config: BTreeMap::new(),
        };
        let mut handle = server.turn_start(params).await.expect("turn start");

        let first_event = time::timeout(Duration::from_secs(2), handle.events.recv())
            .await
            .expect("event timeout")
            .expect("event value");
        let turn_id = match first_event {
            AppNotification::Item {
                thread_id: tid,
                turn_id: Some(turn),
                item,
            } => {
                assert_eq!(tid, thread_id);
                assert!(item.get("message").is_some());
                turn
            }
            other => panic!("unexpected event: {other:?}"),
        };

        let second_event = time::timeout(Duration::from_secs(2), handle.events.recv())
            .await
            .expect("event timeout")
            .expect("event value");
        match second_event {
            AppNotification::TaskComplete {
                thread_id: tid,
                turn_id: event_turn,
                result,
            } => {
                assert_eq!(tid, thread_id);
                assert_eq!(event_turn.as_deref(), Some(turn_id.as_str()));
                assert_eq!(result, serde_json::json!({ "ok": true }));
            }
            other => panic!("unexpected event: {other:?}"),
        }

        let response = time::timeout(Duration::from_secs(2), handle.response)
            .await
            .expect("response timeout")
            .expect("response recv");
        let response = response.expect("response ok");
        assert_eq!(
            response
                .get("turn_id")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            turn_id
        );

        let _ = server.shutdown().await;
    }

    #[tokio::test]
    async fn canceling_app_request_returns_cancelled_error() {
        let (_dir, server) = start_fake_app_server().await;

        let thread_params = ThreadStartParams {
            thread_id: None,
            metadata: Value::Null,
        };
        let thread_handle = server
            .thread_start(thread_params)
            .await
            .expect("thread start");
        let thread_response = time::timeout(Duration::from_secs(2), thread_handle.response)
            .await
            .expect("thread response timeout")
            .expect("thread response recv")
            .expect("thread response ok");
        let thread_id = thread_response
            .get("thread_id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();

        let params = TurnStartParams {
            thread_id: thread_id.clone(),
            input: vec![TurnInput {
                kind: "text".to_string(),
                text: Some("cancel me".to_string()),
            }],
            model: None,
            config: BTreeMap::new(),
        };

        let mut handle = server.turn_start(params).await.expect("turn start");
        server.cancel(handle.request_id).expect("send cancel");

        let cancel_event = time::timeout(Duration::from_secs(2), handle.events.recv())
            .await
            .expect("event timeout")
            .expect("cancel event");
        match cancel_event {
            AppNotification::TaskComplete {
                thread_id: tid,
                turn_id,
                result,
            } => {
                assert_eq!(tid, thread_id);
                assert!(turn_id.is_some());
                assert_eq!(result.get("cancelled"), Some(&Value::Bool(true)));
                assert_eq!(
                    result.get("reason"),
                    Some(&Value::String("client_cancel".into()))
                );
            }
            other => panic!("unexpected cancellation notification: {other:?}"),
        }

        let response = time::timeout(Duration::from_secs(2), handle.response)
            .await
            .expect("response timeout")
            .expect("recv");
        assert!(matches!(response, Err(McpError::Cancelled)));

        let _ = server.shutdown().await;
    }

    #[tokio::test]
    async fn thread_resume_allows_follow_up_turns() {
        let (_dir, server) = start_fake_app_server().await;

        let thread_params = ThreadStartParams {
            thread_id: None,
            metadata: Value::Null,
        };
        let thread_handle = server
            .thread_start(thread_params)
            .await
            .expect("thread start");
        let thread_response = time::timeout(Duration::from_secs(2), thread_handle.response)
            .await
            .expect("thread response timeout")
            .expect("recv")
            .expect("ok");
        let thread_id = thread_response
            .get("thread_id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();

        let resume_params = ThreadResumeParams {
            thread_id: thread_id.clone(),
        };
        let resume_handle = server
            .thread_resume(resume_params)
            .await
            .expect("thread resume");
        let resume_response = time::timeout(Duration::from_secs(2), resume_handle.response)
            .await
            .expect("resume response timeout")
            .expect("recv")
            .expect("ok");
        assert_eq!(
            resume_response
                .get("thread_id")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            thread_id
        );
        assert!(resume_response
            .get("resumed")
            .and_then(Value::as_bool)
            .unwrap_or(false));

        let params = TurnStartParams {
            thread_id: thread_id.clone(),
            input: vec![TurnInput {
                kind: "text".to_string(),
                text: Some("resume flow".to_string()),
            }],
            model: None,
            config: BTreeMap::new(),
        };
        let mut turn = server.turn_start(params).await.expect("turn start");

        let item = time::timeout(Duration::from_secs(2), turn.events.recv())
            .await
            .expect("event timeout")
            .expect("item event");
        let turn_id = match item {
            AppNotification::Item {
                thread_id: tid,
                turn_id: Some(turn_id),
                ..
            } => {
                assert_eq!(tid, thread_id);
                turn_id
            }
            other => panic!("unexpected event: {other:?}"),
        };

        let complete = time::timeout(Duration::from_secs(2), turn.events.recv())
            .await
            .expect("event timeout")
            .expect("completion event");
        match complete {
            AppNotification::TaskComplete {
                thread_id: tid,
                turn_id: event_turn,
                result,
            } => {
                assert_eq!(tid, thread_id);
                assert_eq!(event_turn.as_deref(), Some(turn_id.as_str()));
                assert_eq!(result, serde_json::json!({ "ok": true }));
            }
            other => panic!("unexpected event: {other:?}"),
        }

        let turn_response = time::timeout(Duration::from_secs(2), turn.response)
            .await
            .expect("response timeout")
            .expect("recv")
            .expect("ok");
        assert_eq!(
            turn_response
                .get("turn_id")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            turn_id
        );

        let _ = server.shutdown().await;
    }

    #[tokio::test]
    async fn turn_interrupt_sends_cancel_notification() {
        let (_dir, server) = start_fake_app_server().await;

        let thread_params = ThreadStartParams {
            thread_id: None,
            metadata: Value::Null,
        };
        let thread_handle = server
            .thread_start(thread_params)
            .await
            .expect("thread start");
        let thread_response = time::timeout(Duration::from_secs(2), thread_handle.response)
            .await
            .expect("thread response timeout")
            .expect("recv")
            .expect("ok");
        let thread_id = thread_response
            .get("thread_id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();

        let params = TurnStartParams {
            thread_id: thread_id.clone(),
            input: vec![TurnInput {
                kind: "text".to_string(),
                text: Some("please interrupt".to_string()),
            }],
            model: None,
            config: BTreeMap::new(),
        };
        let mut turn = server.turn_start(params).await.expect("turn start");

        let first_event = time::timeout(Duration::from_secs(2), turn.events.recv())
            .await
            .expect("event timeout")
            .expect("event value");
        let turn_id = match first_event {
            AppNotification::Item {
                thread_id: tid,
                turn_id: Some(turn),
                ..
            } => {
                assert_eq!(tid, thread_id);
                turn
            }
            other => panic!("unexpected event: {other:?}"),
        };

        let interrupt = server
            .turn_interrupt(TurnInterruptParams {
                thread_id: Some(thread_id.clone()),
                turn_id: turn_id.clone(),
            })
            .await
            .expect("send interrupt");

        let cancel_event = time::timeout(Duration::from_secs(2), turn.events.recv())
            .await
            .expect("event timeout")
            .expect("cancel event");
        match cancel_event {
            AppNotification::TaskComplete {
                thread_id: tid,
                turn_id: event_turn,
                result,
            } => {
                assert_eq!(tid, thread_id);
                assert_eq!(event_turn.as_deref(), Some(turn_id.as_str()));
                assert_eq!(result.get("cancelled"), Some(&Value::Bool(true)));
                assert_eq!(
                    result.get("reason"),
                    Some(&Value::String("interrupted".into()))
                );
            }
            other => panic!("unexpected cancel notification: {other:?}"),
        }

        let turn_response = time::timeout(Duration::from_secs(2), turn.response)
            .await
            .expect("response timeout")
            .expect("recv");
        assert!(matches!(turn_response, Err(McpError::Cancelled)));

        let interrupt_response = time::timeout(Duration::from_secs(2), interrupt.response)
            .await
            .expect("interrupt response timeout")
            .expect("recv")
            .expect("ok");
        assert!(interrupt_response
            .get("interrupted")
            .and_then(Value::as_bool)
            .unwrap_or(false));

        let _ = server.shutdown().await;
    }

    #[test]
    fn runtime_api_lists_launchers_without_changing_config() {
        let (dir, manager) = temp_config_manager();
        let stdio_env_key = "MCP_RUNTIME_API_STDIO_ENV";
        let request_env_key = "MCP_RUNTIME_API_REQUEST_ENV";
        let http_env_key = "MCP_RUNTIME_API_HTTP_ENV";
        env::set_var(http_env_key, "token-api");

        let mut stdio = stdio_definition("runtime-api-stdio");
        stdio.description = Some("stdio runtime".into());
        stdio.tags = vec!["local".into()];
        stdio.tools = Some(McpToolConfig {
            enabled: vec!["fmt".into()],
            disabled: vec!["lint".into()],
        });
        if let McpTransport::Stdio(ref mut stdio_def) = stdio.transport {
            stdio_def.args.push("--flag".into());
            stdio_def
                .env
                .insert(stdio_env_key.into(), "runtime-env".into());
            stdio_def.timeout_ms = Some(2400);
        }

        let mut env_map = BTreeMap::new();
        env_map.insert(request_env_key.to_string(), "injected".to_string());

        manager
            .add_server(AddServerRequest {
                name: "local-api".into(),
                definition: stdio,
                overwrite: false,
                env: env_map,
                bearer_token: None,
            })
            .expect("add stdio server");

        let mut http = streamable_definition("https://example.test/runtime-api", http_env_key);
        http.description = Some("http runtime".into());
        http.tags = vec!["remote".into()];
        http.tools = Some(McpToolConfig {
            enabled: vec!["alpha".into()],
            disabled: vec!["beta".into()],
        });
        if let McpTransport::StreamableHttp(ref mut http_def) = http.transport {
            http_def.headers.insert("X-Req".into(), "true".into());
            http_def.request_timeout_ms = Some(2200);
        }

        manager
            .add_server(AddServerRequest {
                name: "remote-api".into(),
                definition: http,
                overwrite: false,
                env: BTreeMap::new(),
                bearer_token: None,
            })
            .expect("add http server");

        let before = fs::read_to_string(manager.config_path()).expect("read config before");
        let cwd = dir.path().join("cwd");

        let defaults = StdioServerConfig {
            binary: PathBuf::from("codex"),
            code_home: Some(dir.path().to_path_buf()),
            current_dir: Some(cwd.clone()),
            env: vec![
                (OsString::from("DEFAULT_ONLY"), OsString::from("default")),
                (
                    OsString::from(request_env_key),
                    OsString::from("base-default"),
                ),
            ],
            mirror_stdio: true,
            startup_timeout: Duration::from_secs(3),
        };

        let api = McpRuntimeApi::from_config(&manager, &defaults).expect("runtime api");

        let available = api.available();
        assert_eq!(available.len(), 2);

        let stdio_summary = available
            .iter()
            .find(|entry| entry.name == "local-api")
            .expect("stdio summary");
        assert_eq!(stdio_summary.transport, McpRuntimeSummaryTransport::Stdio);
        let stdio_tools = stdio_summary.tools.as_ref().expect("stdio tools");
        assert_eq!(stdio_tools.enabled, vec!["fmt".to_string()]);
        assert_eq!(stdio_tools.disabled, vec!["lint".to_string()]);

        let stdio_launcher = api.stdio_launcher("local-api").expect("stdio launcher");
        assert_eq!(stdio_launcher.args, vec!["--flag".to_string()]);
        assert_eq!(stdio_launcher.timeout, Duration::from_millis(2400));
        assert!(stdio_launcher.mirror_stdio);
        assert_eq!(stdio_launcher.current_dir.as_deref(), Some(cwd.as_path()));

        let env_map: HashMap<OsString, OsString> = stdio_launcher.env.into_iter().collect();
        assert_eq!(
            env_map.get(&OsString::from("CODEX_HOME")),
            Some(&dir.path().as_os_str().to_os_string())
        );
        assert_eq!(
            env_map.get(&OsString::from("DEFAULT_ONLY")),
            Some(&OsString::from("default"))
        );
        assert_eq!(
            env_map.get(&OsString::from(request_env_key)),
            Some(&OsString::from("injected"))
        );
        assert_eq!(
            env_map.get(&OsString::from(stdio_env_key)),
            Some(&OsString::from("runtime-env"))
        );

        let http_connector = api.http_connector("remote-api").expect("http connector");
        assert_eq!(http_connector.bearer_token.as_deref(), Some("token-api"));
        assert_eq!(
            http_connector
                .headers
                .get("Authorization")
                .map(String::as_str),
            Some("Bearer token-api")
        );
        assert_eq!(
            http_connector.headers.get("X-Req").map(String::as_str),
            Some("true")
        );
        assert_eq!(
            http_connector.request_timeout,
            Some(Duration::from_millis(2200))
        );

        let http_tools = available
            .iter()
            .find(|entry| entry.name == "remote-api")
            .and_then(|entry| entry.tools.as_ref())
            .expect("http tools");
        assert_eq!(http_tools.enabled, vec!["alpha".to_string()]);
        assert_eq!(http_tools.disabled, vec!["beta".to_string()]);

        match api.stdio_launcher("remote-api") {
            Err(McpRuntimeError::UnsupportedTransport {
                name,
                expected,
                actual,
            }) => {
                assert_eq!(name, "remote-api");
                assert_eq!(expected, "stdio");
                assert_eq!(actual, "streamable_http");
            }
            other => panic!("unexpected result: {other:?}"),
        }

        match api.http_connector("local-api") {
            Err(McpRuntimeError::UnsupportedTransport {
                name,
                expected,
                actual,
            }) => {
                assert_eq!(name, "local-api");
                assert_eq!(expected, "streamable_http");
                assert_eq!(actual, "stdio");
            }
            other => panic!("unexpected http result: {other:?}"),
        }

        let after = fs::read_to_string(manager.config_path()).expect("read config after");
        assert_eq!(before, after);

        env::remove_var(http_env_key);
        env::remove_var(request_env_key);
    }

    #[test]
    fn runtime_api_prepare_http_is_non_destructive() {
        let (dir, manager) = temp_config_manager();
        let env_var = "MCP_RUNTIME_API_PREPARE";
        env::set_var(env_var, "prepare-token");

        let mut http = streamable_definition("https://example.test/prepare", env_var);
        http.tags = vec!["prepare".into()];
        http.tools = Some(McpToolConfig {
            enabled: vec!["delta".into()],
            disabled: vec![],
        });

        manager
            .add_server(AddServerRequest {
                name: "prepare-http".into(),
                definition: http,
                overwrite: false,
                env: BTreeMap::new(),
                bearer_token: None,
            })
            .expect("add http server");

        let before = fs::read_to_string(manager.config_path()).expect("read config before");

        let defaults = StdioServerConfig {
            binary: PathBuf::from("codex"),
            code_home: Some(dir.path().to_path_buf()),
            current_dir: None,
            env: Vec::new(),
            mirror_stdio: false,
            startup_timeout: Duration::from_secs(2),
        };

        let api = McpRuntimeApi::from_config(&manager, &defaults).expect("runtime api");
        let handle = api.prepare("prepare-http").expect("prepare http");

        match handle {
            McpRuntimeHandle::StreamableHttp(http_handle) => {
                assert_eq!(http_handle.name, "prepare-http");
                assert_eq!(
                    http_handle.connector.bearer_token.as_deref(),
                    Some("prepare-token")
                );
                assert_eq!(
                    http_handle
                        .connector
                        .headers
                        .get("Authorization")
                        .map(String::as_str),
                    Some("Bearer prepare-token")
                );
                let tools = http_handle.tools.expect("tool hints");
                assert_eq!(tools.enabled, vec!["delta".to_string()]);
            }
            other => panic!("expected http handle, got {other:?}"),
        }

        let after = fs::read_to_string(manager.config_path()).expect("read config after");
        assert_eq!(before, after);

        env::remove_var(env_var);
    }

    #[test]
    fn app_runtime_api_lists_and_merges_without_writes() {
        let (dir, manager) = temp_config_manager();

        let alpha_home = dir.path().join("app-home-a");
        let alpha_cwd = dir.path().join("app-cwd-a");
        let mut alpha_env = BTreeMap::new();
        alpha_env.insert("APP_RUNTIME_ENV".into(), "alpha".into());
        alpha_env.insert("OVERRIDE_ME".into(), "runtime".into());

        manager
            .add_app_runtime(AddAppRuntimeRequest {
                name: "alpha".into(),
                definition: AppRuntimeDefinition {
                    description: Some("local app".into()),
                    tags: vec!["local".into()],
                    env: alpha_env,
                    code_home: Some(alpha_home.clone()),
                    current_dir: Some(alpha_cwd.clone()),
                    mirror_stdio: Some(true),
                    startup_timeout_ms: Some(4200),
                    binary: Some(PathBuf::from("/bin/app-alpha")),
                    metadata: serde_json::json!({"thread": "t-alpha"}),
                },
                overwrite: false,
            })
            .expect("add alpha app runtime");

        let mut beta_env = BTreeMap::new();
        beta_env.insert("APP_RUNTIME_ENV".into(), "beta".into());

        manager
            .add_app_runtime(AddAppRuntimeRequest {
                name: "beta".into(),
                definition: AppRuntimeDefinition {
                    description: None,
                    tags: vec!["default".into()],
                    env: beta_env,
                    code_home: None,
                    current_dir: None,
                    mirror_stdio: None,
                    startup_timeout_ms: None,
                    binary: None,
                    metadata: serde_json::json!({"resume": true}),
                },
                overwrite: false,
            })
            .expect("add beta app runtime");

        let before = fs::read_to_string(manager.config_path()).expect("read config before");

        let default_home = dir.path().join("default-home");
        let default_cwd = dir.path().join("default-cwd");
        let defaults = StdioServerConfig {
            binary: PathBuf::from("codex"),
            code_home: Some(default_home.clone()),
            current_dir: Some(default_cwd.clone()),
            env: vec![
                (OsString::from("DEFAULT_ONLY"), OsString::from("base")),
                (OsString::from("OVERRIDE_ME"), OsString::from("base")),
            ],
            mirror_stdio: false,
            startup_timeout: Duration::from_secs(3),
        };

        let api = AppRuntimeApi::from_config(&manager, &defaults).expect("app runtime api");

        let available = api.available();
        assert_eq!(available.len(), 2);

        let alpha_summary = available
            .iter()
            .find(|entry| entry.name == "alpha")
            .expect("alpha summary");
        assert_eq!(alpha_summary.description.as_deref(), Some("local app"));
        assert_eq!(alpha_summary.tags, vec!["local".to_string()]);
        assert_eq!(
            alpha_summary.metadata,
            serde_json::json!({"thread": "t-alpha"})
        );

        let alpha = api.prepare("alpha").expect("prepare alpha");
        assert_eq!(alpha.name, "alpha");
        assert_eq!(alpha.metadata, serde_json::json!({"thread": "t-alpha"}));
        assert_eq!(alpha.config.binary, PathBuf::from("/bin/app-alpha"));
        assert_eq!(
            alpha.config.code_home.as_deref(),
            Some(alpha_home.as_path())
        );
        assert_eq!(
            alpha.config.current_dir.as_deref(),
            Some(alpha_cwd.as_path())
        );
        assert!(alpha.config.mirror_stdio);
        assert_eq!(alpha.config.startup_timeout, Duration::from_millis(4200));

        let alpha_env: HashMap<OsString, OsString> = alpha.config.env.into_iter().collect();
        assert_eq!(
            alpha_env.get(&OsString::from("CODEX_HOME")),
            Some(&alpha_home.as_os_str().to_os_string())
        );
        assert_eq!(
            alpha_env.get(&OsString::from("DEFAULT_ONLY")),
            Some(&OsString::from("base"))
        );
        assert_eq!(
            alpha_env.get(&OsString::from("OVERRIDE_ME")),
            Some(&OsString::from("runtime"))
        );
        assert_eq!(
            alpha_env.get(&OsString::from("APP_RUNTIME_ENV")),
            Some(&OsString::from("alpha"))
        );

        let beta = api.stdio_config("beta").expect("beta config");
        assert_eq!(beta.binary, PathBuf::from("codex"));
        assert_eq!(beta.code_home.as_deref(), Some(default_home.as_path()));
        assert_eq!(beta.current_dir.as_deref(), Some(default_cwd.as_path()));
        assert!(!beta.mirror_stdio);
        assert_eq!(beta.startup_timeout, Duration::from_secs(3));

        let beta_env: HashMap<OsString, OsString> = beta.env.into_iter().collect();
        assert_eq!(
            beta_env.get(&OsString::from("CODEX_HOME")),
            Some(&default_home.as_os_str().to_os_string())
        );
        assert_eq!(
            beta_env.get(&OsString::from("DEFAULT_ONLY")),
            Some(&OsString::from("base"))
        );
        assert_eq!(
            beta_env.get(&OsString::from("OVERRIDE_ME")),
            Some(&OsString::from("base"))
        );
        assert_eq!(
            beta_env.get(&OsString::from("APP_RUNTIME_ENV")),
            Some(&OsString::from("beta"))
        );

        let beta_summary = available
            .iter()
            .find(|entry| entry.name == "beta")
            .expect("beta summary");
        assert_eq!(beta_summary.metadata, serde_json::json!({"resume": true}));

        let after = fs::read_to_string(manager.config_path()).expect("read config after");
        assert_eq!(before, after);
    }

    #[tokio::test]
    async fn app_runtime_lifecycle_starts_and_stops_without_mutation() {
        let (config_dir, manager) = temp_config_manager();
        let (_server_dir, server_path) = write_fake_app_server();
        let code_home = config_dir.path().join("app-lifecycle-home");

        let mut env_map = BTreeMap::new();
        env_map.insert("APP_RUNTIME_LIFECYCLE".into(), "runtime-env".into());

        let metadata = serde_json::json!({"resume_thread": "thread-lifecycle"});
        manager
            .add_app_runtime(AddAppRuntimeRequest {
                name: "lifecycle".into(),
                definition: AppRuntimeDefinition {
                    description: Some("app lifecycle".into()),
                    tags: vec!["app".into()],
                    env: env_map,
                    code_home: None,
                    current_dir: None,
                    mirror_stdio: Some(true),
                    startup_timeout_ms: Some(1500),
                    binary: None,
                    metadata: metadata.clone(),
                },
                overwrite: false,
            })
            .expect("add app runtime");

        let defaults = StdioServerConfig {
            binary: server_path.clone(),
            code_home: Some(code_home.clone()),
            current_dir: None,
            env: vec![(
                OsString::from("APP_RUNTIME_LIFECYCLE"),
                OsString::from("default"),
            )],
            mirror_stdio: false,
            startup_timeout: Duration::from_secs(3),
        };

        let before = fs::read_to_string(manager.config_path()).expect("read config before");
        let api = AppRuntimeApi::from_config(&manager, &defaults).expect("build api");
        let client = test_client();

        let runtime = api
            .start("lifecycle", client.clone())
            .await
            .expect("start runtime");
        assert_eq!(runtime.name, "lifecycle");
        assert_eq!(runtime.metadata, metadata);

        let env_values: HashMap<OsString, OsString> = runtime.config.env.iter().cloned().collect();
        assert_eq!(
            env_values.get(&OsString::from("CODEX_HOME")),
            Some(&code_home.as_os_str().to_os_string())
        );
        assert_eq!(
            env_values.get(&OsString::from("APP_RUNTIME_LIFECYCLE")),
            Some(&OsString::from("runtime-env"))
        );

        let thread = runtime
            .server
            .thread_start(ThreadStartParams {
                thread_id: None,
                metadata: serde_json::json!({"from": "lifecycle"}),
            })
            .await
            .expect("thread start");
        let thread_response = time::timeout(Duration::from_secs(2), thread.response)
            .await
            .expect("thread response timeout")
            .expect("recv thread response")
            .expect("thread response ok");
        let thread_id = thread_response
            .get("thread_id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        assert!(!thread_id.is_empty());

        runtime.stop().await.expect("shutdown runtime");

        let after = fs::read_to_string(manager.config_path()).expect("read config after");
        assert_eq!(before, after);

        let prepared = api.prepare("lifecycle").expect("prepare after stop");
        assert_eq!(prepared.metadata, metadata);
    }

    #[tokio::test]
    async fn app_runtime_api_not_found_errors() {
        let api = AppRuntimeApi::new(Vec::new());
        match api.prepare("missing") {
            Err(AppRuntimeError::NotFound(name)) => assert_eq!(name, "missing"),
            other => panic!("unexpected result: {other:?}"),
        }

        let client = test_client();
        match api.start("missing", client).await {
            Err(AppRuntimeError::NotFound(name)) => assert_eq!(name, "missing"),
            other => panic!("unexpected start result: {other:?}"),
        }
    }

    #[tokio::test]
    async fn app_runtime_pool_api_reuses_and_restarts_stdio() {
        let (config_dir, manager) = temp_config_manager();
        let (_server_dir, server_path) = write_fake_app_server();
        let code_home = config_dir.path().join("app-pool-home");

        let mut env_map = BTreeMap::new();
        env_map.insert("APP_POOL_ENV".into(), "runtime".into());

        let metadata = serde_json::json!({"resume_thread": "thread-pool"});
        manager
            .add_app_runtime(AddAppRuntimeRequest {
                name: "pooled".into(),
                definition: AppRuntimeDefinition {
                    description: Some("pooled app".into()),
                    tags: vec!["pool".into()],
                    env: env_map,
                    code_home: None,
                    current_dir: None,
                    mirror_stdio: Some(true),
                    startup_timeout_ms: Some(2000),
                    binary: None,
                    metadata: metadata.clone(),
                },
                overwrite: false,
            })
            .expect("add app runtime");

        let defaults = StdioServerConfig {
            binary: server_path.clone(),
            code_home: Some(code_home.clone()),
            current_dir: None,
            env: vec![
                (OsString::from("APP_POOL_ENV"), OsString::from("default")),
                (OsString::from("POOL_ONLY"), OsString::from("base")),
            ],
            mirror_stdio: false,
            startup_timeout: Duration::from_secs(3),
        };

        let before = fs::read_to_string(manager.config_path()).expect("read config before");
        let api = AppRuntimePoolApi::from_config(&manager, &defaults).expect("build pool api");
        let client = test_client();

        let available = api.available();
        assert_eq!(available.len(), 1);
        let pooled_summary = &available[0];
        assert_eq!(pooled_summary.name, "pooled");
        assert_eq!(pooled_summary.metadata, metadata);

        let launcher = api.launcher("pooled").expect("pooled launcher");
        assert_eq!(launcher.description.as_deref(), Some("pooled app"));
        assert_eq!(launcher.metadata, metadata);

        let launcher_config = launcher.config.clone();
        assert_eq!(launcher_config.binary, server_path);
        assert_eq!(
            launcher_config.code_home.as_deref(),
            Some(code_home.as_path())
        );
        assert_eq!(launcher_config.startup_timeout, Duration::from_secs(2));

        let launcher_env: HashMap<OsString, OsString> = launcher_config.env.into_iter().collect();
        assert_eq!(
            launcher_env.get(&OsString::from("CODEX_HOME")),
            Some(&code_home.as_os_str().to_os_string())
        );
        assert_eq!(
            launcher_env.get(&OsString::from("POOL_ONLY")),
            Some(&OsString::from("base"))
        );
        assert_eq!(
            launcher_env.get(&OsString::from("APP_POOL_ENV")),
            Some(&OsString::from("runtime"))
        );

        let stdio_config = api
            .stdio_config("pooled")
            .expect("pooled stdio config without starting");
        assert_eq!(stdio_config.binary, server_path);
        assert_eq!(stdio_config.code_home.as_deref(), Some(code_home.as_path()));
        let stdio_env: HashMap<OsString, OsString> = stdio_config.env.into_iter().collect();
        assert_eq!(
            stdio_env.get(&OsString::from("POOL_ONLY")),
            Some(&OsString::from("base"))
        );
        assert_eq!(
            stdio_env.get(&OsString::from("CODEX_HOME")),
            Some(&code_home.as_os_str().to_os_string())
        );
        assert_eq!(
            stdio_env.get(&OsString::from("APP_POOL_ENV")),
            Some(&OsString::from("runtime"))
        );

        assert!(api.running().await.is_empty());

        let runtime = api
            .start("pooled", client.clone())
            .await
            .expect("start pooled runtime");
        assert_eq!(runtime.name, "pooled");
        assert_eq!(runtime.metadata, metadata);

        let env_values: HashMap<OsString, OsString> = runtime.config.env.iter().cloned().collect();
        assert_eq!(
            env_values.get(&OsString::from("CODEX_HOME")),
            Some(&code_home.as_os_str().to_os_string())
        );
        assert_eq!(
            env_values.get(&OsString::from("POOL_ONLY")),
            Some(&OsString::from("base"))
        );
        assert_eq!(
            env_values.get(&OsString::from("APP_POOL_ENV")),
            Some(&OsString::from("runtime"))
        );

        let thread = runtime
            .server
            .thread_start(ThreadStartParams {
                thread_id: None,
                metadata: serde_json::json!({"from": "pool"}),
            })
            .await
            .expect("thread start");
        let response = time::timeout(Duration::from_secs(2), thread.response)
            .await
            .expect("thread response timeout")
            .expect("recv thread response")
            .expect("thread response ok");
        let thread_id = response
            .get("thread_id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        assert!(!thread_id.is_empty());

        let running = api.running().await;
        let running_summary = running
            .iter()
            .find(|summary| summary.name == "pooled")
            .expect("running summary present");
        assert_eq!(running_summary.metadata, metadata);

        let reused = api
            .start("pooled", client.clone())
            .await
            .expect("reuse pooled runtime");
        assert!(Arc::ptr_eq(&runtime, &reused));

        api.stop("pooled").await.expect("stop pooled runtime");
        match api.stop("pooled").await {
            Err(AppRuntimeError::NotFound(name)) => assert_eq!(name, "pooled"),
            other => panic!("expected not found on second stop, got {other:?}"),
        }

        assert!(api.running().await.is_empty());

        let restarted = api
            .start("pooled", client)
            .await
            .expect("restart pooled runtime");
        assert!(!Arc::ptr_eq(&runtime, &restarted));
        assert_eq!(restarted.metadata, metadata);

        let prepared = api.prepare("pooled").expect("prepare after restart");
        assert_eq!(prepared.metadata, metadata);

        let after = fs::read_to_string(manager.config_path()).expect("read config after");
        assert_eq!(before, after);
    }

    #[tokio::test]
    async fn app_runtime_pool_api_stop_all_shuts_down_runtimes() {
        let (config_dir, manager) = temp_config_manager();
        let (_server_dir, server_path) = write_fake_app_server();
        let code_home = config_dir.path().join("app-pool-stop-home");

        let alpha_metadata = serde_json::json!({"resume_thread": "alpha"});
        manager
            .add_app_runtime(AddAppRuntimeRequest {
                name: "alpha".into(),
                definition: AppRuntimeDefinition {
                    description: Some("alpha runtime".into()),
                    tags: vec!["pool".into()],
                    env: BTreeMap::new(),
                    code_home: None,
                    current_dir: None,
                    mirror_stdio: Some(false),
                    startup_timeout_ms: Some(2000),
                    binary: None,
                    metadata: alpha_metadata.clone(),
                },
                overwrite: false,
            })
            .expect("add alpha runtime");

        let beta_metadata = serde_json::json!({"resume_thread": "beta"});
        manager
            .add_app_runtime(AddAppRuntimeRequest {
                name: "beta".into(),
                definition: AppRuntimeDefinition {
                    description: Some("beta runtime".into()),
                    tags: vec!["pool".into()],
                    env: BTreeMap::new(),
                    code_home: None,
                    current_dir: None,
                    mirror_stdio: Some(false),
                    startup_timeout_ms: Some(2000),
                    binary: None,
                    metadata: beta_metadata.clone(),
                },
                overwrite: false,
            })
            .expect("add beta runtime");

        let defaults = StdioServerConfig {
            binary: server_path.clone(),
            code_home: Some(code_home.clone()),
            current_dir: None,
            env: Vec::new(),
            mirror_stdio: false,
            startup_timeout: Duration::from_secs(3),
        };

        let before = fs::read_to_string(manager.config_path()).expect("read config before");
        let api = AppRuntimePoolApi::from_config(&manager, &defaults).expect("build pool api");
        let client = test_client();

        assert!(api.running().await.is_empty());

        let alpha = api
            .start("alpha", client.clone())
            .await
            .expect("start alpha runtime");
        let beta = api
            .start("beta", client.clone())
            .await
            .expect("start beta runtime");

        assert_eq!(alpha.metadata, alpha_metadata);
        assert_eq!(beta.metadata, beta_metadata);

        let mut running = api.running().await;
        running.sort_by(|a, b| a.name.cmp(&b.name));
        assert_eq!(running.len(), 2);
        assert_eq!(running[0].name, "alpha");
        assert_eq!(running[0].metadata, alpha_metadata);
        assert_eq!(running[1].name, "beta");
        assert_eq!(running[1].metadata, beta_metadata);

        let alpha_thread = alpha
            .server
            .thread_start(ThreadStartParams {
                thread_id: None,
                metadata: serde_json::json!({"from": "alpha"}),
            })
            .await
            .expect("alpha thread start");
        let _ = time::timeout(Duration::from_secs(2), alpha_thread.response)
            .await
            .expect("alpha thread response timeout")
            .expect("alpha response recv")
            .expect("alpha ok");

        api.stop_all().await.expect("stop all runtimes");
        assert!(api.running().await.is_empty());

        let restarted_alpha = api
            .start("alpha", client.clone())
            .await
            .expect("restart alpha");
        assert!(!Arc::ptr_eq(&alpha, &restarted_alpha));
        assert_eq!(restarted_alpha.metadata, alpha_metadata);

        let restarted_beta = api.start("beta", client).await.expect("restart beta");
        assert!(!Arc::ptr_eq(&beta, &restarted_beta));
        assert_eq!(restarted_beta.metadata, beta_metadata);

        let prepared_alpha = api.prepare("alpha").expect("prepare alpha");
        assert_eq!(prepared_alpha.metadata, alpha_metadata);
        let prepared_beta = api.prepare("beta").expect("prepare beta");
        assert_eq!(prepared_beta.metadata, beta_metadata);

        let after = fs::read_to_string(manager.config_path()).expect("read config after");
        assert_eq!(before, after);
    }

    #[tokio::test]
    async fn runtime_manager_starts_and_stops_stdio() {
        let (_dir, script) = write_env_probe_server("MCP_RUNTIME_ENV_E8");
        let code_home = tempfile::tempdir().expect("code_home");

        let defaults = StdioServerConfig {
            binary: PathBuf::from("codex"),
            code_home: Some(code_home.path().to_path_buf()),
            current_dir: None,
            env: vec![(
                OsString::from("MCP_RUNTIME_ENV_E8"),
                OsString::from("manager-ok"),
            )],
            mirror_stdio: false,
            startup_timeout: Duration::from_secs(5),
        };

        let runtime = McpRuntimeServer {
            name: "env-probe".into(),
            transport: McpRuntimeTransport::Stdio(StdioServerDefinition {
                command: script.to_string_lossy().to_string(),
                args: Vec::new(),
                env: BTreeMap::new(),
                timeout_ms: Some(1500),
            }),
            description: None,
            tags: vec!["local".into()],
            tools: Some(McpToolConfig {
                enabled: vec!["tool-x".into()],
                disabled: vec![],
            }),
        };

        let launcher = runtime.into_launcher(&defaults);
        let manager = McpRuntimeManager::new(vec![launcher]);

        let mut handle = match manager.prepare("env-probe").expect("prepare stdio") {
            McpRuntimeHandle::Stdio(handle) => handle,
            other => panic!("expected stdio handle, got {other:?}"),
        };

        let mut reader = BufReader::new(handle.stdout_mut());
        let mut line = String::new();
        let _ = time::timeout(Duration::from_secs(2), reader.read_line(&mut line))
            .await
            .expect("read timeout")
            .expect("read env line");
        assert_eq!(line.trim(), "manager-ok");

        let tools = handle.tools().expect("tool hints");
        assert_eq!(tools.enabled, vec!["tool-x".to_string()]);

        handle.stop().await.expect("stop server");
    }

    #[test]
    fn runtime_manager_propagates_tool_hints_for_http() {
        let env_var = "MCP_HTTP_TOKEN_E8_HINTS";
        env::set_var(env_var, "token-hints");

        let mut http = StreamableHttpDefinition {
            url: "https://example.test/hints".into(),
            headers: BTreeMap::new(),
            bearer_env_var: Some(env_var.to_string()),
            connect_timeout_ms: Some(1200),
            request_timeout_ms: Some(2400),
        };
        http.headers.insert("X-Test".into(), "true".into());

        let runtime = McpRuntimeServer {
            name: "remote-http".into(),
            transport: McpRuntimeTransport::StreamableHttp(resolve_streamable_http(http)),
            description: Some("http runtime".into()),
            tags: vec!["http".into()],
            tools: Some(McpToolConfig {
                enabled: vec!["alpha".into()],
                disabled: vec!["beta".into()],
            }),
        };

        let defaults = StdioServerConfig {
            binary: PathBuf::from("codex"),
            code_home: None,
            current_dir: None,
            env: Vec::new(),
            mirror_stdio: false,
            startup_timeout: Duration::from_secs(2),
        };

        let launcher = runtime.into_launcher(&defaults);
        let manager = McpRuntimeManager::new(vec![launcher]);

        let available = manager.available();
        assert_eq!(available.len(), 1);
        let summary = &available[0];
        assert_eq!(summary.name, "remote-http");
        assert_eq!(
            summary.transport,
            McpRuntimeSummaryTransport::StreamableHttp
        );
        let summary_tools = summary.tools.as_ref().expect("tool hints present");
        assert_eq!(summary_tools.enabled, vec!["alpha".to_string()]);
        assert_eq!(summary_tools.disabled, vec!["beta".to_string()]);

        match manager.prepare("remote-http").expect("prepare http") {
            McpRuntimeHandle::StreamableHttp(http_handle) => {
                let tools = http_handle.tools.as_ref().expect("tool hints on handle");
                assert_eq!(tools.enabled, vec!["alpha".to_string()]);
                assert_eq!(tools.disabled, vec!["beta".to_string()]);
                assert_eq!(
                    http_handle.connector.bearer_token.as_deref(),
                    Some("token-hints")
                );
            }
            other => panic!("expected http handle, got {other:?}"),
        }

        env::remove_var(env_var);
    }

    #[test]
    fn http_connector_retrieval_is_non_destructive() {
        let env_var = "MCP_HTTP_TOKEN_E8_REUSE";
        env::set_var(env_var, "token-reuse");

        let runtime = McpRuntimeServer {
            name: "remote-reuse".into(),
            transport: McpRuntimeTransport::StreamableHttp(resolve_streamable_http(
                StreamableHttpDefinition {
                    url: "https://example.test/reuse".into(),
                    headers: BTreeMap::new(),
                    bearer_env_var: Some(env_var.to_string()),
                    connect_timeout_ms: Some(1500),
                    request_timeout_ms: Some(3200),
                },
            )),
            description: None,
            tags: vec!["http".into()],
            tools: Some(McpToolConfig {
                enabled: vec!["one".into()],
                disabled: vec![],
            }),
        };

        let defaults = StdioServerConfig {
            binary: PathBuf::from("codex"),
            code_home: None,
            current_dir: None,
            env: Vec::new(),
            mirror_stdio: false,
            startup_timeout: Duration::from_secs(2),
        };

        let launcher = runtime.into_launcher(&defaults);
        let manager = McpRuntimeManager::new(vec![launcher]);

        let first = manager.prepare("remote-reuse").expect("first prepare");
        let second = manager.prepare("remote-reuse").expect("second prepare");

        let first_token = match first {
            McpRuntimeHandle::StreamableHttp(handle) => handle.connector.bearer_token,
            other => panic!("expected http handle, got {other:?}"),
        };
        let second_token = match second {
            McpRuntimeHandle::StreamableHttp(handle) => handle.connector.bearer_token,
            other => panic!("expected http handle, got {other:?}"),
        };

        assert_eq!(first_token.as_deref(), Some("token-reuse"));
        assert_eq!(second_token.as_deref(), Some("token-reuse"));

        let summary = manager
            .available()
            .into_iter()
            .find(|s| s.name == "remote-reuse")
            .expect("summary present");
        assert_eq!(
            summary.transport,
            McpRuntimeSummaryTransport::StreamableHttp
        );
        let tools = summary.tools.as_ref().expect("tool hints preserved");
        assert_eq!(tools.enabled, vec!["one".to_string()]);

        env::remove_var(env_var);
    }
}
