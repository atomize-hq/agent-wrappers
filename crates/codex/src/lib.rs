//! Minimal async wrapper over the [OpenAI Codex CLI](https://github.com/openai/codex).
//!
//! The CLI ships both an interactive TUI (`codex`) and a headless automation mode (`codex exec`).
//! This crate targets the latter: it shells out to `codex exec`, enforces sensible defaults
//! (non-interactive color handling, timeouts, optional model selection), and returns whatever
//! the CLI prints to stdout (the agent's final response per upstream docs).

use std::{
    collections::BTreeMap,
    env,
    ffi::OsStr,
    future::Future,
    io::{self as stdio, Write},
    path::{Path, PathBuf},
    pin::Pin,
    process::ExitStatus,
    time::Duration,
};

use futures_core::Stream;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tempfile::TempDir;
use thiserror::Error;
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWriteExt},
    process::Command,
    task, time,
};
use tracing::debug;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(120);
const DEFAULT_REASONING_CONFIG_GPT5: &[(&str, &str)] = &[
    ("model_reasoning_effort", "minimal"),
    ("model_reasoning_summary", "auto"),
    ("model_verbosity", "low"),
];

const DEFAULT_REASONING_CONFIG_GPT5_CODEX: &[(&str, &str)] = &[
    ("model_reasoning_effort", "low"),
    ("model_reasoning_summary", "auto"),
    ("model_verbosity", "low"),
];
const CODEX_BINARY_ENV: &str = "CODEX_BINARY";

/// High-level client for interacting with `codex exec`.
#[derive(Clone, Debug)]
pub struct CodexClient {
    binary: PathBuf,
    model: Option<String>,
    timeout: Duration,
    color_mode: ColorMode,
    working_dir: Option<PathBuf>,
    images: Vec<PathBuf>,
    json_output: bool,
    quiet: bool,
    mirror_stdout: bool,
}

/// Current authentication state reported by `codex login status`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CodexAuthStatus {
    /// The CLI reports an active session.
    LoggedIn(CodexAuthMethod),
    /// No credentials stored locally.
    LoggedOut,
}

/// Authentication mechanism used to sign in.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CodexAuthMethod {
    ChatGpt,
    ApiKey { masked_key: Option<String> },
}

/// Result of invoking `codex logout`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CodexLogoutStatus {
    LoggedOut,
    AlreadyLoggedOut,
}

impl CodexClient {
    /// Returns a [`CodexClientBuilder`] preloaded with safe defaults.
    pub fn builder() -> CodexClientBuilder {
        CodexClientBuilder::default()
    }

    /// Sends `prompt` to `codex exec` and returns its stdout (the final agent message) on success.
    pub async fn send_prompt(&self, prompt: impl AsRef<str>) -> Result<String, CodexError> {
        let prompt = prompt.as_ref();
        if prompt.trim().is_empty() {
            return Err(CodexError::EmptyPrompt);
        }

        self.invoke_codex_exec(prompt).await
    }

    /// Spawns a `codex login` session using the default ChatGPT OAuth flow.
    ///
    /// The returned child inherits `kill_on_drop` so abandoning the handle cleans up the login helper.
    pub fn spawn_login_process(&self) -> Result<tokio::process::Child, CodexError> {
        let mut command = Command::new(&self.binary);
        command
            .arg("login")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);

        if env::var_os("RUST_LOG").is_none() {
            command.env("RUST_LOG", "error");
        }

        command.spawn().map_err(|source| CodexError::Spawn {
            binary: self.binary.clone(),
            source,
        })
    }

    /// Returns the current Codex authentication state by invoking `codex login status`.
    pub async fn login_status(&self) -> Result<CodexAuthStatus, CodexError> {
        let output = self.run_basic_command(["login", "status"]).await?;
        let stderr = String::from_utf8(output.stderr.clone()).unwrap_or_default();
        let stdout = String::from_utf8(output.stdout.clone()).unwrap_or_default();
        let combined = if stderr.trim().is_empty() {
            stdout
        } else {
            stderr
        };

        if output.status.success() {
            parse_login_success(&combined).ok_or_else(|| CodexError::NonZeroExit {
                status: output.status,
                stderr: combined,
            })
        } else if combined.to_lowercase().contains("not logged in") {
            Ok(CodexAuthStatus::LoggedOut)
        } else {
            Err(CodexError::NonZeroExit {
                status: output.status,
                stderr: combined,
            })
        }
    }

    /// Removes cached credentials via `codex logout`.
    pub async fn logout(&self) -> Result<CodexLogoutStatus, CodexError> {
        let output = self.run_basic_command(["logout"]).await?;
        let stderr = String::from_utf8(output.stderr).unwrap_or_default();
        let stdout = String::from_utf8(output.stdout).unwrap_or_default();
        let combined = if stderr.trim().is_empty() {
            stdout
        } else {
            stderr
        };

        if !output.status.success() {
            return Err(CodexError::NonZeroExit {
                status: output.status,
                stderr: combined,
            });
        }

        let normalized = combined.to_lowercase();
        if normalized.contains("successfully logged out") {
            Ok(CodexLogoutStatus::LoggedOut)
        } else if normalized.contains("not logged in") {
            Ok(CodexLogoutStatus::AlreadyLoggedOut)
        } else {
            Ok(CodexLogoutStatus::LoggedOut)
        }
    }

    async fn invoke_codex_exec(&self, prompt: &str) -> Result<String, CodexError> {
        let dir_ctx = self.directory_context()?;

        let mut command = Command::new(&self.binary);
        command
            .arg("exec")
            .arg("--color")
            .arg(self.color_mode.as_str())
            .arg("--skip-git-repo-check")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .current_dir(dir_ctx.path());

        let send_prompt_via_stdin = self.json_output;
        if !send_prompt_via_stdin {
            command.arg(prompt);
        }
        let stdin_mode = if send_prompt_via_stdin {
            std::process::Stdio::piped()
        } else {
            std::process::Stdio::null()
        };
        command.stdin(stdin_mode);

        if let Some(config) = reasoning_config_for(self.model.as_deref()) {
            for (key, value) in config {
                command.arg("--config").arg(format!("{key}={value}"));
            }
        }

        if let Some(model) = &self.model {
            command.arg("--model").arg(model);
        }

        for image in &self.images {
            command.arg("--image").arg(image);
        }

        if self.json_output {
            command.arg("--json");
        }

        if env::var_os("RUST_LOG").is_none() {
            command.env("RUST_LOG", "error");
        }

        let mut child = command.spawn().map_err(|source| CodexError::Spawn {
            binary: self.binary.clone(),
            source,
        })?;

        if send_prompt_via_stdin {
            let mut stdin = child.stdin.take().ok_or(CodexError::StdinUnavailable)?;
            stdin
                .write_all(prompt.as_bytes())
                .await
                .map_err(CodexError::StdinWrite)?;
            stdin
                .write_all(b"\n")
                .await
                .map_err(CodexError::StdinWrite)?;
            stdin.shutdown().await.map_err(CodexError::StdinWrite)?;
        } else {
            let _ = child.stdin.take();
        }

        let stdout = child.stdout.take().ok_or(CodexError::StdoutUnavailable)?;
        let stderr = child.stderr.take().ok_or(CodexError::StderrUnavailable)?;

        let stdout_task = tokio::spawn(tee_stream(
            stdout,
            ConsoleTarget::Stdout,
            self.mirror_stdout,
        ));
        let stderr_task = tokio::spawn(tee_stream(stderr, ConsoleTarget::Stderr, !self.quiet));

        let wait_task = async move {
            let status = child
                .wait()
                .await
                .map_err(|source| CodexError::Wait { source })?;
            let stdout_bytes = stdout_task
                .await
                .map_err(CodexError::Join)?
                .map_err(CodexError::CaptureIo)?;
            let stderr_bytes = stderr_task
                .await
                .map_err(CodexError::Join)?
                .map_err(CodexError::CaptureIo)?;
            Ok::<_, CodexError>((status, stdout_bytes, stderr_bytes))
        };

        let (status, stdout_bytes, stderr_bytes) = if self.timeout.is_zero() {
            wait_task.await?
        } else {
            match time::timeout(self.timeout, wait_task).await {
                Ok(result) => result?,
                Err(_) => {
                    return Err(CodexError::Timeout {
                        timeout: self.timeout,
                    });
                }
            }
        };

        let stderr_string = String::from_utf8(stderr_bytes).unwrap_or_default();
        if !status.success() {
            return Err(CodexError::NonZeroExit {
                status,
                stderr: stderr_string,
            });
        }

        let primary_output = if self.json_output && stdout_bytes.is_empty() {
            stderr_string
        } else {
            String::from_utf8(stdout_bytes)?
        };
        let trimmed = if self.json_output {
            primary_output
        } else {
            primary_output.trim().to_string()
        };
        debug!(binary = ?self.binary, bytes = trimmed.len(), "received Codex output");
        Ok(trimmed)
    }

    fn directory_context(&self) -> Result<DirectoryContext, CodexError> {
        if let Some(dir) = &self.working_dir {
            return Ok(DirectoryContext::Fixed(dir.clone()));
        }

        let temp = tempfile::tempdir().map_err(CodexError::TempDir)?;
        Ok(DirectoryContext::Ephemeral(temp))
    }

    async fn run_basic_command<S, I>(&self, args: I) -> Result<CommandOutput, CodexError>
    where
        S: AsRef<OsStr>,
        I: IntoIterator<Item = S>,
    {
        let mut command = Command::new(&self.binary);
        command
            .args(args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);

        if env::var_os("RUST_LOG").is_none() {
            command.env("RUST_LOG", "error");
        }

        let mut child = command.spawn().map_err(|source| CodexError::Spawn {
            binary: self.binary.clone(),
            source,
        })?;

        let stdout = child.stdout.take().ok_or(CodexError::StdoutUnavailable)?;
        let stderr = child.stderr.take().ok_or(CodexError::StderrUnavailable)?;

        let stdout_task = tokio::spawn(tee_stream(stdout, ConsoleTarget::Stdout, false));
        let stderr_task = tokio::spawn(tee_stream(stderr, ConsoleTarget::Stderr, false));

        let wait_task = async move {
            let status = child
                .wait()
                .await
                .map_err(|source| CodexError::Wait { source })?;
            let stdout_bytes = stdout_task
                .await
                .map_err(CodexError::Join)?
                .map_err(CodexError::CaptureIo)?;
            let stderr_bytes = stderr_task
                .await
                .map_err(CodexError::Join)?
                .map_err(CodexError::CaptureIo)?;
            Ok::<_, CodexError>((status, stdout_bytes, stderr_bytes))
        };

        let (status, stdout_bytes, stderr_bytes) = if self.timeout.is_zero() {
            wait_task.await?
        } else {
            match time::timeout(self.timeout, wait_task).await {
                Ok(result) => result?,
                Err(_) => {
                    return Err(CodexError::Timeout {
                        timeout: self.timeout,
                    });
                }
            }
        };

        Ok(CommandOutput {
            status,
            stdout: stdout_bytes,
            stderr: stderr_bytes,
        })
    }
}

impl Default for CodexClient {
    fn default() -> Self {
        CodexClient::builder().build()
    }
}

/// Builder for [`CodexClient`].
#[derive(Clone, Debug)]
pub struct CodexClientBuilder {
    binary: PathBuf,
    model: Option<String>,
    timeout: Duration,
    color_mode: ColorMode,
    working_dir: Option<PathBuf>,
    images: Vec<PathBuf>,
    json_output: bool,
    quiet: bool,
    mirror_stdout: bool,
}

impl CodexClientBuilder {
    /// Starts a new builder with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the path to the Codex binary. Defaults to `codex`.
    pub fn binary(mut self, binary: impl Into<PathBuf>) -> Self {
        self.binary = binary.into();
        self
    }

    /// Sets the model that should be used for every `codex exec` call.
    pub fn model(mut self, model: impl Into<String>) -> Self {
        let model = model.into();
        self.model = (!model.trim().is_empty()).then_some(model);
        self
    }

    /// Overrides the maximum amount of time to wait for Codex to respond.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Controls whether Codex may emit ANSI colors (`--color`). Defaults to [`ColorMode::Never`].
    pub fn color_mode(mut self, color_mode: ColorMode) -> Self {
        self.color_mode = color_mode;
        self
    }

    /// Forces Codex to run with the provided working directory instead of a fresh temp dir.
    pub fn working_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.working_dir = Some(dir.into());
        self
    }

    /// Adds an image to the prompt by passing `--image <path>` to `codex exec`.
    pub fn image(mut self, path: impl Into<PathBuf>) -> Self {
        self.images.push(path.into());
        self
    }

    /// Replaces the current image list with the provided collection.
    pub fn images<I, P>(mut self, images: I) -> Self
    where
        I: IntoIterator<Item = P>,
        P: Into<PathBuf>,
    {
        self.images = images.into_iter().map(Into::into).collect();
        self
    }

    /// Enables Codex's JSONL output mode (`--json`).
    pub fn json(mut self, enable: bool) -> Self {
        self.json_output = enable;
        self
    }

    /// Suppresses mirroring Codex stderr to the console.
    pub fn quiet(mut self, enable: bool) -> Self {
        self.quiet = enable;
        self
    }

    /// Controls whether Codex stdout should be mirrored to the console while
    /// also being captured. Disable this when you plan to parse JSONL output.
    pub fn mirror_stdout(mut self, enable: bool) -> Self {
        self.mirror_stdout = enable;
        self
    }

    /// Builds the [`CodexClient`].
    pub fn build(self) -> CodexClient {
        CodexClient {
            binary: self.binary,
            model: self.model,
            timeout: self.timeout,
            color_mode: self.color_mode,
            working_dir: self.working_dir,
            images: self.images,
            json_output: self.json_output,
            quiet: self.quiet,
            mirror_stdout: self.mirror_stdout,
        }
    }
}

impl Default for CodexClientBuilder {
    fn default() -> Self {
        Self {
            binary: default_binary_path(),
            model: None,
            timeout: DEFAULT_TIMEOUT,
            color_mode: ColorMode::Never,
            working_dir: None,
            images: Vec::new(),
            json_output: false,
            quiet: false,
            mirror_stdout: true,
        }
    }
}

/// ANSI color behavior for `codex exec` output.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ColorMode {
    /// Match upstream defaults: use color codes when stdout/stderr look like terminals.
    Auto,
    /// Force colorful output even when piping.
    Always,
    /// Fully disable ANSI sequences for deterministic parsing/logging (default).
    Never,
}

impl ColorMode {
    const fn as_str(self) -> &'static str {
        match self {
            ColorMode::Auto => "auto",
            ColorMode::Always => "always",
            ColorMode::Never => "never",
        }
    }
}

fn reasoning_config_for(model: Option<&str>) -> Option<&'static [(&'static str, &'static str)]> {
    match model {
        Some(name) if name.eq_ignore_ascii_case("gpt-5-codex") => {
            Some(DEFAULT_REASONING_CONFIG_GPT5_CODEX)
        }
        _ => Some(DEFAULT_REASONING_CONFIG_GPT5),
    }
}

/// Errors that may occur while invoking the Codex CLI.
#[derive(Debug, Error)]
pub enum CodexError {
    #[error("codex binary `{binary}` could not be spawned: {source}")]
    Spawn {
        binary: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to wait for codex process: {source}")]
    Wait {
        #[source]
        source: std::io::Error,
    },
    #[error("codex exceeded timeout of {timeout:?}")]
    Timeout { timeout: Duration },
    #[error("codex exited with {status:?}: {stderr}")]
    NonZeroExit { status: ExitStatus, stderr: String },
    #[error("codex output was not valid UTF-8: {0}")]
    InvalidUtf8(#[from] std::string::FromUtf8Error),
    #[error("prompt must not be empty")]
    EmptyPrompt,
    #[error("failed to create temporary working directory: {0}")]
    TempDir(#[source] std::io::Error),
    #[error("codex stdout unavailable")]
    StdoutUnavailable,
    #[error("codex stderr unavailable")]
    StderrUnavailable,
    #[error("codex stdin unavailable")]
    StdinUnavailable,
    #[error("failed to capture codex output: {0}")]
    CaptureIo(#[from] std::io::Error),
    #[error("failed to write prompt to codex stdin: {0}")]
    StdinWrite(#[source] std::io::Error),
    #[error("failed to join codex output task: {0}")]
    Join(#[from] tokio::task::JoinError),
}

/// Single JSONL event emitted by `codex exec --json`.
///
/// Each line on stdout maps to a [`ThreadEvent`] with lifecycle edges:
/// - `thread.started` is emitted once per invocation.
/// - `turn.started` begins the turn associated with the provided prompt.
/// - one or more `item.*` events stream output and tool activity.
/// - `turn.completed` or `turn.failed` closes the stream; `error` captures transport-level failures.
///
/// Item variants mirror the upstream `item_type` field: `agent_message`, `reasoning`,
/// `command_execution`, `file_change`, `mcp_tool_call`, `web_search`, `todo_list`, and `error`.
/// Unknown or future fields are preserved in `extra` maps to keep the parser forward-compatible.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum ThreadEvent {
    #[serde(rename = "thread.started")]
    ThreadStarted(ThreadStarted),
    #[serde(rename = "turn.started")]
    TurnStarted(TurnStarted),
    #[serde(rename = "turn.completed")]
    TurnCompleted(TurnCompleted),
    #[serde(rename = "turn.failed")]
    TurnFailed(TurnFailed),
    #[serde(rename = "item.started")]
    ItemStarted(ItemEnvelope<ItemSnapshot>),
    #[serde(rename = "item.delta")]
    ItemDelta(ItemDelta),
    #[serde(rename = "item.completed")]
    ItemCompleted(ItemEnvelope<ItemSnapshot>),
    #[serde(rename = "item.failed")]
    ItemFailed(ItemEnvelope<ItemFailure>),
    #[serde(rename = "error")]
    Error(EventError),
}

/// Marks the start of a new thread.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ThreadStarted {
    pub thread_id: String,
    #[serde(flatten, default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, Value>,
}

/// Indicates the CLI accepted a new turn within a thread.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TurnStarted {
    pub thread_id: String,
    pub turn_id: String,
    /// Original input text when upstream echoes it; may be omitted for security reasons.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_text: Option<String>,
    #[serde(flatten, default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, Value>,
}

/// Reports a completed turn.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TurnCompleted {
    pub thread_id: String,
    pub turn_id: String,
    /// Identifier of the last output item when provided by the CLI.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_item_id: Option<String>,
    #[serde(flatten, default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, Value>,
}

/// Indicates a turn-level failure.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TurnFailed {
    pub thread_id: String,
    pub turn_id: String,
    pub error: EventError,
    #[serde(flatten, default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, Value>,
}

/// Shared wrapper for item events that always include thread/turn context.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ItemEnvelope<T> {
    pub thread_id: String,
    pub turn_id: String,
    #[serde(flatten)]
    pub item: T,
}

/// Snapshot of an item at start/completion time.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ItemSnapshot {
    #[serde(rename = "item_id", alias = "id")]
    pub item_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index: Option<u32>,
    #[serde(default)]
    pub status: ItemStatus,
    #[serde(flatten)]
    pub payload: ItemPayload,
    #[serde(flatten, default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, Value>,
}

/// Streaming delta describing the next piece of an item.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ItemDelta {
    pub thread_id: String,
    pub turn_id: String,
    #[serde(rename = "item_id")]
    pub item_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index: Option<u32>,
    #[serde(flatten)]
    pub delta: ItemDeltaPayload,
    #[serde(flatten, default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, Value>,
}

/// Terminal item failure event.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ItemFailure {
    #[serde(rename = "item_id", alias = "id")]
    pub item_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index: Option<u32>,
    pub error: EventError,
    #[serde(flatten, default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, Value>,
}

/// Fully-typed item payload for start/completed events.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "item_type", content = "content", rename_all = "snake_case")]
pub enum ItemPayload {
    AgentMessage(TextContent),
    Reasoning(TextContent),
    CommandExecution(CommandExecutionState),
    FileChange(FileChangeState),
    McpToolCall(McpToolCallState),
    WebSearch(WebSearchState),
    TodoList(TodoListState),
    Error(EventError),
}

/// Delta form of an item payload. Each delta should be applied in order to reconstruct the item.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "item_type", content = "delta", rename_all = "snake_case")]
pub enum ItemDeltaPayload {
    AgentMessage(TextDelta),
    Reasoning(TextDelta),
    CommandExecution(CommandExecutionDelta),
    FileChange(FileChangeDelta),
    McpToolCall(McpToolCallDelta),
    WebSearch(WebSearchDelta),
    TodoList(TodoListDelta),
    Error(EventError),
}

/// Item status supplied by the CLI for bookkeeping.
#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ItemStatus {
    InProgress,
    Completed,
    Failed,
    #[serde(other)]
    Unknown,
}

impl Default for ItemStatus {
    fn default() -> Self {
        ItemStatus::InProgress
    }
}

/// Human-readable content emitted by the agent.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TextContent {
    pub text: String,
}

/// Incremental content fragment for streaming items.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TextDelta {
    pub text_delta: String,
}

/// Snapshot of a command execution, including accumulated stdout/stderr.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CommandExecutionState {
    pub command: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub stdout: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub stderr: String,
    #[serde(flatten, default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, Value>,
}

/// Streaming delta for command execution.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CommandExecutionDelta {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub stdout: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub stderr: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    #[serde(flatten, default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, Value>,
}

/// File change or diff applied by the agent.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FileChangeState {
    pub path: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub change: Option<FileChangeKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diff: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub stdout: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub stderr: String,
    #[serde(flatten, default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, Value>,
}

/// Streaming delta describing a file change.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FileChangeDelta {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diff: Option<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub stdout: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub stderr: String,
    #[serde(flatten, default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, Value>,
}

/// Type of file operation being reported.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FileChangeKind {
    Apply,
    Diff,
    #[serde(other)]
    Unknown,
}

/// State of an MCP tool call.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct McpToolCallState {
    pub server_name: String,
    pub tool_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(default)]
    pub status: ToolCallStatus,
    #[serde(flatten, default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, Value>,
}

/// Streaming delta for MCP tool call output.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct McpToolCallDelta {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(default)]
    pub status: ToolCallStatus,
    #[serde(flatten, default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, Value>,
}

/// Lifecycle state for a tool call.
#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ToolCallStatus {
    Pending,
    Running,
    Completed,
    Failed,
    #[serde(other)]
    Unknown,
}

impl Default for ToolCallStatus {
    fn default() -> Self {
        ToolCallStatus::Pending
    }
}

/// Details of a web search step.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WebSearchState {
    pub query: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub results: Option<Value>,
    #[serde(default)]
    pub status: WebSearchStatus,
    #[serde(flatten, default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, Value>,
}

/// Streaming delta for search results.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WebSearchDelta {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub results: Option<Value>,
    #[serde(default)]
    pub status: WebSearchStatus,
    #[serde(flatten, default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, Value>,
}

/// Search progress indicator.
#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum WebSearchStatus {
    Pending,
    Running,
    Completed,
    Failed,
    #[serde(other)]
    Unknown,
}

impl Default for WebSearchStatus {
    fn default() -> Self {
        WebSearchStatus::Pending
    }
}

/// Checklist maintained by the agent.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TodoListState {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<TodoItem>,
    #[serde(flatten, default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, Value>,
}

/// Streaming delta for todo list mutations.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TodoListDelta {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<TodoItem>,
    #[serde(flatten, default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, Value>,
}

/// Single todo item.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TodoItem {
    pub title: String,
    #[serde(default)]
    pub completed: bool,
    #[serde(flatten, default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, Value>,
}

/// Error payload shared by turn/item failures.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EventError {
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(flatten, default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, Value>,
}

/// Options configuring a streaming exec invocation.
#[derive(Clone, Debug)]
pub struct ExecStreamRequest {
    /// User prompt that will be forwarded to `codex exec`.
    pub prompt: String,
    /// Per-event idle timeout. If no JSON lines arrive before the duration elapses,
    /// [`ExecStreamError::IdleTimeout`] is returned.
    pub idle_timeout: Option<Duration>,
    /// Optional file path passed through to `--output-last-message`. When unset, the wrapper
    /// will request a temporary path and return it in [`ExecCompletion::last_message_path`].
    pub output_last_message: Option<PathBuf>,
    /// Optional file path passed through to `--output-schema` so clients can persist the schema
    /// describing the item envelope structure seen during the run.
    pub output_schema: Option<PathBuf>,
}

/// Ergonomic container for the streaming surface; produced by `stream_exec` (implemented in D2).
///
/// `events` yields parsed [`ThreadEvent`] values as soon as each JSONL line arrives from the CLI.
/// `completion` resolves once the Codex process exits and is the place to surface `--output-last-message`
/// and `--output-schema` paths after streaming finishes.
pub struct ExecStream {
    pub events: DynThreadEventStream,
    pub completion: DynExecCompletion,
}

/// Type-erased stream of events from the Codex CLI.
pub type DynThreadEventStream =
    Pin<Box<dyn Stream<Item = Result<ThreadEvent, ExecStreamError>> + Send>>;

/// Type-erased completion future that resolves when streaming stops.
pub type DynExecCompletion =
    Pin<Box<dyn Future<Output = Result<ExecCompletion, ExecStreamError>> + Send>>;

/// Summary returned when the codex child process exits.
#[derive(Clone, Debug)]
pub struct ExecCompletion {
    pub status: ExitStatus,
    /// Path that codex wrote when `--output-last-message` was enabled. The wrapper may eagerly
    /// read the file and populate `last_message` when feasible.
    pub last_message_path: Option<PathBuf>,
    pub last_message: Option<String>,
    /// Path to the JSON schema requested via `--output-schema`, if provided by the caller.
    pub schema_path: Option<PathBuf>,
}

/// Errors that may occur while consuming the JSONL stream.
#[derive(Debug, Error)]
pub enum ExecStreamError {
    #[error(transparent)]
    Codex(#[from] CodexError),
    #[error("failed to parse codex JSONL event: {source}: `{line}`")]
    Parse {
        line: String,
        #[source]
        source: serde_json::Error,
    },
    #[error("codex JSON stream idle for {idle_for:?}")]
    IdleTimeout { idle_for: Duration },
    #[error("codex JSON stream closed unexpectedly")]
    ChannelClosed,
}

enum DirectoryContext {
    Fixed(PathBuf),
    Ephemeral(TempDir),
}

impl DirectoryContext {
    fn path(&self) -> &Path {
        match self {
            DirectoryContext::Fixed(path) => path.as_path(),
            DirectoryContext::Ephemeral(dir) => dir.path(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    fn env_guard() -> std::sync::MutexGuard<'static, ()> {
        static ENV_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();
        ENV_MUTEX.get_or_init(|| Mutex::new(())).lock().unwrap()
    }

    #[test]
    fn builder_defaults_are_sane() {
        let builder = CodexClient::builder();
        assert!(builder.model.is_none());
        assert_eq!(builder.timeout, DEFAULT_TIMEOUT);
        assert_eq!(builder.color_mode, ColorMode::Never);
        assert!(builder.working_dir.is_none());
        assert!(builder.images.is_empty());
        assert!(!builder.json_output);
        assert!(!builder.quiet);
    }

    #[test]
    fn builder_collects_images() {
        let client = CodexClient::builder()
            .image("foo.png")
            .image("bar.jpg")
            .build();
        assert_eq!(client.images.len(), 2);
        assert_eq!(client.images[0], PathBuf::from("foo.png"));
        assert_eq!(client.images[1], PathBuf::from("bar.jpg"));
    }

    #[test]
    fn builder_sets_json_flag() {
        let client = CodexClient::builder().json(true).build();
        assert!(client.json_output);
    }

    #[test]
    fn builder_sets_quiet_flag() {
        let client = CodexClient::builder().quiet(true).build();
        assert!(client.quiet);
    }

    #[test]
    fn builder_mirrors_stdout_by_default() {
        let client = CodexClient::builder().build();
        assert!(client.mirror_stdout);
    }

    #[test]
    fn builder_can_disable_stdout_mirroring() {
        let client = CodexClient::builder().mirror_stdout(false).build();
        assert!(!client.mirror_stdout);
    }

    #[test]
    fn builder_uses_env_binary_when_set() {
        let _guard = env_guard();
        let key = CODEX_BINARY_ENV;
        let original = env::var_os(key);
        env::set_var(key, "custom_codex");
        let builder = CodexClient::builder();
        assert_eq!(builder.binary, PathBuf::from("custom_codex"));
        if let Some(value) = original {
            env::set_var(key, value);
        } else {
            env::remove_var(key);
        }
    }

    #[test]
    fn reasoning_config_by_model() {
        assert_eq!(
            reasoning_config_for(Some("gpt-5")).unwrap(),
            DEFAULT_REASONING_CONFIG_GPT5
        );
        assert_eq!(
            reasoning_config_for(Some("gpt-5-codex")).unwrap(),
            DEFAULT_REASONING_CONFIG_GPT5_CODEX
        );
        assert_eq!(
            reasoning_config_for(None).unwrap(),
            DEFAULT_REASONING_CONFIG_GPT5
        );
    }

    #[test]
    fn color_mode_strings_are_stable() {
        assert_eq!(ColorMode::Auto.as_str(), "auto");
        assert_eq!(ColorMode::Always.as_str(), "always");
        assert_eq!(ColorMode::Never.as_str(), "never");
    }

    #[test]
    fn parses_chatgpt_login() {
        let message = "Logged in using ChatGPT";
        let parsed = parse_login_success(message);
        assert!(matches!(
            parsed,
            Some(CodexAuthStatus::LoggedIn(CodexAuthMethod::ChatGpt))
        ));
    }

    #[test]
    fn parses_api_key_login() {
        let message = "Logged in using an API key - sk-1234***abcd";
        let parsed = parse_login_success(message);
        match parsed {
            Some(CodexAuthStatus::LoggedIn(CodexAuthMethod::ApiKey { masked_key })) => {
                assert_eq!(masked_key.as_deref(), Some("sk-1234***abcd"));
            }
            other => panic!("unexpected status: {other:?}"),
        }
    }
}

fn default_binary_path() -> PathBuf {
    env::var_os(CODEX_BINARY_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("codex"))
}

#[derive(Clone, Copy)]
enum ConsoleTarget {
    Stdout,
    Stderr,
}

async fn tee_stream<R>(
    mut reader: R,
    target: ConsoleTarget,
    mirror_console: bool,
) -> Result<Vec<u8>, std::io::Error>
where
    R: AsyncRead + Unpin,
{
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        let n = reader.read(&mut chunk).await?;
        if n == 0 {
            break;
        }
        if mirror_console {
            task::block_in_place(|| match target {
                ConsoleTarget::Stdout => {
                    let mut out = stdio::stdout();
                    out.write_all(&chunk[..n])?;
                    out.flush()
                }
                ConsoleTarget::Stderr => {
                    let mut out = stdio::stderr();
                    out.write_all(&chunk[..n])?;
                    out.flush()
                }
            })?;
        }
        buffer.extend_from_slice(&chunk[..n]);
    }
    Ok(buffer)
}

fn parse_login_success(output: &str) -> Option<CodexAuthStatus> {
    let lower = output.to_lowercase();
    if lower.contains("chatgpt") {
        return Some(CodexAuthStatus::LoggedIn(CodexAuthMethod::ChatGpt));
    }
    if lower.contains("api key") || lower.contains("apikey") {
        // Prefer everything after the first " - " so we do not chop the key itself.
        let masked = output
            .split_once(" - ")
            .map(|(_, value)| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .or_else(|| output.split_whitespace().last().map(|v| v.to_string()));
        return Some(CodexAuthStatus::LoggedIn(CodexAuthMethod::ApiKey {
            masked_key: masked,
        }));
    }
    None
}

struct CommandOutput {
    status: ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}
