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
    env,
    ffi::OsString,
    fs, io,
    path::{Path, PathBuf},
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

/// Default config filename placed under CODEX_HOME.
pub const DEFAULT_CONFIG_FILE: &str = "config.toml";
const MCP_SERVERS_KEY: &str = "mcp_servers";

/// MCP server definition coupled with its name.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpServerEntry {
    pub name: String,
    pub definition: McpServerDefinition,
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

/// Helper to load and mutate MCP config stored under `[mcp_servers]`.
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
        self.runtime_servers()
            .map(|servers| {
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

/// Client wrapper around the stdio app-server.
pub struct CodexAppServer {
    transport: Arc<JsonRpcTransport>,
}

impl CodexAppServer {
    /// Launch `codex app-server`, issue `initialize`, and return a connected handle.
    pub async fn start(config: StdioServerConfig, client: ClientInfo) -> Result<Self, McpError> {
        Self::with_capabilities(config, client, Value::Null).await
    }

    /// Launch with explicit capabilities to send during `initialize`.
    pub async fn with_capabilities(
        config: StdioServerConfig,
        client: ClientInfo,
        capabilities: Value,
    ) -> Result<Self, McpError> {
        let transport = JsonRpcTransport::spawn_app(config).await?;
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

/// Internal transport that handles stdio JSON-RPC.
struct JsonRpcTransport {
    writer: mpsc::UnboundedSender<String>,
    pending: Arc<Mutex<HashMap<RequestId, oneshot::Sender<Result<Value, McpError>>>>>,
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
    pending: Arc<Mutex<HashMap<RequestId, oneshot::Sender<Result<Value, McpError>>>>>,
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
        env, fs,
        ffi::OsString,
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
    conversation_id = params.get("conversation_id") or f"conv-{req_id}"
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
    elif method == "codex/codex" or method == "codex/codex-reply":
        handle_codex(msg.get("id"), msg.get("params", {}))
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
    thread_id = params.get("thread_id") or "thread-unknown"
    turn_id = params.get("turn_id") or f"turn-{req_id}"
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
        thread_id = params.get("thread_id")
        send({"jsonrpc": "2.0", "id": msg.get("id"), "result": {"thread_id": thread_id, "resumed": True}})
    elif method == "turn/start":
        handle_turn(msg.get("id"), msg.get("params", {}))
    elif method == "turn/interrupt":
        params = msg.get("params", {})
        turn_id = params.get("turn_id")
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
        assert!(servers.get("one").is_none());
        assert!(servers.get("two").is_some());
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
                assert_eq!(
                    launch.current_dir.as_ref(),
                    defaults.current_dir.as_ref()
                );
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
        definition
            .headers
            .insert("X-Test".into(), "true".into());

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
                assert_eq!(
                    connector.connect_timeout,
                    Some(Duration::from_millis(1200))
                );
                assert_eq!(
                    connector.request_timeout,
                    Some(Duration::from_millis(3400))
                );
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
        assert_eq!(response.conversation_id, event_conversation);
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
            config: BTreeMap::new(),
        };
        let first = server.codex(params).await.expect("start codex");
        let first_response = time::timeout(Duration::from_secs(2), first.response)
            .await
            .expect("response timeout")
            .expect("recv")
            .expect("ok");
        let conversation_id = first_response.conversation_id;
        assert!(!conversation_id.is_empty());

        let reply_params = CodexReplyParams {
            conversation_id: conversation_id.clone(),
            prompt: "follow up".into(),
            model: None,
            cwd: None,
            sandbox: None,
            approval_policy: None,
            config: BTreeMap::new(),
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
        assert_eq!(reply_response.conversation_id, conversation_id);
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
            prompt: "hi".into(),
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
            prompt: "cancel me".into(),
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
        assert_eq!(
            resume_response
                .get("resumed")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            true
        );

        let params = TurnStartParams {
            thread_id: thread_id.clone(),
            prompt: "resume flow".into(),
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
            prompt: "please interrupt".into(),
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
        assert_eq!(
            interrupt_response
                .get("interrupted")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            true
        );

        let _ = server.shutdown().await;
    }
}
