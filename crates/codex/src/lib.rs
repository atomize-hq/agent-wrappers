//! Minimal async wrapper over the [OpenAI Codex CLI](https://github.com/openai/codex).
//!
//! The CLI ships both an interactive TUI (`codex`) and a headless automation mode (`codex exec`).
//! This crate targets the latter: it shells out to `codex exec`, enforces sensible defaults
//! (non-interactive color handling, timeouts, optional model selection), and returns whatever
//! the CLI prints to stdout (the agent's final response per upstream docs).
//!
//! ## Binary + CODEX_HOME design (Workstream A)
//! - `CodexClientBuilder` will grow environment knobs: `binary_path: PathBuf` (default still
//!   `default_binary_path()`), `codex_home: Option<PathBuf>`, and `create_home_dirs: bool`
//!   (defaults to `true` when `codex_home` is set) that ensures the on-disk layout exists.
//!   The existing `binary(...)` setter remains; new `codex_home(...)` /
//!   `create_home_dirs(...)` methods are additive.
//! - A shared `CommandEnvironment` helper will prepare every `tokio::process::Command`
//!   (exec/login/status/logout/MCP/app-server) without mutating the parent env. It applies
//!   `CODEX_HOME` when provided, mirrors the resolved binary into `CODEX_BINARY`, reuses the
//!   default `RUST_LOG` fallback, and can pre-create `conversations/` and `logs/` directories
//!   when asked.
//! - Expected `CODEX_HOME` contents: root holds `config.toml`, `auth.json`, `.credentials.json`,
//!   and `history.jsonl`; `conversations/` stores `*.jsonl` transcripts; `logs/` stores
//!   `codex-*.log`. When `codex_home` is unset no directories are created and the ambient
//!   `CODEX_HOME` (if any) is inherited.
//! - Backward compatibility: callers that ignore the new options keep today's behavior (binary
//!   from `CODEX_BINARY` or `codex` on PATH, no forced `CODEX_HOME`, same spawning semantics).
//!   Opting into `codex_home` enables app-scoped state isolation without affecting the host
//!   process environment.

use std::{
    collections::{HashMap, HashSet},
    env,
    ffi::{OsStr, OsString},
    fs,
    io::{self as stdio, Write},
    path::{Path, PathBuf},
    process::ExitStatus,
    sync::{Mutex, OnceLock},
    time::{Duration, SystemTime},
};

use semver::{Prerelease, Version};
use serde_json::Value;
use tempfile::TempDir;
use thiserror::Error;
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWriteExt},
    process::Command,
    task, time,
};
use tracing::{debug, warn};

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
const CODEX_HOME_ENV: &str = "CODEX_HOME";
const RUST_LOG_ENV: &str = "RUST_LOG";
const DEFAULT_RUST_LOG: &str = "error";

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

impl std::fmt::Display for CodexReleaseChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            CodexReleaseChannel::Stable => "stable",
            CodexReleaseChannel::Beta => "beta",
            CodexReleaseChannel::Nightly => "nightly",
            CodexReleaseChannel::Custom => "custom",
        };
        write!(f, "{label}")
    }
}

/// Release metadata for a specific Codex build channel.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodexRelease {
    /// Release channel (stable/beta/nightly/custom).
    pub channel: CodexReleaseChannel,
    /// Parsed semantic version for comparison.
    pub version: Version,
}

/// Caller-supplied table of known latest Codex releases.
///
/// The crate intentionally avoids network requests; hosts should populate this
/// with data from their preferred distribution channel (e.g. `npm view
/// @openai/codex version`, `brew info codex --json`, or the GitHub releases
/// API) before requesting an update advisory.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CodexLatestReleases {
    /// Latest stable release version.
    pub stable: Option<Version>,
    /// Latest beta pre-release when available.
    pub beta: Option<Version>,
    /// Latest nightly build when available.
    pub nightly: Option<Version>,
}

impl CodexLatestReleases {
    /// Returns the most appropriate latest release for the given channel,
    /// falling back to a more stable track when channel-specific data is
    /// missing.
    pub fn select_for_channel(
        &self,
        channel: CodexReleaseChannel,
    ) -> (Option<CodexRelease>, CodexReleaseChannel, bool) {
        if let Some(release) = self.release_for_channel(channel) {
            return (Some(release), channel, false);
        }

        let fallback = self
            .stable
            .as_ref()
            .map(|version| CodexRelease {
                channel: CodexReleaseChannel::Stable,
                version: version.clone(),
            })
            .or_else(|| {
                self.beta.as_ref().map(|version| CodexRelease {
                    channel: CodexReleaseChannel::Beta,
                    version: version.clone(),
                })
            })
            .or_else(|| {
                self.nightly.as_ref().map(|version| CodexRelease {
                    channel: CodexReleaseChannel::Nightly,
                    version: version.clone(),
                })
            });

        let fallback_channel = fallback
            .as_ref()
            .map(|release| release.channel)
            .unwrap_or(channel);
        let fell_back = fallback_channel != channel;
        (fallback, fallback_channel, fell_back)
    }

    fn release_for_channel(&self, channel: CodexReleaseChannel) -> Option<CodexRelease> {
        match channel {
            CodexReleaseChannel::Stable => self.stable.as_ref().map(|version| CodexRelease {
                channel,
                version: version.clone(),
            }),
            CodexReleaseChannel::Beta => self.beta.as_ref().map(|version| CodexRelease {
                channel,
                version: version.clone(),
            }),
            CodexReleaseChannel::Nightly => self.nightly.as_ref().map(|version| CodexRelease {
                channel,
                version: version.clone(),
            }),
            CodexReleaseChannel::Custom => None,
        }
    }
}

/// Update guidance derived from comparing local and latest Codex versions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodexUpdateAdvisory {
    /// Local release as parsed from `codex --version`.
    pub local_release: Option<CodexRelease>,
    /// Latest release used for comparison (may be a fallback track).
    pub latest_release: Option<CodexRelease>,
    /// Channel chosen for comparison (local channel when available, otherwise stable).
    pub comparison_channel: CodexReleaseChannel,
    /// High-level outcome to drive host UX.
    pub status: CodexUpdateStatus,
    /// Human-readable hints callers can log or display.
    pub notes: Vec<String>,
}

impl CodexUpdateAdvisory {
    /// True when the host should prompt for or attempt an update.
    pub fn is_update_recommended(&self) -> bool {
        matches!(
            self.status,
            CodexUpdateStatus::UpdateRecommended | CodexUpdateStatus::UnknownLocalVersion
        )
    }
}

/// Enum summarizing whether an update is needed based on provided release data.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CodexUpdateStatus {
    /// Local binary matches the latest known release for the comparison channel.
    UpToDate,
    /// A newer release exists for the comparison channel.
    UpdateRecommended,
    /// Local binary appears newer than the provided release table (e.g., dev build).
    LocalNewerThanKnown,
    /// No local version data was available (probe failure).
    UnknownLocalVersion,
    /// Caller did not provide a comparable latest release.
    UnknownLatestVersion,
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

impl Default for CapabilityProbePlan {
    fn default() -> Self {
        Self { steps: Vec::new() }
    }
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

fn capability_cache() -> &'static Mutex<HashMap<CapabilityCacheKey, CodexCapabilities>> {
    static CACHE: OnceLock<Mutex<HashMap<CapabilityCacheKey, CodexCapabilities>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn capability_cache_key(binary: &Path) -> CapabilityCacheKey {
    let canonical = fs::canonicalize(binary).unwrap_or_else(|_| binary.to_path_buf());
    CapabilityCacheKey {
        binary_path: canonical,
    }
}

fn cached_capabilities(
    key: &CapabilityCacheKey,
    fingerprint: &Option<BinaryFingerprint>,
) -> Option<CodexCapabilities> {
    let cache = capability_cache().lock().ok()?;
    let cached = cache.get(key)?;
    if fingerprints_match(&cached.fingerprint, fingerprint) {
        Some(cached.clone())
    } else {
        None
    }
}

fn update_capability_cache(capabilities: CodexCapabilities) {
    if let Ok(mut cache) = capability_cache().lock() {
        cache.insert(capabilities.cache_key.clone(), capabilities);
    }
}

fn current_fingerprint(key: &CapabilityCacheKey) -> Option<BinaryFingerprint> {
    let canonical = fs::canonicalize(&key.binary_path).ok();
    let metadata_path = canonical
        .as_deref()
        .unwrap_or_else(|| key.binary_path.as_path());
    let metadata = fs::metadata(metadata_path).ok()?;
    Some(BinaryFingerprint {
        canonical_path: canonical,
        modified: metadata.modified().ok(),
        len: Some(metadata.len()),
    })
}

fn fingerprints_match(
    cached: &Option<BinaryFingerprint>,
    fresh: &Option<BinaryFingerprint>,
) -> bool {
    cached == fresh
}

/// High-level client for interacting with `codex exec`.
#[derive(Clone, Debug)]
pub struct CodexClient {
    command_env: CommandEnvironment,
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
        let mut command = Command::new(self.command_env.binary_path());
        command
            .arg("login")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);

        self.command_env.apply(&mut command)?;

        command.spawn().map_err(|source| CodexError::Spawn {
            binary: self.command_env.binary_path().to_path_buf(),
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

    /// Probes the configured binary for version/build metadata and supported feature flags.
    ///
    /// Results are cached per canonical binary path and invalidated when file metadata changes.
    /// Failures are logged and return conservative defaults so callers can gate optional flags.
    pub async fn probe_capabilities(&self) -> CodexCapabilities {
        let cache_key = capability_cache_key(self.command_env.binary_path());
        let fingerprint = current_fingerprint(&cache_key);

        if let Some(cached) = cached_capabilities(&cache_key, &fingerprint) {
            return cached;
        }

        let mut plan = CapabilityProbePlan::default();
        let mut features = CodexFeatureFlags::default();
        let mut version = None;

        plan.steps.push(CapabilityProbeStep::VersionFlag);
        match self.run_basic_command(["--version"]).await {
            Ok(output) => {
                if !output.status.success() {
                    warn!(
                        status = ?output.status,
                        binary = ?cache_key.binary_path,
                        "codex --version exited non-zero"
                    );
                }
                let text = command_output_text(&output);
                if !text.trim().is_empty() {
                    version = Some(parse_version_output(&text));
                }
            }
            Err(error) => warn!(
                ?error,
                binary = ?cache_key.binary_path,
                "codex --version probe failed"
            ),
        }

        let mut parsed_features = false;

        plan.steps.push(CapabilityProbeStep::FeaturesListJson);
        match self.run_basic_command(["features", "list", "--json"]).await {
            Ok(output) => {
                if !output.status.success() {
                    warn!(
                        status = ?output.status,
                        binary = ?cache_key.binary_path,
                        "codex features list --json exited non-zero"
                    );
                }
                if output.status.success() {
                    features.supports_features_list = true;
                }
                let text = command_output_text(&output);
                if let Some(parsed) = parse_features_from_json(&text) {
                    merge_feature_flags(&mut features, parsed);
                    parsed_features = detected_feature_flags(&features);
                } else if !text.is_empty() {
                    let parsed = parse_features_from_text(&text);
                    merge_feature_flags(&mut features, parsed);
                    parsed_features = detected_feature_flags(&features);
                }
            }
            Err(error) => warn!(
                ?error,
                binary = ?cache_key.binary_path,
                "codex features list --json probe failed"
            ),
        }

        if !parsed_features {
            plan.steps.push(CapabilityProbeStep::FeaturesListText);
            match self.run_basic_command(["features", "list"]).await {
                Ok(output) => {
                    if !output.status.success() {
                        warn!(
                            status = ?output.status,
                            binary = ?cache_key.binary_path,
                            "codex features list exited non-zero"
                        );
                    }
                    if output.status.success() {
                        features.supports_features_list = true;
                    }
                    let text = command_output_text(&output);
                    let parsed = parse_features_from_text(&text);
                    merge_feature_flags(&mut features, parsed);
                }
                Err(error) => warn!(
                    ?error,
                    binary = ?cache_key.binary_path,
                    "codex features list probe failed"
                ),
            }
        }

        if should_run_help_fallback(&features) {
            plan.steps.push(CapabilityProbeStep::HelpFallback);
            match self.run_basic_command(["--help"]).await {
                Ok(output) => {
                    if !output.status.success() {
                        warn!(
                            status = ?output.status,
                            binary = ?cache_key.binary_path,
                            "codex --help exited non-zero"
                        );
                    }
                    let text = command_output_text(&output);
                    let parsed = parse_help_output(&text);
                    merge_feature_flags(&mut features, parsed);
                }
                Err(error) => warn!(
                    ?error,
                    binary = ?cache_key.binary_path,
                    "codex --help probe failed"
                ),
            }
        }

        let capabilities = CodexCapabilities {
            cache_key,
            fingerprint,
            version,
            features,
            probe_plan: plan,
            collected_at: SystemTime::now(),
        };

        update_capability_cache(capabilities.clone());
        capabilities
    }

    /// Computes an update advisory by comparing the probed Codex version against
    /// caller-supplied latest releases.
    ///
    /// The crate does not fetch release metadata itself; hosts should populate
    /// [`CodexLatestReleases`] using their preferred update channel (npm,
    /// Homebrew, GitHub releases) and then call this helper. Results leverage
    /// the capability probe cache; callers with an existing
    /// [`CodexCapabilities`] snapshot can skip the probe by invoking
    /// [`update_advisory_from_capabilities`].
    pub async fn update_advisory(
        &self,
        latest_releases: &CodexLatestReleases,
    ) -> CodexUpdateAdvisory {
        let capabilities = self.probe_capabilities().await;
        update_advisory_from_capabilities(&capabilities, latest_releases)
    }

    async fn invoke_codex_exec(&self, prompt: &str) -> Result<String, CodexError> {
        let dir_ctx = self.directory_context()?;

        let mut command = Command::new(self.command_env.binary_path());
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

        self.command_env.apply(&mut command)?;

        let mut child = command.spawn().map_err(|source| CodexError::Spawn {
            binary: self.command_env.binary_path().to_path_buf(),
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
        debug!(
            binary = ?self.command_env.binary_path(),
            bytes = trimmed.len(),
            "received Codex output"
        );
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
        let mut command = Command::new(self.command_env.binary_path());
        command
            .args(args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);

        self.command_env.apply(&mut command)?;

        let mut child = command.spawn().map_err(|source| CodexError::Spawn {
            binary: self.command_env.binary_path().to_path_buf(),
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
    codex_home: Option<PathBuf>,
    create_home_dirs: bool,
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

    /// Sets a custom `CODEX_HOME` path that will be applied per command.
    /// Directories are created by default; disable via [`Self::create_home_dirs`].
    pub fn codex_home(mut self, home: impl Into<PathBuf>) -> Self {
        self.codex_home = Some(home.into());
        self
    }

    /// Controls whether the CODEX_HOME directory tree should be created if missing.
    /// Defaults to `true` when [`Self::codex_home`] is set.
    pub fn create_home_dirs(mut self, enable: bool) -> Self {
        self.create_home_dirs = enable;
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
        let command_env =
            CommandEnvironment::new(self.binary, self.codex_home, self.create_home_dirs);
        CodexClient {
            command_env,
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
            codex_home: None,
            create_home_dirs: true,
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

#[derive(Clone, Debug)]
struct CommandEnvironment {
    binary: PathBuf,
    codex_home: Option<CodexHome>,
    create_home_dirs: bool,
}

impl CommandEnvironment {
    fn new(binary: PathBuf, codex_home: Option<PathBuf>, create_home_dirs: bool) -> Self {
        Self {
            binary,
            codex_home: codex_home.map(CodexHome::new),
            create_home_dirs,
        }
    }

    fn binary_path(&self) -> &Path {
        &self.binary
    }

    fn environment_overrides(&self) -> Result<Vec<(OsString, OsString)>, CodexError> {
        if let Some(home) = &self.codex_home {
            if self.create_home_dirs {
                home.ensure_layout()?;
            }
        }

        let mut envs = Vec::new();
        envs.push((
            OsString::from(CODEX_BINARY_ENV),
            self.binary.as_os_str().to_os_string(),
        ));

        if let Some(home) = &self.codex_home {
            envs.push((
                OsString::from(CODEX_HOME_ENV),
                home.root().as_os_str().to_os_string(),
            ));
        }

        if env::var_os(RUST_LOG_ENV).is_none() {
            envs.push((
                OsString::from(RUST_LOG_ENV),
                OsString::from(DEFAULT_RUST_LOG),
            ));
        }

        Ok(envs)
    }

    fn apply(&self, command: &mut Command) -> Result<(), CodexError> {
        for (key, value) in self.environment_overrides()? {
            command.env(key, value);
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
struct CodexHome {
    root: PathBuf,
}

impl CodexHome {
    fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn root(&self) -> &Path {
        self.root.as_path()
    }

    fn conversations_dir(&self) -> PathBuf {
        self.root.join("conversations")
    }

    fn logs_dir(&self) -> PathBuf {
        self.root.join("logs")
    }

    fn ensure_layout(&self) -> Result<(), CodexError> {
        let conversations = self.conversations_dir();
        let logs = self.logs_dir();
        for path in [self.root(), conversations.as_path(), logs.as_path()] {
            fs::create_dir_all(path).map_err(|source| CodexError::PrepareCodexHome {
                path: path.to_path_buf(),
                source,
            })?;
        }
        Ok(())
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
    #[error("failed to prepare CODEX_HOME at `{path}`: {source}")]
    PrepareCodexHome {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
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

fn command_output_text(output: &CommandOutput) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = stdout.trim_end();
    let stderr = stderr.trim_end();
    if stdout.is_empty() {
        stderr.to_string()
    } else if stderr.is_empty() {
        stdout.to_string()
    } else {
        format!("{stdout}\n{stderr}")
    }
}

fn parse_semver_from_raw(raw: &str) -> Option<Version> {
    for token in raw.split_whitespace() {
        let candidate = token
            .trim_matches(|c: char| matches!(c, '(' | ')' | ',' | ';'))
            .trim_start_matches('v');
        if let Ok(version) = Version::parse(candidate) {
            return Some(version);
        }
    }
    None
}

fn parse_version_output(output: &str) -> CodexVersionInfo {
    let raw = output.trim().to_string();
    let parsed_version = parse_semver_from_raw(&raw);
    let semantic = parsed_version
        .as_ref()
        .map(|version| (version.major, version.minor, version.patch));
    let mut commit = extract_commit_hash(&raw);
    if commit.is_none() {
        for token in raw.split_whitespace() {
            let candidate = token
                .trim_matches(|c: char| matches!(c, '(' | ')' | ',' | ';'))
                .trim_start_matches('v');
            if let Some(cleaned) = cleaned_hex(candidate) {
                commit = Some(cleaned);
                break;
            }
        }
    }
    let channel = parsed_version
        .as_ref()
        .map(release_channel_for_version)
        .unwrap_or_else(|| infer_release_channel(&raw));

    CodexVersionInfo {
        raw,
        semantic,
        commit,
        channel,
    }
}

fn release_channel_for_version(version: &Version) -> CodexReleaseChannel {
    if version.pre.is_empty() {
        CodexReleaseChannel::Stable
    } else {
        let prerelease = version.pre.as_str().to_ascii_lowercase();
        if prerelease.contains("beta") {
            CodexReleaseChannel::Beta
        } else if prerelease.contains("nightly") {
            CodexReleaseChannel::Nightly
        } else {
            CodexReleaseChannel::Custom
        }
    }
}

fn infer_release_channel(raw: &str) -> CodexReleaseChannel {
    let lower = raw.to_ascii_lowercase();
    if lower.contains("beta") {
        CodexReleaseChannel::Beta
    } else if lower.contains("nightly") {
        CodexReleaseChannel::Nightly
    } else {
        CodexReleaseChannel::Custom
    }
}

fn codex_semver(info: &CodexVersionInfo) -> Option<Version> {
    if let Some(parsed) = parse_semver_from_raw(&info.raw) {
        return Some(parsed);
    }
    let (major, minor, patch) = info.semantic?;
    let mut version = Version::new(major, minor, patch);
    if version.pre.is_empty() {
        match info.channel {
            CodexReleaseChannel::Beta => {
                version.pre = Prerelease::new("beta").ok()?;
            }
            CodexReleaseChannel::Nightly => {
                version.pre = Prerelease::new("nightly").ok()?;
            }
            CodexReleaseChannel::Stable | CodexReleaseChannel::Custom => {}
        }
    }
    Some(version)
}

fn codex_release_from_info(info: &CodexVersionInfo) -> Option<CodexRelease> {
    let version = codex_semver(info)?;
    Some(CodexRelease {
        channel: info.channel,
        version,
    })
}

fn extract_commit_hash(raw: &str) -> Option<String> {
    let tokens: Vec<&str> = raw.split_whitespace().collect();
    for window in tokens.windows(2) {
        if window[0].eq_ignore_ascii_case("commit") {
            if let Some(cleaned) = cleaned_hex(window[1]) {
                return Some(cleaned);
            }
        }
    }

    for token in tokens {
        if let Some(cleaned) = cleaned_hex(token) {
            return Some(cleaned);
        }
    }
    None
}

fn cleaned_hex(token: &str) -> Option<String> {
    let trimmed = token
        .trim_matches(|c: char| matches!(c, '(' | ')' | ',' | ';'))
        .trim_start_matches("commit")
        .trim_start_matches(':')
        .trim_start_matches('g');
    if trimmed.len() >= 7 && trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
        Some(trimmed.to_string())
    } else {
        None
    }
}

fn parse_features_from_json(output: &str) -> Option<CodexFeatureFlags> {
    let parsed: Value = serde_json::from_str(output).ok()?;
    let mut tokens = HashSet::new();
    collect_feature_tokens(&parsed, &mut tokens);
    if tokens.is_empty() {
        return None;
    }

    let mut flags = CodexFeatureFlags::default();
    for token in tokens {
        apply_feature_token(&mut flags, &token);
    }
    Some(flags)
}

fn collect_feature_tokens(value: &Value, tokens: &mut HashSet<String>) {
    match value {
        Value::String(value) => {
            if !value.trim().is_empty() {
                tokens.insert(value.clone());
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_feature_tokens(item, tokens);
            }
        }
        Value::Object(map) => {
            for (key, value) in map {
                if let Value::Bool(true) = value {
                    tokens.insert(key.clone());
                }
                collect_feature_tokens(value, tokens);
            }
        }
        _ => {}
    }
}

fn parse_features_from_text(output: &str) -> CodexFeatureFlags {
    let mut flags = CodexFeatureFlags::default();
    let lower = output.to_ascii_lowercase();
    if lower.contains("features list") {
        flags.supports_features_list = true;
    }
    if lower.contains("--output-schema") || lower.contains("output schema") {
        flags.supports_output_schema = true;
    }
    if lower.contains("add-dir") || lower.contains("add dir") {
        flags.supports_add_dir = true;
    }
    if lower.contains("login --mcp") || lower.contains("mcp login") {
        flags.supports_mcp_login = true;
    }
    if lower.contains("login") && lower.contains("mcp") {
        flags.supports_mcp_login = true;
    }

    for token in lower
        .split(|c: char| c.is_ascii_whitespace() || c == ',' || c == ';' || c == '|')
        .filter(|token| !token.is_empty())
    {
        apply_feature_token(&mut flags, token);
    }
    flags
}

fn parse_help_output(output: &str) -> CodexFeatureFlags {
    let mut flags = parse_features_from_text(output);
    let lower = output.to_ascii_lowercase();
    if lower.contains("features list") {
        flags.supports_features_list = true;
    }
    flags
}

fn merge_feature_flags(target: &mut CodexFeatureFlags, update: CodexFeatureFlags) {
    target.supports_features_list |= update.supports_features_list;
    target.supports_output_schema |= update.supports_output_schema;
    target.supports_add_dir |= update.supports_add_dir;
    target.supports_mcp_login |= update.supports_mcp_login;
}

fn detected_feature_flags(flags: &CodexFeatureFlags) -> bool {
    flags.supports_output_schema || flags.supports_add_dir || flags.supports_mcp_login
}

fn should_run_help_fallback(flags: &CodexFeatureFlags) -> bool {
    !flags.supports_features_list
        || !flags.supports_output_schema
        || !flags.supports_add_dir
        || !flags.supports_mcp_login
}

fn normalize_feature_token(token: &str) -> String {
    token
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect()
}

fn apply_feature_token(flags: &mut CodexFeatureFlags, token: &str) {
    let normalized = normalize_feature_token(token);
    let compact = normalized.replace('_', "");
    if normalized.contains("features_list") || compact.contains("featureslist") {
        flags.supports_features_list = true;
    }
    if normalized.contains("output_schema") || compact.contains("outputschema") {
        flags.supports_output_schema = true;
    }
    if normalized.contains("add_dir") || compact.contains("adddir") {
        flags.supports_add_dir = true;
    }
    if normalized.contains("mcp_login")
        || (normalized.contains("login") && normalized.contains("mcp"))
    {
        flags.supports_mcp_login = true;
    }
}

/// Computes an update advisory for a previously probed binary.
///
/// Callers that already have a [`CodexCapabilities`] snapshot can use this
/// helper to avoid re-running `codex --version`. Provide a [`CodexLatestReleases`]
/// table sourced from your preferred distribution channel.
pub fn update_advisory_from_capabilities(
    capabilities: &CodexCapabilities,
    latest_releases: &CodexLatestReleases,
) -> CodexUpdateAdvisory {
    let local_release = capabilities
        .version
        .as_ref()
        .and_then(codex_release_from_info);
    let preferred_channel = local_release
        .as_ref()
        .map(|release| release.channel)
        .unwrap_or(CodexReleaseChannel::Stable);
    let (latest_release, comparison_channel, fell_back) =
        latest_releases.select_for_channel(preferred_channel);
    let mut notes = Vec::new();

    if fell_back {
        notes.push(format!(
            "No latest {preferred_channel} release provided; comparing against {comparison_channel}."
        ));
    }

    let status = match (local_release.as_ref(), latest_release.as_ref()) {
        (None, None) => CodexUpdateStatus::UnknownLatestVersion,
        (None, Some(_)) => CodexUpdateStatus::UnknownLocalVersion,
        (Some(_), None) => CodexUpdateStatus::UnknownLatestVersion,
        (Some(local), Some(latest)) => {
            if local.version < latest.version {
                CodexUpdateStatus::UpdateRecommended
            } else if local.version > latest.version {
                CodexUpdateStatus::LocalNewerThanKnown
            } else {
                CodexUpdateStatus::UpToDate
            }
        }
    };

    match status {
        CodexUpdateStatus::UpdateRecommended => {
            if let (Some(local), Some(latest)) = (local_release.as_ref(), latest_release.as_ref()) {
                notes.push(format!(
                    "Local codex {local_version} is behind latest {comparison_channel} {latest_version}.",
                    local_version = local.version,
                    latest_version = latest.version
                ));
            }
        }
        CodexUpdateStatus::LocalNewerThanKnown => {
            if let Some(local) = local_release.as_ref() {
                let known = latest_release
                    .as_ref()
                    .map(|release| release.version.to_string())
                    .unwrap_or_else(|| "unknown".to_string());
                notes.push(format!(
                    "Local codex {local_version} is newer than provided {comparison_channel} metadata (latest table: {known}).",
                    local_version = local.version
                ));
            }
        }
        CodexUpdateStatus::UnknownLocalVersion => {
            if let Some(latest) = latest_release.as_ref() {
                notes.push(format!(
                    "Latest known {comparison_channel} release is {latest_version}; local version could not be parsed.",
                    latest_version = latest.version
                ));
            } else {
                notes.push(
                    "Local version could not be parsed and no latest release was provided."
                        .to_string(),
                );
            }
        }
        CodexUpdateStatus::UnknownLatestVersion => notes.push(
            "No latest Codex release information provided; update advisory unavailable."
                .to_string(),
        ),
        CodexUpdateStatus::UpToDate => {
            if let Some(latest) = latest_release.as_ref() {
                notes.push(format!(
                    "Local codex matches latest {comparison_channel} release {latest_version}.",
                    latest_version = latest.version
                ));
            }
        }
    }

    CodexUpdateAdvisory {
        local_release,
        latest_release,
        comparison_channel,
        status,
        notes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::sync::{Mutex, OnceLock};
    use std::time::{Duration, SystemTime};

    fn env_guard() -> std::sync::MutexGuard<'static, ()> {
        static ENV_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();
        ENV_MUTEX.get_or_init(|| Mutex::new(())).lock().unwrap()
    }

    fn clear_capability_cache() {
        if let Ok(mut cache) = capability_cache().lock() {
            cache.clear();
        }
    }

    fn write_fake_codex(dir: &Path, script: &str) -> PathBuf {
        let path = dir.join("codex");
        fs::write(&path, script).unwrap();
        let mut perms = fs::metadata(&path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&path, perms).unwrap();
        path
    }

    fn capabilities_with_version(raw_version: &str) -> CodexCapabilities {
        CodexCapabilities {
            cache_key: CapabilityCacheKey {
                binary_path: PathBuf::from("codex"),
            },
            fingerprint: None,
            version: Some(parse_version_output(raw_version)),
            features: CodexFeatureFlags::default(),
            probe_plan: CapabilityProbePlan::default(),
            collected_at: SystemTime::now(),
        }
    }

    fn capabilities_without_version() -> CodexCapabilities {
        CodexCapabilities {
            cache_key: CapabilityCacheKey {
                binary_path: PathBuf::from("codex"),
            },
            fingerprint: None,
            version: None,
            features: CodexFeatureFlags::default(),
            probe_plan: CapabilityProbePlan::default(),
            collected_at: SystemTime::now(),
        }
    }

    #[test]
    fn builder_defaults_are_sane() {
        let builder = CodexClient::builder();
        assert!(builder.model.is_none());
        assert_eq!(builder.timeout, DEFAULT_TIMEOUT);
        assert_eq!(builder.color_mode, ColorMode::Never);
        assert!(builder.codex_home.is_none());
        assert!(builder.create_home_dirs);
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
    fn command_env_sets_expected_overrides() {
        let _guard = env_guard();
        let rust_log_original = env::var_os(RUST_LOG_ENV);
        env::remove_var(RUST_LOG_ENV);

        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().join("codex_home");
        let env_prep =
            CommandEnvironment::new(PathBuf::from("/custom/codex"), Some(home.clone()), true);
        let overrides = env_prep.environment_overrides().unwrap();
        let map: HashMap<OsString, OsString> = overrides.into_iter().collect();

        assert_eq!(
            map.get(&OsString::from(CODEX_BINARY_ENV)),
            Some(&OsString::from("/custom/codex"))
        );
        assert_eq!(
            map.get(&OsString::from(CODEX_HOME_ENV)),
            Some(&home.as_os_str().to_os_string())
        );
        assert_eq!(
            map.get(&OsString::from(RUST_LOG_ENV)),
            Some(&OsString::from(DEFAULT_RUST_LOG))
        );

        assert!(home.is_dir());
        assert!(home.join("conversations").is_dir());
        assert!(home.join("logs").is_dir());

        match rust_log_original {
            Some(value) => env::set_var(RUST_LOG_ENV, value),
            None => env::remove_var(RUST_LOG_ENV),
        }
    }

    #[test]
    fn command_env_respects_existing_rust_log() {
        let _guard = env_guard();
        let rust_log_original = env::var_os(RUST_LOG_ENV);
        env::set_var(RUST_LOG_ENV, "trace");

        let env_prep = CommandEnvironment::new(PathBuf::from("codex"), None, true);
        let overrides = env_prep.environment_overrides().unwrap();
        let map: HashMap<OsString, OsString> = overrides.into_iter().collect();

        assert_eq!(
            map.get(&OsString::from(CODEX_BINARY_ENV)),
            Some(&OsString::from("codex"))
        );
        assert!(!map.contains_key(&OsString::from(RUST_LOG_ENV)));

        match rust_log_original {
            Some(value) => env::set_var(RUST_LOG_ENV, value),
            None => env::remove_var(RUST_LOG_ENV),
        }
    }

    #[test]
    fn command_env_can_skip_home_creation() {
        let _guard = env_guard();
        let rust_log_original = env::var_os(RUST_LOG_ENV);
        env::remove_var(RUST_LOG_ENV);

        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().join("codex_home");
        let env_prep = CommandEnvironment::new(PathBuf::from("codex"), Some(home.clone()), false);
        let overrides = env_prep.environment_overrides().unwrap();
        let map: HashMap<OsString, OsString> = overrides.into_iter().collect();

        assert!(!home.exists());
        assert!(!home.join("conversations").exists());
        assert!(!home.join("logs").exists());
        assert_eq!(
            map.get(&OsString::from(CODEX_HOME_ENV)),
            Some(&home.as_os_str().to_os_string())
        );

        match rust_log_original {
            Some(value) => env::set_var(RUST_LOG_ENV, value),
            None => env::remove_var(RUST_LOG_ENV),
        }
    }

    #[test]
    fn parses_version_output_fields() {
        let parsed = parse_version_output("codex v3.4.5-nightly (commit abc1234)");
        assert_eq!(parsed.semantic, Some((3, 4, 5)));
        assert_eq!(parsed.channel, CodexReleaseChannel::Nightly);
        assert_eq!(parsed.commit.as_deref(), Some("abc1234"));
        assert_eq!(
            parsed.raw,
            "codex v3.4.5-nightly (commit abc1234)".to_string()
        );
    }

    #[test]
    fn update_advisory_detects_newer_release() {
        let capabilities = capabilities_with_version("codex 1.0.0");
        let latest = CodexLatestReleases {
            stable: Some(Version::parse("1.1.0").unwrap()),
            ..Default::default()
        };
        let advisory = update_advisory_from_capabilities(&capabilities, &latest);
        assert_eq!(advisory.status, CodexUpdateStatus::UpdateRecommended);
        assert!(advisory.is_update_recommended());
        assert_eq!(
            advisory
                .latest_release
                .as_ref()
                .map(|release| release.version.clone()),
            latest.stable
        );
    }

    #[test]
    fn update_advisory_handles_unknown_local_version() {
        let capabilities = capabilities_without_version();
        let latest = CodexLatestReleases {
            stable: Some(Version::parse("3.2.1").unwrap()),
            ..Default::default()
        };
        let advisory = update_advisory_from_capabilities(&capabilities, &latest);
        assert_eq!(advisory.status, CodexUpdateStatus::UnknownLocalVersion);
        assert!(advisory.is_update_recommended());
        assert!(advisory
            .notes
            .iter()
            .any(|note| note.contains("could not be parsed")));
    }

    #[test]
    fn update_advisory_marks_up_to_date() {
        let capabilities = capabilities_with_version("codex 2.0.1");
        let latest = CodexLatestReleases {
            stable: Some(Version::parse("2.0.1").unwrap()),
            ..Default::default()
        };
        let advisory = update_advisory_from_capabilities(&capabilities, &latest);
        assert_eq!(advisory.status, CodexUpdateStatus::UpToDate);
        assert!(!advisory.is_update_recommended());
    }

    #[test]
    fn update_advisory_falls_back_when_channel_missing() {
        let capabilities = capabilities_with_version("codex 2.0.0-beta");
        let latest = CodexLatestReleases {
            stable: Some(Version::parse("2.0.1").unwrap()),
            ..Default::default()
        };
        let advisory = update_advisory_from_capabilities(&capabilities, &latest);
        assert_eq!(advisory.comparison_channel, CodexReleaseChannel::Stable);
        assert_eq!(advisory.status, CodexUpdateStatus::UpdateRecommended);
        assert!(advisory
            .notes
            .iter()
            .any(|note| note.contains("comparing against stable")));
    }

    #[test]
    fn parses_features_from_json_and_text() {
        let json = r#"{"features":["output_schema","add_dir"],"mcp_login":true}"#;
        let parsed_json = parse_features_from_json(json).unwrap();
        assert!(parsed_json.supports_output_schema);
        assert!(parsed_json.supports_add_dir);
        assert!(parsed_json.supports_mcp_login);

        let text = "Features: output-schema add-dir login --mcp";
        let parsed_text = parse_features_from_text(text);
        assert!(parsed_text.supports_output_schema);
        assert!(parsed_text.supports_add_dir);
        assert!(parsed_text.supports_mcp_login);
    }

    #[test]
    fn parses_help_output_flags() {
        let help =
            "Usage: codex --output-schema ... add-dir ... login --mcp. See `codex features list`.";
        let parsed = parse_help_output(help);
        assert!(parsed.supports_output_schema);
        assert!(parsed.supports_add_dir);
        assert!(parsed.supports_mcp_login);
        assert!(parsed.supports_features_list);
    }

    #[tokio::test]
    async fn probe_capabilities_caches_and_invalidates() {
        let _guard = env_guard();
        clear_capability_cache();

        let temp = tempfile::tempdir().unwrap();
        let script_v1 = r#"#!/bin/bash
if [[ "$1" == "--version" ]]; then
  echo "codex 1.2.3-beta (commit cafe123)"
elif [[ "$1" == "features" && "$2" == "list" && "$3" == "--json" ]]; then
  echo '{"features":["output_schema","add_dir","mcp_login"]}'
elif [[ "$1" == "features" && "$2" == "list" ]]; then
  echo "output_schema add-dir login --mcp"
elif [[ "$1" == "--help" ]]; then
  echo "Usage: codex --output-schema add-dir login --mcp"
fi
"#;
        let binary = write_fake_codex(temp.path(), script_v1);
        let client = CodexClient::builder()
            .binary(&binary)
            .timeout(Duration::from_secs(5))
            .build();

        let first = client.probe_capabilities().await;
        assert_eq!(
            first.version.as_ref().and_then(|v| v.semantic),
            Some((1, 2, 3))
        );
        assert_eq!(
            first.version.as_ref().map(|v| v.channel),
            Some(CodexReleaseChannel::Beta)
        );
        assert_eq!(
            first.version.as_ref().and_then(|v| v.commit.as_deref()),
            Some("cafe123")
        );
        assert!(first.features.supports_features_list);
        assert!(first.features.supports_output_schema);
        assert!(first.features.supports_add_dir);
        assert!(first.features.supports_mcp_login);

        let cached = client.probe_capabilities().await;
        assert_eq!(cached, first);

        let script_v2 = r#"#!/bin/bash
if [[ "$1" == "--version" ]]; then
  echo "codex 2.0.0 (commit deadbeef)"
elif [[ "$1" == "features" && "$2" == "list" && "$3" == "--json" ]]; then
  echo '{"features":["add_dir"]}'
elif [[ "$1" == "features" && "$2" == "list" ]]; then
  echo "add-dir"
elif [[ "$1" == "--help" ]]; then
  echo "Usage: codex add-dir"
fi
"#;
        fs::write(&binary, script_v2).unwrap();
        let mut perms = fs::metadata(&binary).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&binary, perms).unwrap();

        let refreshed = client.probe_capabilities().await;
        assert_ne!(refreshed.version, first.version);
        assert_eq!(
            refreshed.version.as_ref().and_then(|v| v.semantic),
            Some((2, 0, 0))
        );
        assert!(refreshed.features.supports_features_list);
        assert!(refreshed.features.supports_add_dir);
        assert!(!refreshed.features.supports_output_schema);
        assert!(!refreshed.features.supports_mcp_login);
        clear_capability_cache();
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
