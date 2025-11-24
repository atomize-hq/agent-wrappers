//! Async wrapper over the [OpenAI Codex CLI](https://github.com/openai/codex).
//!
//! Two surfaces are exposed:
//! - [`CodexClient::send_prompt`] for a single prompt/response with optional `--json` output.
//! - [`CodexClient::stream_exec`] for typed, real-time JSONL events from `codex exec --json`,
//!   returning an [`ExecStream`] with an event stream plus a completion future.
//!
//! Logging + defaults:
//! - Set `json_event_log` on the builder or [`ExecStreamRequest`] to tee raw JSONL lines to disk
//!   before parsing. Logs append to existing files, flush per line, and create parent directories.
//! - Disable `mirror_stdout` when parsing JSON so stdout stays under caller control; `quiet`
//!   controls stderr mirroring.
//! - When `RUST_LOG` is unset, the spawned `codex` process inherits `RUST_LOG=error` to mute
//!   verbose tracing. Existing values are preserved if you want more detail.
//!
//! See `examples/json_stream.rs` for an end-to-end streaming walkthrough.

use std::{
    collections::BTreeMap,
    env,
    ffi::OsStr,
    future::Future,
    io::{self as stdio, Write},
    path::{Path, PathBuf},
    pin::Pin,
    process::ExitStatus,
    task::{Context, Poll},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use futures_core::Stream;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tempfile::TempDir;
use thiserror::Error;
use tokio::{
    fs,
    fs::OpenOptions,
    io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWriteExt, BufReader, BufWriter},
    process::Command,
    sync::mpsc,
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
    json_event_log: Option<PathBuf>,
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

    /// Streams structured JSONL events from `codex exec --json`.
    ///
    /// Respects `mirror_stdout` (raw JSON echoing) and tees raw lines to `json_event_log` when
    /// configured on the builder or request. Returns an [`ExecStream`] with both the parsed event
    /// stream and a completion future that reports `--output-last-message`/schema paths.
    pub async fn stream_exec(
        &self,
        request: ExecStreamRequest,
    ) -> Result<ExecStream, ExecStreamError> {
        if request.prompt.trim().is_empty() {
            return Err(CodexError::EmptyPrompt.into());
        }

        let ExecStreamRequest {
            prompt,
            idle_timeout,
            output_last_message,
            output_schema,
            json_event_log,
        } = request;

        let dir_ctx = self.directory_context()?;
        let dir_path = dir_ctx.path().to_path_buf();
        let last_message_path =
            output_last_message.unwrap_or_else(|| unique_temp_path("codex_last_message_", "txt"));

        let mut command = Command::new(&self.binary);
        command
            .arg("exec")
            .arg("--color")
            .arg(ColorMode::Never.as_str())
            .arg("--skip-git-repo-check")
            .arg("--json")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .stdin(std::process::Stdio::piped())
            .kill_on_drop(true)
            .current_dir(&dir_path);

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

        command.arg("--output-last-message").arg(&last_message_path);

        if let Some(schema_path) = &output_schema {
            command.arg("--output-schema").arg(schema_path);
        }

        apply_rust_log_default(&mut command);

        let mut child = command.spawn().map_err(|source| CodexError::Spawn {
            binary: self.binary.clone(),
            source,
        })?;

        {
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
        }

        let stdout = child.stdout.take().ok_or(CodexError::StdoutUnavailable)?;
        let stderr = child.stderr.take().ok_or(CodexError::StderrUnavailable)?;

        let (tx, rx) = mpsc::channel(32);
        let json_log = prepare_json_log(
            json_event_log
                .or_else(|| self.json_event_log.clone())
                .filter(|path| !path.as_os_str().is_empty()),
        )
        .await?;
        let stdout_task = tokio::spawn(forward_json_events(
            stdout,
            tx,
            self.mirror_stdout,
            json_log,
        ));
        let stderr_task = tokio::spawn(tee_stream(stderr, ConsoleTarget::Stderr, !self.quiet));

        let events = EventChannelStream::new(rx, idle_timeout);
        let timeout = self.timeout;
        let schema_path = output_schema.clone();
        let completion = Box::pin(async move {
            let _dir_ctx = dir_ctx;
            let wait_task = async move {
                let status = child
                    .wait()
                    .await
                    .map_err(|source| CodexError::Wait { source })?;
                let stdout_result = stdout_task.await.map_err(CodexError::Join)?;
                stdout_result?;
                let stderr_bytes = stderr_task
                    .await
                    .map_err(CodexError::Join)?
                    .map_err(CodexError::CaptureIo)?;
                if !status.success() {
                    return Err(CodexError::NonZeroExit {
                        status,
                        stderr: String::from_utf8(stderr_bytes).unwrap_or_default(),
                    }
                    .into());
                }
                let last_message = read_last_message(&last_message_path).await;
                Ok(ExecCompletion {
                    status,
                    last_message_path: Some(last_message_path),
                    last_message,
                    schema_path,
                })
            };

            if timeout.is_zero() {
                wait_task.await
            } else {
                match time::timeout(timeout, wait_task).await {
                    Ok(result) => result,
                    Err(_) => Err(CodexError::Timeout { timeout }.into()),
                }
            }
        });

        Ok(ExecStream {
            events: Box::pin(events),
            completion,
        })
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

        apply_rust_log_default(&mut command);

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

    /// Applies the most recent Codex diff by invoking `codex apply` and captures stdout/stderr.
    pub async fn apply(&self) -> Result<ApplyDiffArtifacts, CodexError> {
        self.apply_or_diff("apply").await
    }

    /// Shows the most recent Codex diff by invoking `codex diff` and captures stdout/stderr.
    pub async fn diff(&self) -> Result<ApplyDiffArtifacts, CodexError> {
        self.apply_or_diff("diff").await
    }

    async fn apply_or_diff(&self, subcommand: &str) -> Result<ApplyDiffArtifacts, CodexError> {
        let dir_ctx = self.directory_context()?;

        let mut command = Command::new(&self.binary);
        command
            .arg(subcommand)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .current_dir(dir_ctx.path());

        apply_rust_log_default(&mut command);

        let mut child = command.spawn().map_err(|source| CodexError::Spawn {
            binary: self.binary.clone(),
            source,
        })?;

        let stdout = child.stdout.take().ok_or(CodexError::StdoutUnavailable)?;
        let stderr = child.stderr.take().ok_or(CodexError::StderrUnavailable)?;

        let stdout_task = tokio::spawn(tee_stream(
            stdout,
            ConsoleTarget::Stdout,
            self.mirror_stdout,
        ));
        let stderr_task = tokio::spawn(tee_stream(
            stderr,
            ConsoleTarget::Stderr,
            !self.quiet,
        ));

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

        Ok(ApplyDiffArtifacts {
            status,
            stdout: String::from_utf8(stdout_bytes)?,
            stderr: String::from_utf8(stderr_bytes)?,
        })
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

        apply_rust_log_default(&mut command);

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

        apply_rust_log_default(&mut command);

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
    json_event_log: Option<PathBuf>,
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

    /// Tees each JSONL event line from [`CodexClient::stream_exec`] into a log file.
    /// Logs append to existing files, flush after each line, and create parent directories as
    /// needed. [`ExecStreamRequest::json_event_log`] overrides this default per request.
    pub fn json_event_log(mut self, path: impl Into<PathBuf>) -> Self {
        self.json_event_log = Some(path.into());
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
            json_event_log: self.json_event_log,
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
            json_event_log: None,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
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
    /// Optional file path that receives a tee of every raw JSONL event line as it streams in.
    /// Appends to existing files, flushes each line, and creates parent directories. Overrides
    /// [`CodexClientBuilder::json_event_log`] for this request when provided.
    pub json_event_log: Option<PathBuf>,
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

/// Captured output from `codex apply` or `codex diff`.
#[derive(Clone, Debug)]
pub struct ApplyDiffArtifacts {
    pub status: ExitStatus,
    pub stdout: String,
    pub stderr: String,
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

async fn prepare_json_log(path: Option<PathBuf>) -> Result<Option<JsonLogSink>, ExecStreamError> {
    match path {
        Some(path) => {
            let sink = JsonLogSink::new(path)
                .await
                .map_err(|err| ExecStreamError::from(CodexError::CaptureIo(err)))?;
            Ok(Some(sink))
        }
        None => Ok(None),
    }
}

#[derive(Debug)]
struct JsonLogSink {
    writer: BufWriter<fs::File>,
}

impl JsonLogSink {
    async fn new(path: PathBuf) -> Result<Self, std::io::Error> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).await?;
            }
        }

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .await?;

        Ok(Self {
            writer: BufWriter::new(file),
        })
    }

    async fn write_line(&mut self, line: &str) -> Result<(), std::io::Error> {
        self.writer.write_all(line.as_bytes()).await?;
        self.writer.write_all(b"\n").await?;
        self.writer.flush().await
    }
}

struct EventChannelStream {
    rx: mpsc::Receiver<Result<ThreadEvent, ExecStreamError>>,
    idle_timeout: Option<Duration>,
    idle_timer: Option<Pin<Box<time::Sleep>>>,
}

impl EventChannelStream {
    fn new(
        rx: mpsc::Receiver<Result<ThreadEvent, ExecStreamError>>,
        idle_timeout: Option<Duration>,
    ) -> Self {
        Self {
            rx,
            idle_timeout,
            idle_timer: None,
        }
    }

    fn reset_timer(&mut self) {
        self.idle_timer = self
            .idle_timeout
            .map(|duration| Box::pin(time::sleep(duration)));
    }
}

impl Stream for EventChannelStream {
    type Item = Result<ThreadEvent, ExecStreamError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();

        if let Some(timer) = this.idle_timer.as_mut() {
            if let Poll::Ready(()) = timer.as_mut().poll(cx) {
                let idle_for = this.idle_timeout.expect("idle_timer implies timeout");
                this.idle_timer = None;
                return Poll::Ready(Some(Err(ExecStreamError::IdleTimeout { idle_for })));
            }
        }

        match this.rx.poll_recv(cx) {
            Poll::Ready(Some(item)) => {
                if this.idle_timeout.is_some() {
                    this.reset_timer();
                }
                Poll::Ready(Some(item))
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => {
                if this.idle_timer.is_none() {
                    if let Some(duration) = this.idle_timeout {
                        let mut sleep = Box::pin(time::sleep(duration));
                        if let Poll::Ready(()) = sleep.as_mut().poll(cx) {
                            return Poll::Ready(Some(Err(ExecStreamError::IdleTimeout {
                                idle_for: duration,
                            })));
                        }
                        this.idle_timer = Some(sleep);
                    }
                }
                Poll::Pending
            }
        }
    }
}

async fn forward_json_events<R>(
    reader: R,
    sender: mpsc::Sender<Result<ThreadEvent, ExecStreamError>>,
    mirror_stdout: bool,
    mut log: Option<JsonLogSink>,
) -> Result<(), ExecStreamError>
where
    R: AsyncRead + Unpin,
{
    let mut lines = BufReader::new(reader).lines();
    loop {
        let line = match lines.next_line().await {
            Ok(Some(line)) => line,
            Ok(None) => break,
            Err(err) => {
                return Err(CodexError::CaptureIo(err).into());
            }
        };

        if line.trim().is_empty() {
            continue;
        }

        if let Some(sink) = log.as_mut() {
            sink.write_line(&line)
                .await
                .map_err(|err| ExecStreamError::from(CodexError::CaptureIo(err)))?;
        }

        if mirror_stdout {
            if let Err(err) = task::block_in_place(|| {
                let mut out = stdio::stdout();
                out.write_all(line.as_bytes())?;
                out.write_all(b"\n")?;
                out.flush()
            }) {
                return Err(CodexError::CaptureIo(err).into());
            }
        }

        let parsed =
            serde_json::from_str::<ThreadEvent>(&line).map_err(|source| ExecStreamError::Parse {
                line: line.clone(),
                source,
            });
        let send_result = match parsed {
            Ok(event) => sender.send(Ok(event)).await,
            Err(err) => {
                let _ = sender.send(Err(err)).await;
                break;
            }
        };
        if send_result.is_err() {
            break;
        }
    }

    Ok(())
}

async fn read_last_message(path: &Path) -> Option<String> {
    match fs::read_to_string(path).await {
        Ok(contents) => Some(contents),
        Err(_) => None,
    }
}

fn unique_temp_path(prefix: &str, extension: &str) -> PathBuf {
    let mut path = env::temp_dir();
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_nanos();
    path.push(format!(
        "{prefix}{timestamp}_{}.{}",
        std::process::id(),
        extension
    ));
    path
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
    use futures_util::{pin_mut, StreamExt};
    use serde_json::json;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use std::sync::{Mutex, OnceLock};
    use tokio::io::AsyncWriteExt;

    fn env_guard() -> std::sync::MutexGuard<'static, ()> {
        static ENV_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();
        ENV_MUTEX.get_or_init(|| Mutex::new(())).lock().unwrap()
    }

    #[tokio::test]
    async fn json_stream_preserves_order_and_parses_tool_calls() {
        let lines = vec![
            r#"{"type":"thread.started","thread_id":"thread-1"}"#.to_string(),
            serde_json::to_string(&json!({
                "type": "item.started",
                "thread_id": "thread-1",
                "turn_id": "turn-1",
                "item_id": "item-1",
                "item_type": "mcp_tool_call",
                "content": {
                    "server_name": "files",
                    "tool_name": "list",
                    "status": "running"
                }
            }))
            .unwrap(),
            serde_json::to_string(&json!({
                "type": "item.delta",
                "thread_id": "thread-1",
                "turn_id": "turn-1",
                "item_id": "item-1",
                "item_type": "mcp_tool_call",
                "delta": {
                    "result": {"paths": ["foo.rs"]},
                    "status": "completed"
                }
            }))
            .unwrap(),
        ];

        let (mut writer, reader) = tokio::io::duplex(4096);
        let (tx, rx) = mpsc::channel(8);
        let forward_handle = tokio::spawn(forward_json_events(reader, tx, false, None));

        for line in &lines {
            writer.write_all(line.as_bytes()).await.unwrap();
            writer.write_all(b"\n").await.unwrap();
        }
        writer.shutdown().await.unwrap();

        let stream = EventChannelStream::new(rx, None);
        pin_mut!(stream);
        let events: Vec<_> = stream.collect().await;
        forward_handle.await.unwrap().unwrap();

        assert_eq!(events.len(), lines.len(), "events: {events:?}");

        match &events[0] {
            Ok(ThreadEvent::ThreadStarted(event)) => {
                assert_eq!(event.thread_id, "thread-1");
            }
            other => panic!("unexpected first event: {other:?}"),
        }

        match &events[1] {
            Ok(ThreadEvent::ItemStarted(envelope)) => {
                assert_eq!(envelope.thread_id, "thread-1");
                assert_eq!(envelope.turn_id, "turn-1");
                match &envelope.item.payload {
                    ItemPayload::McpToolCall(state) => {
                        assert_eq!(state.server_name, "files");
                        assert_eq!(state.tool_name, "list");
                        assert_eq!(state.status, ToolCallStatus::Running);
                    }
                    other => panic!("unexpected payload: {other:?}"),
                }
            }
            other => panic!("unexpected second event: {other:?}"),
        }

        match &events[2] {
            Ok(ThreadEvent::ItemDelta(delta)) => {
                assert_eq!(delta.item_id, "item-1");
                match &delta.delta {
                    ItemDeltaPayload::McpToolCall(call_delta) => {
                        assert_eq!(call_delta.status, ToolCallStatus::Completed);
                        let result = call_delta
                            .result
                            .as_ref()
                            .expect("tool call delta result is captured");
                        assert_eq!(result["paths"][0], "foo.rs");
                    }
                    other => panic!("unexpected delta payload: {other:?}"),
                }
            }
            other => panic!("unexpected third event: {other:?}"),
        }
    }

    #[tokio::test]
    async fn json_stream_propagates_parse_errors() {
        let (mut writer, reader) = tokio::io::duplex(1024);
        let (tx, rx) = mpsc::channel(4);
        let forward_handle = tokio::spawn(forward_json_events(reader, tx, false, None));

        writer
            .write_all(br#"{"type":"thread.started","thread_id":"thread-err"}"#)
            .await
            .unwrap();
        writer.write_all(b"\nthis is not json\n").await.unwrap();
        writer.shutdown().await.unwrap();

        let stream = EventChannelStream::new(rx, None);
        pin_mut!(stream);
        let events: Vec<_> = stream.collect().await;
        forward_handle.await.unwrap().unwrap();

        assert_eq!(events.len(), 2);
        assert!(matches!(
            events[0],
            Ok(ThreadEvent::ThreadStarted(ThreadStarted { ref thread_id, .. }))
                if thread_id == "thread-err"
        ));
        match &events[1] {
            Err(ExecStreamError::Parse { line, .. }) => assert_eq!(line, "this is not json"),
            other => panic!("expected parse error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn json_stream_tees_logs_before_forwarding() {
        let lines = vec![
            r#"{"type":"thread.started","thread_id":"tee-thread"}"#.to_string(),
            r#"{"type":"turn.started","thread_id":"tee-thread","turn_id":"turn-tee"}"#.to_string(),
        ];

        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("events.log");

        let (mut writer, reader) = tokio::io::duplex(2048);
        let (tx, rx) = mpsc::channel(4);
        let log_sink = JsonLogSink::new(log_path.clone()).await.unwrap();
        let forward_handle = tokio::spawn(forward_json_events(reader, tx, false, Some(log_sink)));

        let stream = EventChannelStream::new(rx, None);
        pin_mut!(stream);

        writer.write_all(lines[0].as_bytes()).await.unwrap();
        writer.write_all(b"\n").await.unwrap();

        let first = stream.next().await.unwrap().unwrap();
        assert!(matches!(first, ThreadEvent::ThreadStarted(_)));

        let logged = fs::read_to_string(&log_path).await.unwrap();
        assert_eq!(logged, format!("{}\n", lines[0]));

        writer.write_all(lines[1].as_bytes()).await.unwrap();
        writer.write_all(b"\n").await.unwrap();
        writer.shutdown().await.unwrap();

        let second = stream.next().await.unwrap().unwrap();
        assert!(matches!(second, ThreadEvent::TurnStarted(_)));
        assert!(stream.next().await.is_none());

        forward_handle.await.unwrap().unwrap();

        let final_log = fs::read_to_string(&log_path).await.unwrap();
        assert_eq!(final_log, format!("{}\n{}\n", lines[0], lines[1]));
    }

    #[tokio::test]
    async fn json_event_log_captures_apply_diff_and_tool_payloads() {
        let diff = "@@ -1 +1 @@\n-fn foo() {}\n+fn bar() {}";
        let lines = vec![
            r#"{"type":"thread.started","thread_id":"log-thread"}"#.to_string(),
            serde_json::to_string(&json!({
                "type": "item.started",
                "thread_id": "log-thread",
                "turn_id": "turn-log",
                "item_id": "apply-1",
                "item_type": "file_change",
                "content": {
                    "path": "src/main.rs",
                    "change": "apply",
                    "diff": diff,
                    "stdout": "patched\n"
                }
            }))
            .unwrap(),
            serde_json::to_string(&json!({
                "type": "item.delta",
                "thread_id": "log-thread",
                "turn_id": "turn-log",
                "item_id": "apply-1",
                "item_type": "file_change",
                "delta": {
                    "diff": diff,
                    "stderr": "warning",
                    "exit_code": 2
                }
            }))
            .unwrap(),
            serde_json::to_string(&json!({
                "type": "item.delta",
                "thread_id": "log-thread",
                "turn_id": "turn-log",
                "item_id": "tool-1",
                "item_type": "mcp_tool_call",
                "delta": {
                    "result": {"paths": ["a.rs", "b.rs"]},
                    "status": "completed"
                }
            }))
            .unwrap(),
        ];

        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("json.log");

        let (mut writer, reader) = tokio::io::duplex(4096);
        let (tx, rx) = mpsc::channel(8);
        let log_sink = JsonLogSink::new(log_path.clone()).await.unwrap();
        let forward_handle = tokio::spawn(forward_json_events(reader, tx, false, Some(log_sink)));

        for line in &lines {
            writer.write_all(line.as_bytes()).await.unwrap();
            writer.write_all(b"\n").await.unwrap();
        }
        writer.shutdown().await.unwrap();

        let stream = EventChannelStream::new(rx, None);
        pin_mut!(stream);
        let events: Vec<_> = stream.collect().await;
        forward_handle.await.unwrap().unwrap();

        assert_eq!(events.len(), lines.len());

        let log_contents = fs::read_to_string(&log_path).await.unwrap();
        assert_eq!(log_contents, lines.join("\n") + "\n");
    }

    #[tokio::test]
    async fn event_channel_stream_times_out_when_idle() {
        let (_tx, rx) = mpsc::channel(1);
        let stream = EventChannelStream::new(rx, Some(Duration::from_millis(5)));
        pin_mut!(stream);

        let next = stream.next().await;
        match next {
            Some(Err(ExecStreamError::IdleTimeout { idle_for })) => {
                assert_eq!(idle_for, Duration::from_millis(5));
            }
            other => panic!("expected idle timeout, got {other:?}"),
        }
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
        assert!(builder.json_event_log.is_none());
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
    fn builder_sets_json_event_log() {
        let client = CodexClient::builder().json_event_log("events.log").build();
        assert_eq!(client.json_event_log, Some(PathBuf::from("events.log")));
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
    fn default_rust_log_is_error_when_unset() {
        let _guard = env_guard();
        let original = env::var_os("RUST_LOG");
        env::remove_var("RUST_LOG");

        assert_eq!(default_rust_log_value(), Some("error"));

        if let Some(value) = original {
            env::set_var("RUST_LOG", value);
        } else {
            env::remove_var("RUST_LOG");
        }
    }

    #[test]
    fn default_rust_log_respects_existing_env() {
        let _guard = env_guard();
        let original = env::var_os("RUST_LOG");
        env::set_var("RUST_LOG", "info");

        assert_eq!(default_rust_log_value(), None);

        if let Some(value) = original {
            env::set_var("RUST_LOG", value);
        } else {
            env::remove_var("RUST_LOG");
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn apply_and_diff_capture_outputs_and_status() {
        let dir = tempfile::tempdir().unwrap();
        let script_path = dir.path().join("codex");
        std::fs::write(
            &script_path,
            r#"#!/usr/bin/env bash
set -e
cmd="$1"
if [[ "$cmd" == "apply" ]]; then
  echo "applied"
  echo "apply-stderr" >&2
  exit 0
elif [[ "$cmd" == "diff" ]]; then
  echo "diff-body"
  echo "diff-stderr" >&2
  exit 3
else
  echo "unknown $cmd" >&2
  exit 99
fi
"#,
        )
        .unwrap();
        let mut perms = std::fs::metadata(&script_path).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&script_path, perms).unwrap();

        let client = CodexClient::builder()
            .binary(&script_path)
            .mirror_stdout(false)
            .quiet(true)
            .build();

        let apply = client.apply().await.unwrap();
        assert!(apply.status.success());
        assert_eq!(apply.stdout.trim(), "applied");
        assert_eq!(apply.stderr.trim(), "apply-stderr");

        let diff = client.diff().await.unwrap();
        assert!(!diff.status.success());
        assert_eq!(diff.status.code(), Some(3));
        assert_eq!(diff.stdout.trim(), "diff-body");
        assert_eq!(diff.stderr.trim(), "diff-stderr");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn apply_respects_rust_log_default() {
        let _guard = env_guard();
        let original = env::var_os("RUST_LOG");
        env::remove_var("RUST_LOG");

        let dir = tempfile::tempdir().unwrap();
        let script_path = dir.path().join("codex-rust-log");
        std::fs::write(
            &script_path,
            r#"#!/usr/bin/env bash
echo "${RUST_LOG:-missing}"
exit 0
"#,
        )
        .unwrap();
        let mut perms = std::fs::metadata(&script_path).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&script_path, perms).unwrap();

        let client = CodexClient::builder()
            .binary(&script_path)
            .mirror_stdout(false)
            .quiet(true)
            .build();

        let apply = client.apply().await.unwrap();
        assert_eq!(apply.stdout.trim(), "error");

        if let Some(value) = original {
            env::set_var("RUST_LOG", value);
        } else {
            env::remove_var("RUST_LOG");
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

fn default_rust_log_value() -> Option<&'static str> {
    env::var_os("RUST_LOG").is_none().then_some("error")
}

fn apply_rust_log_default(command: &mut Command) {
    if let Some(value) = default_rust_log_value() {
        command.env("RUST_LOG", value);
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
