//! Minimal async wrapper over the [OpenAI Codex CLI](https://github.com/openai/codex).
//!
//! The CLI ships both an interactive TUI (`codex`) and a headless automation mode (`codex exec`).
//! This crate targets the latter: it shells out to `codex exec`, enforces sensible defaults
//! (non-interactive color handling, timeouts, optional model selection), and returns whatever
//! the CLI prints to stdout (the agent's final response per upstream docs).

use std::{
    env,
    ffi::OsStr,
    io::{self as stdio, Write},
    path::{Path, PathBuf},
    process::ExitStatus,
    time::{Duration, SystemTime},
};

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

/// Snapshot of Codex CLI capabilities derived from probing a specific binary.
///
/// Instances of this type are intended to be cached per binary path so callers can
/// gate optional flags (like `--output-schema`) without repeatedly spawning the CLI.
/// A process-wide `HashMap<CapabilityCacheKey, CodexCapabilities>` (behind a mutex/once)
/// keeps probes cheap; entries should use canonical binary paths where possible and
/// ship a [`BinaryFingerprint`] so we can invalidate stale snapshots when the binary
/// on disk changes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodexCapabilities {
    /// Canonical path used as the cache key.
    pub cache_key: CapabilityCacheKey,
    /// File metadata used to detect when a cached entry is stale.
    pub fingerprint: Option<BinaryFingerprint>,
    /// Parsed output from `codex --version`; `None` when the command fails.
    pub version: Option<CodexVersionInfo>,
    /// Known feature toggles; fields default to `false` when detection fails.
    pub features: CodexFeatureFlags,
    /// Steps attempted while interrogating the binary (version, features, help).
    pub probe_plan: CapabilityProbePlan,
    /// Timestamp of when the probe finished.
    pub collected_at: SystemTime,
}

/// Parsed version details emitted by `codex --version`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodexVersionInfo {
    /// Raw stdout from `codex --version` so we do not lose channel/build metadata.
    pub raw: String,
    /// Parsed `major.minor.patch` triplet when the output contains a semantic version.
    pub semantic: Option<(u64, u64, u64)>,
    /// Optional commit hash or build identifier printed by pre-release builds.
    pub commit: Option<String>,
    /// Release channel inferred from the version string suffix (e.g., `-beta`).
    pub channel: CodexReleaseChannel,
}

/// Release channel segments inferred from the Codex version string.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CodexReleaseChannel {
    Stable,
    Beta,
    Nightly,
    /// Fallback for bespoke or vendor-patched builds.
    Custom,
}

/// Feature gates for Codex CLI flags.
///
/// All fields default to `false` so callers can conservatively avoid passing flags
/// unless probes prove that the binary understands them.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CodexFeatureFlags {
    /// True when `codex features list` is available.
    pub supports_features_list: bool,
    /// True when `--output-schema` is accepted by `codex exec`.
    pub supports_output_schema: bool,
    /// True when `codex add-dir` is available for recursive prompting.
    pub supports_add_dir: bool,
    /// True when `codex login --mcp` is recognized for MCP integration.
    pub supports_mcp_login: bool,
}

/// Description of how we interrogate the CLI to populate a [`CodexCapabilities`] snapshot.
///
/// Probes should prefer an explicit feature list when available, fall back to parsing
/// `codex --help` flags, and finally rely on coarse version heuristics. Each attempted
/// step is recorded so hosts can trace why a particular flag was enabled or skipped.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilityProbePlan {
    /// Steps attempted in order; consumers should push entries as probes run.
    pub steps: Vec<CapabilityProbeStep>,
}

/// Command-level probes used to infer feature support.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CapabilityProbeStep {
    /// Invoke `codex --version` to capture version/build metadata.
    VersionFlag,
    /// Prefer `codex features list --json` when supported for structured output.
    FeaturesListJson,
    /// Fallback to `codex features list` when only plain text is available.
    FeaturesListText,
    /// Parse `codex --help` to spot known flags (e.g., `--output-schema`, `add-dir`, `login --mcp`) when the features list is missing.
    HelpFallback,
}

/// Cache key for capability snapshots derived from a specific Codex binary path.
///
/// Cache lookups should canonicalize the path when possible so symlinked binaries
/// collapse to a single entry.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct CapabilityCacheKey {
    /// Canonical binary path when resolvable; otherwise the original path.
    pub binary_path: PathBuf,
}

/// File metadata used to invalidate cached capability snapshots when the binary changes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BinaryFingerprint {
    /// Canonical path if the binary resolves through symlinks.
    pub canonical_path: Option<PathBuf>,
    /// Last modification time of the binary on disk (`metadata().modified()`).
    pub modified: Option<SystemTime>,
    /// File length from `metadata().len()`, useful for cheap change detection.
    pub len: Option<u64>,
}

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
