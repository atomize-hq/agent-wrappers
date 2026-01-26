//! Async helper around the OpenAI Codex CLI for programmatic prompting, streaming, apply/diff helpers, and server flows.
//!
//! Shells out to `codex exec`, applies sane defaults (non-interactive color handling, timeouts, model hints), and surfaces single-response, streaming, apply/diff, and MCP/app-server helpers.
//!
//! ## Setup: binary + `CODEX_HOME`
//! - Defaults pull `CODEX_BINARY` or `codex` on `PATH`; call [`CodexClientBuilder::binary`] (optionally fed by [`resolve_bundled_binary`]) to pin an app-bundled binary without touching user installs.
//! - Isolate state with [`CodexClientBuilder::codex_home`] (config/auth/history/logs live under that directory) and optionally create the layout with [`CodexClientBuilder::create_home_dirs`]. [`CodexHomeLayout`] inspects `config.toml`, `auth.json`, `.credentials.json`, `history.jsonl`, `conversations/`, and `logs/`.
//! - [`CodexHomeLayout::seed_auth_from`] copies `auth.json`/`.credentials.json` from a trusted seed home into an isolated `CODEX_HOME` without touching history/logs; use [`AuthSeedOptions`] to require files or skip missing ones.
//! - [`AuthSessionHelper`] checks `codex login status` and can launch ChatGPT or API key login flows with an app-scoped `CODEX_HOME` without mutating the parent process env.
//! - Wrapper defaults: temp working dir per call unless `working_dir` is set, `--skip-git-repo-check`, 120s timeout (use `Duration::ZERO` to disable), ANSI colors off, `RUST_LOG=error` if unset.
//! - Model defaults: `gpt-5*`/`gpt-5.1*` (including codex variants) get `model_reasoning_effort="medium"`/`model_reasoning_summary="auto"`/`model_verbosity="low"` to avoid unsupported “minimal” combos.
//!
//! ## Bundled binary (Workstream J)
//! - Apps can ship Codex inside an app-owned bundle rooted at e.g. `~/.myapp/codex-bin/<platform>/<version>/codex`; [`resolve_bundled_binary`] resolves that path without ever falling back to `PATH` or `CODEX_BINARY`. Hosts own downloads and version pins; missing bundles are hard errors.
//! - Pair bundled binaries with per-project `CODEX_HOME` roots such as `~/.myapp/codex-homes/<project>/`, optionally seeding `auth.json` + `.credentials.json` from an app-owned seed home. History/logs remain per project; the wrapper still injects `CODEX_BINARY`/`CODEX_HOME` per spawn so the parent env stays untouched.
//! - Default behavior remains unchanged until the helper is used; env/CLI defaults stay as documented above.
//!
//! ```rust,no_run
//! use codex::CodexClient;
//! # use std::time::Duration;
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! std::env::set_var("CODEX_HOME", "/tmp/my-app-codex");
//! let client = CodexClient::builder()
//!     .binary("/opt/myapp/bin/codex")
//!     .model("gpt-5-codex")
//!     .timeout(Duration::from_secs(45))
//!     .build();
//! let reply = client.send_prompt("Health check").await?;
//! println!("{reply}");
//! # Ok(()) }
//! ```
//!
//! Surfaces:
//! - [`CodexClient::send_prompt`] for a single prompt/response with optional `--json` output.
//! - [`CodexClient::stream_exec`] for typed, real-time JSONL events from `codex exec --json`, returning an [`ExecStream`] with an event stream plus a completion future.
//! - [`CodexClient::apply`] / [`CodexClient::diff`] to run `codex apply/diff`, echo stdout/stderr according to the builder (`mirror_stdout` / `quiet`), and return captured output + exit status.
//! - [`CodexClient::generate_app_server_bindings`] to refresh app-server protocol bindings via `codex app-server generate-ts` (optional `--prettier`) or `generate-json-schema`, returning captured stdout/stderr plus the exit status.
//! - [`CodexClient::run_sandbox`] to wrap `codex sandbox <platform>` (macOS/Linux/Windows), pass `--full-auto`/`--log-denials`/`--config`/`--enable`/`--disable`, and return the inner command status + output. macOS is the only platform that emits denial logs; Linux depends on the bundled `codex-linux-sandbox`; Windows sandboxing is experimental and relies on the upstream helper (no capability gating—non-zero exits bubble through).
//! - [`CodexClient::check_execpolicy`] to evaluate shell commands against Starlark execpolicy files with repeatable `--policy` flags, optional pretty JSON, and parsed decision output (allow/prompt/forbidden or noMatch).
//! - [`CodexClient::list_features`] to wrap `codex features list` with optional `--json` parsing, shared config/profile overrides, and parsed feature entries (name/stage/enabled).
//! - [`CodexClient::start_responses_api_proxy`] to launch the `codex responses-api-proxy` helper with an API key piped via stdin plus optional port/server-info/upstream/shutdown flags.
//! - [`CodexClient::stdio_to_uds`] to spawn `codex stdio-to-uds <SOCKET_PATH>` with piped stdio so callers can bridge Unix domain sockets manually.
//!
//! ## Streaming, events, and artifacts
//! - `.json(true)` requests JSONL streaming. Expect `thread.started`/`thread.resumed`, `turn.started`/`turn.completed`/`turn.failed`, and `item.created`/`item.updated` with `item.type` such as `agent_message`, `reasoning`, `command_execution`, `file_change`, `mcp_tool_call`, `web_search`, or `todo_list` plus optional `status`/`content`/`input`. Errors surface as `{"type":"error","message":...}`.
//! - Sample payloads ship with the streaming examples (`crates/codex/examples/fixtures/*`); most examples support `--sample` for offline inspection.
//! - Disable `mirror_stdout` when parsing JSON so stdout stays under caller control; `quiet` controls stderr mirroring. `json_event_log` tees raw JSONL lines to disk before parsing; `idle_timeout`, `output_last_message`, and `output_schema` cover artifact handling.
//! - `crates/codex/examples/stream_events.rs`, `stream_last_message.rs`, `stream_with_log.rs`, and `json_stream.rs` cover typed consumption, artifact handling, log teeing, and minimal streaming.
//!
//! ## Resume + apply/diff
//! - `codex resume --json --skip-git-repo-check --last` (or `--id <conversationId>`) streams the same `thread/turn/item` events as `exec` with an initial `thread.resumed`; reuse the streaming consumers above.
//! - `codex diff --json --skip-git-repo-check` previews staged changes, and `codex apply --json` returns stdout/stderr plus the exit status for the apply step. Streams echo into `file_change` events and any configured JSON log tee.
//! - `crates/codex/examples/resume_apply.rs` strings these together with sample payloads and lets you skip the apply call when you just want the resume stream.
//!
//! ## Servers and capability detection
//! - Integrate the stdio servers via `codex mcp-server` / `codex app-server` (`crates/codex/examples/mcp_codex_flow.rs`, `mcp_codex_tool.rs`, `mcp_codex_reply.rs`, `app_server_turns.rs`, `app_server_thread_turn.rs`) to drive JSON-RPC flows, approvals, and shutdown.
//! - Gate optional flags with `crates/codex/examples/feature_detection.rs`, which parses `codex --version` + `codex features list` to decide whether to enable streaming, log tee, resume/apply/diff helpers, or app-server endpoints. Cache feature probes per binary path and refresh them when the Codex binary path, mtime, or reported version changes; emit upgrade advisories when required capabilities are missing.
//!
//! More end-to-end flows and CLI mappings live in `crates/codex/README.md` and `crates/codex/EXAMPLES.md`.
//!
//! ## Capability/versioning surfaces (Workstream F)
//! - `probe_capabilities` captures `--version`, `features list`, and `--help` hints into a `CodexCapabilities` snapshot with `collected_at` timestamps and `BinaryFingerprint` metadata keyed by canonical binary path.
//! - Guard helpers (`guard_output_schema`, `guard_add_dir`, `guard_mcp_login`, `guard_features_list`) keep optional flags disabled when support is unknown and return operator-facing notes for unsupported features.
//! - Cache controls: `CapabilityCachePolicy::{PreferCache, Refresh, Bypass}` plus builder helpers steer cache reuse. Use `Refresh` for TTL/backoff windows or hot-swaps that reuse the same binary path; use `Bypass` when metadata is missing (FUSE/overlay filesystems) or when you need an isolated probe.
//! - TTL/backoff helper: `capability_cache_ttl_decision` inspects `collected_at` to suggest when to reuse, refresh, or bypass cached snapshots and stretches the recommended policy when metadata is missing.
//! - Overrides + persistence: `capability_snapshot`, `capability_overrides`, `write_capabilities_snapshot`, `read_capabilities_snapshot`, and `capability_snapshot_matches_binary` let hosts reuse snapshots across processes and fall back to probes when fingerprints diverge.

pub mod mcp;

use std::{
    collections::{BTreeMap, HashMap, HashSet},
    env,
    ffi::{OsStr, OsString},
    fs as std_fs,
    future::Future,
    io::{self as stdio, Write},
    path::{Path, PathBuf},
    pin::Pin,
    process::ExitStatus,
    sync::{Mutex, OnceLock},
    task::{Context, Poll},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use futures_core::Stream;
use semver::{Prerelease, Version};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
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
use tracing::{debug, warn};

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(120);
const DEFAULT_REASONING_CONFIG_GPT5: &[(&str, &str)] = &[
    ("model_reasoning_effort", "medium"),
    ("model_reasoning_summary", "auto"),
    ("model_verbosity", "low"),
];

const DEFAULT_REASONING_CONFIG_GPT5_CODEX: &[(&str, &str)] = &[
    ("model_reasoning_effort", "medium"),
    ("model_reasoning_summary", "auto"),
    ("model_verbosity", "low"),
];

const DEFAULT_REASONING_CONFIG_GPT5_1: &[(&str, &str)] = &[
    ("model_reasoning_effort", "medium"),
    ("model_reasoning_summary", "auto"),
    ("model_verbosity", "low"),
];
const CODEX_BINARY_ENV: &str = "CODEX_BINARY";
const CODEX_HOME_ENV: &str = "CODEX_HOME";
const RUST_LOG_ENV: &str = "RUST_LOG";
const DEFAULT_RUST_LOG: &str = "error";

/// Specification for resolving an app-bundled Codex binary.
///
/// Callers supply a bundle root plus the pinned version they expect. Platform
/// defaults to the current target triple label (e.g., `darwin-arm64` or
/// `linux-x64`) but can be overridden when hosts manage their own layout.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BundledBinarySpec<'a> {
    /// Root containing `<platform>/<version>/codex` slices managed by the host.
    pub bundle_root: &'a Path,
    /// Pinned Codex version directory to resolve (semantic version or channel/build id).
    pub version: &'a str,
    /// Optional platform label override; defaults to [`default_bundled_platform_label`].
    pub platform: Option<&'a str>,
}

/// Resolved bundled Codex binary details.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BundledBinary {
    /// Canonicalized path to the bundled Codex binary (`codex` or `codex.exe`).
    pub binary_path: PathBuf,
    /// Platform slice resolved under the bundle root.
    pub platform: String,
    /// Version slice resolved under the platform directory.
    pub version: String,
}

/// Errors that may occur while resolving a bundled Codex binary.
#[derive(Debug, Error)]
pub enum BundledBinaryError {
    #[error("bundled Codex version cannot be empty")]
    EmptyVersion,
    #[error("bundled Codex platform label cannot be empty")]
    EmptyPlatform,
    #[error("bundle root `{bundle_root}` does not exist or is unreadable")]
    BundleRootUnreadable {
        bundle_root: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("bundle root `{bundle_root}` is not a directory")]
    BundleRootNotDirectory { bundle_root: PathBuf },
    #[error("bundle platform directory `{platform_dir}` for `{platform}` does not exist or is unreadable")]
    PlatformUnreadable {
        platform: String,
        platform_dir: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("bundle platform directory `{platform_dir}` for `{platform}` is not a directory")]
    PlatformNotDirectory {
        platform: String,
        platform_dir: PathBuf,
    },
    #[error(
        "bundle version directory `{version_dir}` for `{version}` does not exist or is unreadable"
    )]
    VersionUnreadable {
        version: String,
        version_dir: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("bundle version directory `{version_dir}` for `{version}` is not a directory")]
    VersionNotDirectory {
        version: String,
        version_dir: PathBuf,
    },
    #[error("bundled Codex binary `{binary}` is missing or unreadable")]
    BinaryUnreadable {
        binary: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("bundled Codex binary `{binary}` is not a file")]
    BinaryNotFile { binary: PathBuf },
    #[error("bundled Codex binary `{binary}` is not executable")]
    BinaryNotExecutable { binary: PathBuf },
    #[error("failed to canonicalize bundled Codex binary `{path}`: {source}")]
    Canonicalize {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

/// Resolves a bundled Codex binary under `<bundle_root>/<platform>/<version>/`.
///
/// The helper never consults `PATH` or `CODEX_BINARY`; missing slices are hard
/// errors. The resolved path is canonicalized and should be passed to
/// [`CodexClientBuilder::binary`] to keep behavior isolated from any global
/// Codex install.
pub fn resolve_bundled_binary(
    spec: BundledBinarySpec<'_>,
) -> Result<BundledBinary, BundledBinaryError> {
    let platform = match spec.platform {
        Some(label) => normalize_non_empty(label).ok_or(BundledBinaryError::EmptyPlatform)?,
        None => default_bundled_platform_label(),
    };
    let version = normalize_non_empty(spec.version).ok_or(BundledBinaryError::EmptyVersion)?;

    require_directory(
        spec.bundle_root,
        |source| BundledBinaryError::BundleRootUnreadable {
            bundle_root: spec.bundle_root.to_path_buf(),
            source,
        },
        || BundledBinaryError::BundleRootNotDirectory {
            bundle_root: spec.bundle_root.to_path_buf(),
        },
    )?;

    let platform_dir = spec.bundle_root.join(&platform);
    require_directory(
        &platform_dir,
        |source| BundledBinaryError::PlatformUnreadable {
            platform: platform.clone(),
            platform_dir: platform_dir.clone(),
            source,
        },
        || BundledBinaryError::PlatformNotDirectory {
            platform: platform.clone(),
            platform_dir: platform_dir.clone(),
        },
    )?;

    let version_dir = platform_dir.join(&version);
    require_directory(
        &version_dir,
        |source| BundledBinaryError::VersionUnreadable {
            version: version.clone(),
            version_dir: version_dir.clone(),
            source,
        },
        || BundledBinaryError::VersionNotDirectory {
            version: version.clone(),
            version_dir: version_dir.clone(),
        },
    )?;

    let binary_path = version_dir.join(bundled_binary_filename(&platform));
    let metadata =
        std_fs::metadata(&binary_path).map_err(|source| BundledBinaryError::BinaryUnreadable {
            binary: binary_path.clone(),
            source,
        })?;
    if !metadata.is_file() {
        return Err(BundledBinaryError::BinaryNotFile {
            binary: binary_path.clone(),
        });
    }
    ensure_executable(&metadata, &binary_path)?;

    let canonical =
        std_fs::canonicalize(&binary_path).map_err(|source| BundledBinaryError::Canonicalize {
            path: binary_path.clone(),
            source,
        })?;

    Ok(BundledBinary {
        binary_path: canonical,
        platform,
        version,
    })
}

/// Default bundled platform label for the current target (e.g., `darwin-arm64`, `linux-x64`, `windows-x64`).
pub fn default_bundled_platform_label() -> String {
    let os = match env::consts::OS {
        "macos" => "darwin",
        other => other,
    };
    let arch = match env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "arm64",
        other => other,
    };
    format!("{os}-{arch}")
}

fn require_directory(
    path: &Path,
    on_read_error: impl FnOnce(std::io::Error) -> BundledBinaryError,
    on_wrong_type: impl FnOnce() -> BundledBinaryError,
) -> Result<(), BundledBinaryError> {
    let metadata = std_fs::metadata(path).map_err(on_read_error)?;
    if !metadata.is_dir() {
        return Err(on_wrong_type());
    }
    Ok(())
}

fn ensure_executable(metadata: &std_fs::Metadata, binary: &Path) -> Result<(), BundledBinaryError> {
    if binary_is_executable(metadata) {
        return Ok(());
    }
    Err(BundledBinaryError::BinaryNotExecutable {
        binary: binary.to_path_buf(),
    })
}

fn binary_is_executable(metadata: &std_fs::Metadata) -> bool {
    #[cfg(unix)]
    {
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        // Windows does not use executable bits; existence is sufficient.
        true
    }
}

fn bundled_binary_filename(platform: &str) -> &'static str {
    if platform.to_ascii_lowercase().contains("windows") {
        "codex.exe"
    } else {
        "codex"
    }
}

fn normalize_non_empty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then_some(trimmed.to_string())
}
/// Snapshot of Codex CLI capabilities derived from probing a specific binary.
///
/// Instances of this type are intended to be cached per binary path so callers can
/// gate optional flags (like `--output-schema`) without repeatedly spawning the CLI.
/// A process-wide `HashMap<CapabilityCacheKey, CodexCapabilities>` (behind a mutex/once)
/// keeps probes cheap; entries should use canonical binary paths where possible and
/// ship a [`BinaryFingerprint`] so we can invalidate stale snapshots when the binary
/// on disk changes.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
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
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
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
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
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
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
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
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
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
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
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
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
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
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
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

/// Optional overrides for feature detection that can be layered onto probe results.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct CapabilityFeatureOverrides {
    /// Override for `codex features list` support; `None` defers to probes.
    pub supports_features_list: Option<bool>,
    /// Override for `--output-schema` support; `None` defers to probes.
    pub supports_output_schema: Option<bool>,
    /// Override for `codex add-dir` support; `None` defers to probes.
    pub supports_add_dir: Option<bool>,
    /// Override for `codex login --mcp` support; `None` defers to probes.
    pub supports_mcp_login: Option<bool>,
}

impl CapabilityFeatureOverrides {
    /// Returns true when no overrides are set.
    pub fn is_empty(&self) -> bool {
        self.supports_features_list.is_none()
            && self.supports_output_schema.is_none()
            && self.supports_add_dir.is_none()
            && self.supports_mcp_login.is_none()
    }

    /// Builds overrides that mirror every provided feature flag, including false values.
    pub fn from_flags(flags: CodexFeatureFlags) -> Self {
        CapabilityFeatureOverrides {
            supports_features_list: Some(flags.supports_features_list),
            supports_output_schema: Some(flags.supports_output_schema),
            supports_add_dir: Some(flags.supports_add_dir),
            supports_mcp_login: Some(flags.supports_mcp_login),
        }
    }

    /// Builds overrides that only force-enable flags that are true in the input set.
    pub fn enabling(flags: CodexFeatureFlags) -> Self {
        CapabilityFeatureOverrides {
            supports_features_list: flags.supports_features_list.then_some(true),
            supports_output_schema: flags.supports_output_schema.then_some(true),
            supports_add_dir: flags.supports_add_dir.then_some(true),
            supports_mcp_login: flags.supports_mcp_login.then_some(true),
        }
    }
}

/// Caller-supplied capability data that can short-circuit or adjust probing.
/// Manual snapshots override cached/probed data, and feature/version overrides
/// apply on top of whichever snapshot is returned.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct CapabilityOverrides {
    /// Manual snapshot to return instead of probing when present (after applying feature/version overrides).
    pub snapshot: Option<CodexCapabilities>,
    /// Version override applied after probing.
    pub version: Option<CodexVersionInfo>,
    /// Feature-level overrides merged into probed or manual capabilities.
    pub features: CapabilityFeatureOverrides,
}

impl CapabilityOverrides {
    /// Returns true when no override data is present.
    pub fn is_empty(&self) -> bool {
        self.snapshot.is_none() && self.version.is_none() && self.features.is_empty()
    }
}

/// Supported serialization formats for capability snapshots and overrides.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CapabilitySnapshotFormat {
    Json,
    Toml,
}

impl CapabilitySnapshotFormat {
    fn from_path(path: &Path) -> Option<Self> {
        let ext = path.extension()?.to_string_lossy().to_lowercase();
        match ext.as_str() {
            "json" => Some(Self::Json),
            "toml" => Some(Self::Toml),
            _ => None,
        }
    }
}

/// Errors encountered while saving or loading capability snapshots.
#[derive(Debug, Error)]
pub enum CapabilitySnapshotError {
    #[error("failed to read capability snapshot from `{path}`: {source}")]
    ReadSnapshot {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to write capability snapshot to `{path}`: {source}")]
    WriteSnapshot {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to decode capability snapshot from JSON: {source}")]
    JsonDecode {
        #[source]
        source: serde_json::Error,
    },
    #[error("failed to encode capability snapshot to JSON: {source}")]
    JsonEncode {
        #[source]
        source: serde_json::Error,
    },
    #[error("failed to decode capability snapshot from TOML: {source}")]
    TomlDecode {
        #[source]
        source: toml::de::Error,
    },
    #[error("failed to encode capability snapshot to TOML: {source}")]
    TomlEncode {
        #[source]
        source: toml::ser::Error,
    },
    #[error("unsupported capability snapshot format for `{path}`; use .json/.toml or supply a format explicitly")]
    UnsupportedFormat { path: PathBuf },
}

fn serialize_snapshot<T: Serialize>(
    value: &T,
    format: CapabilitySnapshotFormat,
) -> Result<String, CapabilitySnapshotError> {
    match format {
        CapabilitySnapshotFormat::Json => serde_json::to_string_pretty(value)
            .map_err(|source| CapabilitySnapshotError::JsonEncode { source }),
        CapabilitySnapshotFormat::Toml => toml::to_string_pretty(value)
            .map_err(|source| CapabilitySnapshotError::TomlEncode { source }),
    }
}

fn deserialize_snapshot<T: DeserializeOwned>(
    input: &str,
    format: CapabilitySnapshotFormat,
) -> Result<T, CapabilitySnapshotError> {
    match format {
        CapabilitySnapshotFormat::Json => serde_json::from_str(input)
            .map_err(|source| CapabilitySnapshotError::JsonDecode { source }),
        CapabilitySnapshotFormat::Toml => {
            toml::from_str(input).map_err(|source| CapabilitySnapshotError::TomlDecode { source })
        }
    }
}

fn resolve_snapshot_format(
    format: Option<CapabilitySnapshotFormat>,
    path: &Path,
) -> Result<CapabilitySnapshotFormat, CapabilitySnapshotError> {
    format
        .or_else(|| CapabilitySnapshotFormat::from_path(path))
        .ok_or_else(|| CapabilitySnapshotError::UnsupportedFormat {
            path: path.to_path_buf(),
        })
}

/// Serializes a capability snapshot to a JSON or TOML string.
pub fn serialize_capabilities_snapshot(
    snapshot: &CodexCapabilities,
    format: CapabilitySnapshotFormat,
) -> Result<String, CapabilitySnapshotError> {
    serialize_snapshot(snapshot, format)
}

/// Parses a capability snapshot from serialized JSON or TOML.
pub fn deserialize_capabilities_snapshot(
    input: &str,
    format: CapabilitySnapshotFormat,
) -> Result<CodexCapabilities, CapabilitySnapshotError> {
    deserialize_snapshot(input, format)
}

/// Writes a capability snapshot to disk, inferring format from the file extension when absent.
pub fn write_capabilities_snapshot(
    path: impl AsRef<Path>,
    snapshot: &CodexCapabilities,
    format: Option<CapabilitySnapshotFormat>,
) -> Result<(), CapabilitySnapshotError> {
    let path = path.as_ref();
    let resolved_format = resolve_snapshot_format(format, path)?;
    let contents = serialize_capabilities_snapshot(snapshot, resolved_format)?;
    std_fs::write(path, contents).map_err(|source| CapabilitySnapshotError::WriteSnapshot {
        path: path.to_path_buf(),
        source,
    })
}

/// Loads a capability snapshot from disk, inferring format from the file extension when absent.
pub fn read_capabilities_snapshot(
    path: impl AsRef<Path>,
    format: Option<CapabilitySnapshotFormat>,
) -> Result<CodexCapabilities, CapabilitySnapshotError> {
    let path = path.as_ref();
    let resolved_format = resolve_snapshot_format(format, path)?;
    let contents =
        std_fs::read_to_string(path).map_err(|source| CapabilitySnapshotError::ReadSnapshot {
            path: path.to_path_buf(),
            source,
        })?;
    deserialize_capabilities_snapshot(&contents, resolved_format)
}

/// Serializes capability overrides (snapshot, version, feature flags) to a JSON or TOML string.
pub fn serialize_capability_overrides(
    overrides: &CapabilityOverrides,
    format: CapabilitySnapshotFormat,
) -> Result<String, CapabilitySnapshotError> {
    serialize_snapshot(overrides, format)
}

/// Parses capability overrides from serialized JSON or TOML.
pub fn deserialize_capability_overrides(
    input: &str,
    format: CapabilitySnapshotFormat,
) -> Result<CapabilityOverrides, CapabilitySnapshotError> {
    deserialize_snapshot(input, format)
}

/// Writes capability overrides to disk, inferring format from the file extension when absent.
pub fn write_capability_overrides(
    path: impl AsRef<Path>,
    overrides: &CapabilityOverrides,
    format: Option<CapabilitySnapshotFormat>,
) -> Result<(), CapabilitySnapshotError> {
    let path = path.as_ref();
    let resolved_format = resolve_snapshot_format(format, path)?;
    let contents = serialize_capability_overrides(overrides, resolved_format)?;
    std_fs::write(path, contents).map_err(|source| CapabilitySnapshotError::WriteSnapshot {
        path: path.to_path_buf(),
        source,
    })
}

/// Reads capability overrides from disk, inferring format from the file extension when absent.
pub fn read_capability_overrides(
    path: impl AsRef<Path>,
    format: Option<CapabilitySnapshotFormat>,
) -> Result<CapabilityOverrides, CapabilitySnapshotError> {
    let path = path.as_ref();
    let resolved_format = resolve_snapshot_format(format, path)?;
    let contents =
        std_fs::read_to_string(path).map_err(|source| CapabilitySnapshotError::ReadSnapshot {
            path: path.to_path_buf(),
            source,
        })?;
    deserialize_capability_overrides(&contents, resolved_format)
}

/// True when the snapshot was captured for the same binary path and fingerprint.
///
/// Hosts can consult this before applying a serialized snapshot to avoid
/// reusing stale capability data after binary upgrades.
pub fn capability_snapshot_matches_binary(snapshot: &CodexCapabilities, binary: &Path) -> bool {
    let cache_key = capability_cache_key(binary);
    if snapshot.cache_key != cache_key {
        return false;
    }
    let current = current_fingerprint(&cache_key);
    has_fingerprint_metadata(&snapshot.fingerprint)
        && has_fingerprint_metadata(&current)
        && fingerprints_match(&snapshot.fingerprint, &current)
}

/// Result of applying a TTL/backoff window to a capability snapshot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CapabilityTtlDecision {
    /// True when the snapshot is outside the TTL window and callers should re-run probes.
    pub should_probe: bool,
    /// Recommended cache policy for the next probe (`Refresh` when fingerprints exist, `Bypass` when metadata is missing).
    pub policy: CapabilityCachePolicy,
}

/// Decides whether a cached capability snapshot should be refreshed based on `collected_at`.
///
/// Callers can use this to apply a TTL/backoff in environments where filesystem metadata is
/// missing or unreliable (e.g., FUSE/overlay filesystems) and when binaries are hot-swapped
/// without changing fingerprints. When the TTL has not elapsed, reuse the provided snapshot;
/// when expired, force a probe with [`CapabilityCachePolicy::Refresh`] (fingerprints present)
/// or [`CapabilityCachePolicy::Bypass`] (metadata missing).
///
/// Recommended defaults: start with a 5 minute TTL when fingerprints exist and prefer
/// `Refresh` for hot-swaps that reuse the same path; when metadata is missing, expect `Bypass`
/// and back off further (e.g., stretch the TTL toward 10-15 minutes) to avoid tight probe loops.
pub fn capability_cache_ttl_decision(
    snapshot: Option<&CodexCapabilities>,
    ttl: Duration,
    now: SystemTime,
) -> CapabilityTtlDecision {
    let default_policy = CapabilityCachePolicy::PreferCache;
    let Some(snapshot) = snapshot else {
        return CapabilityTtlDecision {
            should_probe: true,
            policy: default_policy,
        };
    };

    let expired = now
        .duration_since(snapshot.collected_at)
        .map(|elapsed| elapsed >= ttl)
        .unwrap_or(true);

    if !expired {
        return CapabilityTtlDecision {
            should_probe: false,
            policy: default_policy,
        };
    }

    let policy = if snapshot.fingerprint.is_some() {
        CapabilityCachePolicy::Refresh
    } else {
        CapabilityCachePolicy::Bypass
    };

    CapabilityTtlDecision {
        should_probe: true,
        policy,
    }
}

/// High-level view of whether a specific feature can be used safely.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CapabilitySupport {
    Supported,
    Unsupported,
    Unknown,
}

impl CapabilitySupport {
    /// True when it is safe to enable the guarded feature or flag.
    pub const fn is_supported(self) -> bool {
        matches!(self, CapabilitySupport::Supported)
    }

    /// True when support could not be confirmed due to missing probes.
    pub const fn is_unknown(self) -> bool {
        matches!(self, CapabilitySupport::Unknown)
    }
}

/// Feature/flag tokens that can be guarded based on probed capabilities.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CapabilityFeature {
    OutputSchema,
    AddDir,
    McpLogin,
    FeaturesList,
}

impl CapabilityFeature {
    fn label(self) -> &'static str {
        match self {
            CapabilityFeature::OutputSchema => "--output-schema",
            CapabilityFeature::AddDir => "codex add-dir",
            CapabilityFeature::McpLogin => "codex login --mcp",
            CapabilityFeature::FeaturesList => "codex features list",
        }
    }
}

/// Result of gating a Codex feature/flag against probed capabilities.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilityGuard {
    /// Feature being checked.
    pub feature: CapabilityFeature,
    /// Whether the feature is safe to enable.
    pub support: CapabilitySupport,
    /// Notes explaining how the guard was derived.
    pub notes: Vec<String>,
}

impl CapabilityGuard {
    fn supported(feature: CapabilityFeature, note: impl Into<String>) -> Self {
        CapabilityGuard {
            feature,
            support: CapabilitySupport::Supported,
            notes: vec![note.into()],
        }
    }

    fn unsupported(feature: CapabilityFeature, note: impl Into<String>) -> Self {
        CapabilityGuard {
            feature,
            support: CapabilitySupport::Unsupported,
            notes: vec![note.into()],
        }
    }

    fn unknown(feature: CapabilityFeature, notes: Vec<String>) -> Self {
        CapabilityGuard {
            feature,
            support: CapabilitySupport::Unknown,
            notes,
        }
    }

    /// Convenience wrapper for `support.is_supported()`.
    pub const fn is_supported(&self) -> bool {
        self.support.is_supported()
    }

    /// Convenience wrapper for `support.is_unknown()`.
    pub const fn is_unknown(&self) -> bool {
        self.support.is_unknown()
    }
}

/// Description of how we interrogate the CLI to populate a [`CodexCapabilities`] snapshot.
///
/// Probes should prefer an explicit feature list when available, fall back to parsing
/// `codex --help` flags, and finally rely on coarse version heuristics. Each attempted
/// step is recorded so hosts can trace why a particular flag was enabled or skipped.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct CapabilityProbePlan {
    /// Steps attempted in order; consumers should push entries as probes run.
    pub steps: Vec<CapabilityProbeStep>,
}

/// Command-level probes used to infer feature support.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum CapabilityProbeStep {
    /// Invoke `codex --version` to capture version/build metadata.
    VersionFlag,
    /// Prefer `codex features list --json` when supported for structured output.
    FeaturesListJson,
    /// Fallback to `codex features list` when only plain text is available.
    FeaturesListText,
    /// Parse `codex --help` to spot known flags (e.g., `--output-schema`, `add-dir`, `login --mcp`) when the features list is missing.
    HelpFallback,
    /// Caller-supplied capability overrides were applied to the snapshot.
    ManualOverride,
}

impl CodexCapabilities {
    /// Guards whether `--output-schema` should be passed to `codex exec`.
    pub fn guard_output_schema(&self) -> CapabilityGuard {
        self.guard_feature(CapabilityFeature::OutputSchema)
    }

    /// Guards whether `codex add-dir` can be invoked safely.
    pub fn guard_add_dir(&self) -> CapabilityGuard {
        self.guard_feature(CapabilityFeature::AddDir)
    }

    /// Guards whether `codex login --mcp` is available.
    pub fn guard_mcp_login(&self) -> CapabilityGuard {
        self.guard_feature(CapabilityFeature::McpLogin)
    }

    /// Guards whether `codex features list` is supported by the probed binary.
    pub fn guard_features_list(&self) -> CapabilityGuard {
        self.guard_feature(CapabilityFeature::FeaturesList)
    }

    /// Returns a guard describing if a feature/flag is supported by the probed binary.
    ///
    /// The guard treats missing `features list` support as `Unknown` so hosts can
    /// degrade gracefully on older binaries instead of passing unsupported flags.
    pub fn guard_feature(&self, feature: CapabilityFeature) -> CapabilityGuard {
        guard_feature_support(feature, &self.features, self.version.as_ref())
    }
}

fn guard_feature_support(
    feature: CapabilityFeature,
    flags: &CodexFeatureFlags,
    version: Option<&CodexVersionInfo>,
) -> CapabilityGuard {
    let supported = match feature {
        CapabilityFeature::OutputSchema => flags.supports_output_schema,
        CapabilityFeature::AddDir => flags.supports_add_dir,
        CapabilityFeature::McpLogin => flags.supports_mcp_login,
        CapabilityFeature::FeaturesList => flags.supports_features_list,
    };

    if supported {
        return CapabilityGuard::supported(
            feature,
            format!("Support for {} reported by Codex probe.", feature.label()),
        );
    }

    if feature == CapabilityFeature::FeaturesList {
        let mut notes = vec![format!(
            "Support for {} could not be confirmed; feature list probes failed or were unavailable.",
            feature.label()
        )];
        if version.is_none() {
            notes.push(
                "Version was unavailable; assuming compatibility with older Codex builds."
                    .to_string(),
            );
        }
        return CapabilityGuard::unknown(feature, notes);
    }

    if flags.supports_features_list {
        return CapabilityGuard::unsupported(
            feature,
            format!(
                "`{}` did not advertise {}; skipping related flag to stay compatible.",
                CapabilityFeature::FeaturesList.label(),
                feature.label()
            ),
        );
    }

    let mut notes = vec![format!(
        "Support for {} is unknown because {} is unavailable; disable the flag for compatibility.",
        feature.label(),
        CapabilityFeature::FeaturesList.label()
    )];
    if version.is_none() {
        notes.push(
            "Version could not be parsed; treating feature support conservatively to avoid CLI errors."
                .to_string(),
        );
    }

    CapabilityGuard::unknown(feature, notes)
}

fn guard_is_supported(guard: &CapabilityGuard) -> bool {
    matches!(guard.support, CapabilitySupport::Supported)
}

fn log_guard_skip(guard: &CapabilityGuard) {
    warn!(
        feature = guard.feature.label(),
        support = ?guard.support,
        notes = ?guard.notes,
        "Skipping requested Codex capability because support was not confirmed"
    );
}

/// Cache interaction policy for capability probes.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CapabilityCachePolicy {
    /// Use cached entries when fingerprints match; fall back to probing when
    /// fingerprints differ or are missing and write fresh snapshots back.
    #[default]
    PreferCache,
    /// Always run probes, overwriting any existing cache entry for the binary (useful for TTL/backoff windows or hot-swaps that keep the same path).
    Refresh,
    /// Skip cache reads and writes to force an isolated snapshot.
    Bypass,
}

/// Cache key for capability snapshots derived from a specific Codex binary path.
///
/// Cache lookups should canonicalize the path when possible so symlinked binaries
/// collapse to a single entry.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct CapabilityCacheKey {
    /// Canonical binary path when resolvable; otherwise the original path.
    pub binary_path: PathBuf,
}

/// File metadata used to invalidate cached capability snapshots when the binary changes.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
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
    let canonical = std_fs::canonicalize(binary).unwrap_or_else(|_| binary.to_path_buf());
    CapabilityCacheKey {
        binary_path: canonical,
    }
}

fn has_fingerprint_metadata(fingerprint: &Option<BinaryFingerprint>) -> bool {
    fingerprint.is_some()
}

fn cached_capabilities(
    key: &CapabilityCacheKey,
    fingerprint: &Option<BinaryFingerprint>,
) -> Option<CodexCapabilities> {
    let cache = capability_cache().lock().ok()?;
    let cached = cache.get(key)?;
    if !has_fingerprint_metadata(&cached.fingerprint) || !has_fingerprint_metadata(fingerprint) {
        return None;
    }
    if fingerprints_match(&cached.fingerprint, fingerprint) {
        Some(cached.clone())
    } else {
        None
    }
}

fn update_capability_cache(capabilities: CodexCapabilities) {
    if !has_fingerprint_metadata(&capabilities.fingerprint) {
        return;
    }
    if let Ok(mut cache) = capability_cache().lock() {
        cache.insert(capabilities.cache_key.clone(), capabilities);
    }
}

/// Returns all capability cache entries keyed by canonical binary path.
pub fn capability_cache_entries() -> Vec<CodexCapabilities> {
    capability_cache()
        .lock()
        .map(|cache| cache.values().cloned().collect())
        .unwrap_or_default()
}

/// Returns the cached capabilities for a specific binary path if present.
pub fn capability_cache_entry(binary: &Path) -> Option<CodexCapabilities> {
    let key = capability_cache_key(binary);
    capability_cache()
        .lock()
        .ok()
        .and_then(|cache| cache.get(&key).cloned())
}

/// Removes the cached capabilities for a specific binary. Returns true when an entry was removed.
pub fn clear_capability_cache_entry(binary: &Path) -> bool {
    let key = capability_cache_key(binary);
    capability_cache()
        .lock()
        .ok()
        .map(|mut cache| cache.remove(&key).is_some())
        .unwrap_or(false)
}

/// Clears all cached capability snapshots.
pub fn clear_capability_cache() {
    if let Ok(mut cache) = capability_cache().lock() {
        cache.clear();
    }
}

fn current_fingerprint(key: &CapabilityCacheKey) -> Option<BinaryFingerprint> {
    let canonical = std_fs::canonicalize(&key.binary_path).ok();
    let metadata_path = canonical.as_deref().unwrap_or(key.binary_path.as_path());
    let metadata = std_fs::metadata(metadata_path).ok()?;
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

fn finalize_capabilities_with_overrides(
    mut capabilities: CodexCapabilities,
    overrides: &CapabilityOverrides,
    cache_key: CapabilityCacheKey,
    fingerprint: Option<BinaryFingerprint>,
    manual_source: bool,
) -> CodexCapabilities {
    capabilities.cache_key = cache_key;
    capabilities.fingerprint = fingerprint;

    let mut applied = manual_source;

    if let Some(version) = overrides.version.clone() {
        capabilities.version = Some(version);
        applied = true;
    }

    if apply_feature_overrides(&mut capabilities.features, &overrides.features) {
        applied = true;
    }

    if applied
        && !capabilities
            .probe_plan
            .steps
            .contains(&CapabilityProbeStep::ManualOverride)
    {
        capabilities
            .probe_plan
            .steps
            .push(CapabilityProbeStep::ManualOverride);
    }

    capabilities
}

fn apply_feature_overrides(
    features: &mut CodexFeatureFlags,
    overrides: &CapabilityFeatureOverrides,
) -> bool {
    let mut applied = false;

    if let Some(value) = overrides.supports_features_list {
        features.supports_features_list = value;
        applied = true;
    }

    if let Some(value) = overrides.supports_output_schema {
        features.supports_output_schema = value;
        applied = true;
    }

    if let Some(value) = overrides.supports_add_dir {
        features.supports_add_dir = value;
        applied = true;
    }

    if let Some(value) = overrides.supports_mcp_login {
        features.supports_mcp_login = value;
        applied = true;
    }

    applied
}

/// High-level client for interacting with `codex exec`.
///
/// Spawns the CLI with safe defaults (`--skip-git-repo-check`, temp working dirs unless
/// `working_dir` is set, 120s timeout unless zero, ANSI colors off, `RUST_LOG=error` if unset),
/// mirrors stdout by default, and returns whatever the CLI printed. See the crate docs for
/// streaming/log tee/server patterns and example links.
#[derive(Clone, Debug)]
pub struct CodexClient {
    command_env: CommandEnvironment,
    model: Option<String>,
    timeout: Duration,
    color_mode: ColorMode,
    working_dir: Option<PathBuf>,
    add_dirs: Vec<PathBuf>,
    images: Vec<PathBuf>,
    json_output: bool,
    output_schema: bool,
    quiet: bool,
    mirror_stdout: bool,
    json_event_log: Option<PathBuf>,
    cli_overrides: CliOverrides,
    capability_overrides: CapabilityOverrides,
    capability_cache_policy: CapabilityCachePolicy,
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
    ApiKey {
        masked_key: Option<String>,
    },
    /// CLI reported a logged-in state but the auth method could not be parsed (e.g., new wording).
    Unknown {
        raw: String,
    },
}

/// Result of invoking `codex logout`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CodexLogoutStatus {
    LoggedOut,
    AlreadyLoggedOut,
}

/// Helper for checking Codex auth state and triggering login flows with an app-scoped `CODEX_HOME`.
///
/// All commands run with per-process env overrides; the parent process env is never mutated.
#[derive(Clone, Debug)]
pub struct AuthSessionHelper {
    client: CodexClient,
}

impl AuthSessionHelper {
    /// Creates a helper that pins `CODEX_HOME` to `app_codex_home` for every login call.
    pub fn new(app_codex_home: impl Into<PathBuf>) -> Self {
        let client = CodexClient::builder()
            .codex_home(app_codex_home)
            .create_home_dirs(true)
            .build();
        Self { client }
    }

    /// Wraps an existing `CodexClient` (useful when you already configured the binary path).
    pub fn with_client(client: CodexClient) -> Self {
        Self { client }
    }

    /// Returns the underlying `CodexClient`.
    pub fn client(&self) -> CodexClient {
        self.client.clone()
    }

    /// Reports the current login status under the configured `CODEX_HOME`.
    pub async fn status(&self) -> Result<CodexAuthStatus, CodexError> {
        self.client.login_status().await
    }

    /// Logs in with an API key when logged out; otherwise returns the current status.
    pub async fn ensure_api_key_login(
        &self,
        api_key: impl AsRef<str>,
    ) -> Result<CodexAuthStatus, CodexError> {
        match self.status().await? {
            logged @ CodexAuthStatus::LoggedIn(_) => Ok(logged),
            CodexAuthStatus::LoggedOut => self.client.login_with_api_key(api_key).await,
        }
    }

    /// Starts the ChatGPT OAuth login flow when no credentials are present.
    ///
    /// Returns `Ok(None)` when already logged in; otherwise returns the spawned login child so the
    /// caller can surface output/URLs. Dropping the child kills the login helper.
    pub async fn ensure_chatgpt_login(&self) -> Result<Option<tokio::process::Child>, CodexError> {
        match self.status().await? {
            CodexAuthStatus::LoggedIn(_) => Ok(None),
            CodexAuthStatus::LoggedOut => self.client.spawn_login_process().map(Some),
        }
    }

    /// Directly spawns the ChatGPT login process.
    pub fn spawn_chatgpt_login(&self) -> Result<tokio::process::Child, CodexError> {
        self.client.spawn_login_process()
    }

    /// Directly logs in with an API key without checking prior state.
    pub async fn login_with_api_key(
        &self,
        api_key: impl AsRef<str>,
    ) -> Result<CodexAuthStatus, CodexError> {
        self.client.login_with_api_key(api_key).await
    }
}

impl CodexClient {
    /// Returns a [`CodexClientBuilder`] preloaded with safe defaults.
    pub fn builder() -> CodexClientBuilder {
        CodexClientBuilder::default()
    }

    /// Returns the configured `CODEX_HOME` layout, if one was provided.
    /// This does not create any directories on disk; pair with
    /// [`CodexClientBuilder::create_home_dirs`] to control materialization.
    pub fn codex_home_layout(&self) -> Option<CodexHomeLayout> {
        self.command_env.codex_home_layout()
    }

    /// Sends `prompt` to `codex exec` and returns its stdout (the final agent message) on success.
    ///
    /// When `.json(true)` is enabled the CLI emits JSONL events (`thread.started` or
    /// `thread.resumed`, `turn.started`/`turn.completed`/`turn.failed`,
    /// `item.created`/`item.updated`, or `error`). The stream is mirrored to stdout unless
    /// `.mirror_stdout(false)`; the returned string contains the buffered lines for offline
    /// parsing. For per-event handling, see `crates/codex/examples/stream_events.rs`.
    ///
    /// ```rust,no_run
    /// use codex::CodexClient;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = CodexClient::builder().json(true).mirror_stdout(false).build();
    /// let jsonl = client.send_prompt("Stream repo status").await?;
    /// println!("{jsonl}");
    /// # Ok(()) }
    /// ```
    pub async fn send_prompt(&self, prompt: impl AsRef<str>) -> Result<String, CodexError> {
        self.send_prompt_with(ExecRequest::new(prompt.as_ref()))
            .await
    }

    /// Sends an exec request with per-call CLI overrides.
    pub async fn send_prompt_with(&self, request: ExecRequest) -> Result<String, CodexError> {
        if request.prompt.trim().is_empty() {
            return Err(CodexError::EmptyPrompt);
        }

        self.invoke_codex_exec(request).await
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
        self.stream_exec_with_overrides(request, CliOverridesPatch::default())
            .await
    }

    /// Streams JSONL events with per-request CLI overrides.
    pub async fn stream_exec_with_overrides(
        &self,
        request: ExecStreamRequest,
        overrides: CliOverridesPatch,
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
        let needs_capabilities = output_schema.is_some() || !self.add_dirs.is_empty();
        let capabilities = if needs_capabilities {
            Some(self.probe_capabilities().await)
        } else {
            None
        };
        let resolved_overrides =
            resolve_cli_overrides(&self.cli_overrides, &overrides, self.model.as_deref());

        let mut command = Command::new(self.command_env.binary_path());
        command
            .arg("exec")
            .arg("--color")
            .arg(self.color_mode.as_str())
            .arg("--skip-git-repo-check")
            .arg("--json")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .stdin(std::process::Stdio::piped())
            .kill_on_drop(true)
            .current_dir(&dir_path);

        apply_cli_overrides(&mut command, &resolved_overrides, true);

        if let Some(model) = &self.model {
            command.arg("--model").arg(model);
        }

        if let Some(capabilities) = &capabilities {
            if !self.add_dirs.is_empty() {
                let guard = capabilities.guard_add_dir();
                if guard_is_supported(&guard) {
                    for dir in &self.add_dirs {
                        command.arg("--add-dir").arg(dir);
                    }
                } else {
                    log_guard_skip(&guard);
                }
            }
        }

        for image in &self.images {
            command.arg("--image").arg(image);
        }

        command.arg("--output-last-message").arg(&last_message_path);

        if let Some(schema_path) = &output_schema {
            if let Some(capabilities) = &capabilities {
                let guard = capabilities.guard_output_schema();
                if guard_is_supported(&guard) {
                    command.arg("--output-schema").arg(schema_path);
                } else {
                    log_guard_skip(&guard);
                }
            } else {
                command.arg("--output-schema").arg(schema_path);
            }
        }

        self.command_env.apply(&mut command)?;

        let mut child = spawn_with_retry(&mut command, self.command_env.binary_path())?;

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

    /// Streams structured events from `codex resume --json`.
    pub async fn stream_resume(
        &self,
        request: ResumeRequest,
    ) -> Result<ExecStream, ExecStreamError> {
        if let Some(prompt) = &request.prompt {
            if prompt.trim().is_empty() {
                return Err(CodexError::EmptyPrompt.into());
            }
        }

        let ResumeRequest {
            selector,
            prompt,
            idle_timeout,
            output_last_message,
            output_schema,
            json_event_log,
            overrides,
        } = request;

        let dir_ctx = self.directory_context()?;
        let dir_path = dir_ctx.path().to_path_buf();
        let last_message_path =
            output_last_message.unwrap_or_else(|| unique_temp_path("codex_last_message_", "txt"));
        let needs_capabilities = output_schema.is_some() || !self.add_dirs.is_empty();
        let capabilities = if needs_capabilities {
            Some(self.probe_capabilities().await)
        } else {
            None
        };
        let resolved_overrides =
            resolve_cli_overrides(&self.cli_overrides, &overrides, self.model.as_deref());

        let mut command = Command::new(self.command_env.binary_path());
        command
            .arg("resume")
            .arg("--color")
            .arg(self.color_mode.as_str())
            .arg("--skip-git-repo-check")
            .arg("--json")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .stdin(std::process::Stdio::piped())
            .kill_on_drop(true)
            .current_dir(&dir_path);

        apply_cli_overrides(&mut command, &resolved_overrides, true);

        match selector {
            ResumeSelector::Id(id) => {
                command.arg(id);
            }
            ResumeSelector::Last => {
                command.arg("--last");
            }
            ResumeSelector::All => {
                command.arg("--all");
            }
        }

        if let Some(model) = &self.model {
            command.arg("--model").arg(model);
        }

        if let Some(capabilities) = &capabilities {
            if !self.add_dirs.is_empty() {
                let guard = capabilities.guard_add_dir();
                if guard_is_supported(&guard) {
                    for dir in &self.add_dirs {
                        command.arg("--add-dir").arg(dir);
                    }
                } else {
                    log_guard_skip(&guard);
                }
            }
        }

        command.arg("--output-last-message").arg(&last_message_path);

        if let Some(schema_path) = &output_schema {
            if let Some(capabilities) = &capabilities {
                let guard = capabilities.guard_output_schema();
                if guard_is_supported(&guard) {
                    command.arg("--output-schema").arg(schema_path);
                } else {
                    log_guard_skip(&guard);
                }
            } else {
                command.arg("--output-schema").arg(schema_path);
            }
        }

        self.command_env.apply(&mut command)?;

        let mut child = spawn_with_retry(&mut command, self.command_env.binary_path())?;

        if let Some(prompt) = &prompt {
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
        let mut command = Command::new(self.command_env.binary_path());
        command
            .arg("login")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);

        self.command_env.apply(&mut command)?;

        spawn_with_retry(&mut command, self.command_env.binary_path())
    }

    /// Spawns `codex login --mcp` when the probed binary advertises support.
    ///
    /// Returns `Ok(None)` when the capability is unknown or unsupported so
    /// callers can degrade gracefully without attempting the flag.
    pub async fn spawn_mcp_login_process(
        &self,
    ) -> Result<Option<tokio::process::Child>, CodexError> {
        let capabilities = self.probe_capabilities().await;
        let guard = capabilities.guard_mcp_login();
        if !guard_is_supported(&guard) {
            log_guard_skip(&guard);
            return Ok(None);
        }

        let mut command = Command::new(self.command_env.binary_path());
        command
            .arg("login")
            .arg("--mcp")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);

        self.command_env.apply(&mut command)?;

        let child = spawn_with_retry(&mut command, self.command_env.binary_path())?;

        Ok(Some(child))
    }

    /// Logs in with a provided API key by invoking `codex login --api-key <key>`.
    pub async fn login_with_api_key(
        &self,
        api_key: impl AsRef<str>,
    ) -> Result<CodexAuthStatus, CodexError> {
        let api_key = api_key.as_ref().trim();
        if api_key.is_empty() {
            return Err(CodexError::EmptyApiKey);
        }

        let output = self
            .run_basic_command(["login", "--api-key", api_key])
            .await?;
        let combined = preferred_output_channel(&output);

        if output.status.success() {
            Ok(parse_login_success(&combined).unwrap_or_else(|| {
                CodexAuthStatus::LoggedIn(CodexAuthMethod::Unknown {
                    raw: combined.clone(),
                })
            }))
        } else {
            Err(CodexError::NonZeroExit {
                status: output.status,
                stderr: combined,
            })
        }
    }

    /// Returns the current Codex authentication state by invoking `codex login status`.
    pub async fn login_status(&self) -> Result<CodexAuthStatus, CodexError> {
        let output = self.run_basic_command(["login", "status"]).await?;
        let combined = preferred_output_channel(&output);

        if output.status.success() {
            Ok(parse_login_success(&combined).unwrap_or_else(|| {
                CodexAuthStatus::LoggedIn(CodexAuthMethod::Unknown {
                    raw: combined.clone(),
                })
            }))
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
        let combined = preferred_output_channel(&output);

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

    /// Applies the most recent Codex diff by invoking `codex apply`.
    ///
    /// Stdout mirrors to the console when `mirror_stdout` is enabled; stderr mirrors unless `quiet`
    /// is set. Output and exit status are always captured and returned, and `RUST_LOG=error` is
    /// injected for the child process when the environment variable is unset.
    pub async fn apply(&self) -> Result<ApplyDiffArtifacts, CodexError> {
        self.apply_or_diff("apply").await
    }

    /// Shows the most recent Codex diff by invoking `codex diff`.
    ///
    /// Mirrors stdout/stderr using the same `mirror_stdout`/`quiet` defaults as `apply`, but always
    /// returns the captured output alongside the child exit status. Applies the same `RUST_LOG`
    /// defaulting behavior when the variable is unset.
    pub async fn diff(&self) -> Result<ApplyDiffArtifacts, CodexError> {
        self.apply_or_diff("diff").await
    }

    async fn apply_or_diff(&self, subcommand: &str) -> Result<ApplyDiffArtifacts, CodexError> {
        let dir_ctx = self.directory_context()?;
        let resolved_overrides = resolve_cli_overrides(
            &self.cli_overrides,
            &CliOverridesPatch::default(),
            self.model.as_deref(),
        );

        let mut command = Command::new(self.command_env.binary_path());
        command
            .arg(subcommand)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .current_dir(dir_ctx.path());

        apply_cli_overrides(&mut command, &resolved_overrides, false);
        self.command_env.apply(&mut command)?;

        let mut child = spawn_with_retry(&mut command, self.command_env.binary_path())?;

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

        Ok(ApplyDiffArtifacts {
            status,
            stdout: String::from_utf8(stdout_bytes)?,
            stderr: String::from_utf8(stderr_bytes)?,
        })
    }

    /// Generates app-server bindings via `codex app-server generate-ts` or `generate-json-schema`.
    ///
    /// Ensures the output directory exists, mirrors stdout/stderr according to the builder
    /// (`mirror_stdout` / `quiet`), and returns captured output plus the exit status. Non-zero
    /// exits bubble up as [`CodexError::NonZeroExit`] with stderr attached. Use
    /// [`AppServerCodegenRequest::prettier`] to format TypeScript output with a specific
    /// Prettier binary and request-level overrides for config/profile toggles.
    pub async fn generate_app_server_bindings(
        &self,
        request: AppServerCodegenRequest,
    ) -> Result<AppServerCodegenOutput, CodexError> {
        let AppServerCodegenRequest {
            target,
            out_dir,
            overrides,
        } = request;

        std_fs::create_dir_all(&out_dir).map_err(|source| CodexError::PrepareOutputDirectory {
            path: out_dir.clone(),
            source,
        })?;

        let dir_ctx = self.directory_context()?;
        let resolved_overrides =
            resolve_cli_overrides(&self.cli_overrides, &overrides, self.model.as_deref());

        let mut command = Command::new(self.command_env.binary_path());
        command
            .arg("app-server")
            .arg(target.subcommand())
            .arg("--out")
            .arg(&out_dir)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .current_dir(dir_ctx.path());

        apply_cli_overrides(&mut command, &resolved_overrides, true);

        if let Some(prettier) = target.prettier() {
            command.arg("--prettier").arg(prettier);
        }

        self.command_env.apply(&mut command)?;

        let mut child = spawn_with_retry(&mut command, self.command_env.binary_path())?;

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

        if !status.success() {
            return Err(CodexError::NonZeroExit {
                status,
                stderr: String::from_utf8(stderr_bytes)?,
            });
        }

        Ok(AppServerCodegenOutput {
            status,
            stdout: String::from_utf8(stdout_bytes)?,
            stderr: String::from_utf8(stderr_bytes)?,
            out_dir,
        })
    }

    /// Lists CLI features via `codex features list`.
    ///
    /// Requests JSON output when `json(true)` is set and falls back to parsing the text table when
    /// JSON is unavailable. Shared config/profile/search/approval overrides flow through via the
    /// request/builder, stdout/stderr are mirrored according to the builder, and non-zero exits
    /// surface as [`CodexError::NonZeroExit`].
    pub async fn list_features(
        &self,
        request: FeaturesListRequest,
    ) -> Result<FeaturesListOutput, CodexError> {
        let FeaturesListRequest { json, overrides } = request;

        let dir_ctx = self.directory_context()?;
        let resolved_overrides =
            resolve_cli_overrides(&self.cli_overrides, &overrides, self.model.as_deref());

        let mut command = Command::new(self.command_env.binary_path());
        command
            .arg("features")
            .arg("list")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .current_dir(dir_ctx.path());

        apply_cli_overrides(&mut command, &resolved_overrides, true);

        if json {
            command.arg("--json");
        }

        self.command_env.apply(&mut command)?;

        let mut child = spawn_with_retry(&mut command, self.command_env.binary_path())?;

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

        if !status.success() {
            return Err(CodexError::NonZeroExit {
                status,
                stderr: String::from_utf8(stderr_bytes)?,
            });
        }

        let stdout_string = String::from_utf8(stdout_bytes)?;
        let stderr_string = String::from_utf8(stderr_bytes)?;
        let (features, format) =
            parse_feature_list_output(&stdout_string, json).map_err(|reason| {
                CodexError::FeatureListParse {
                    reason,
                    stdout: stdout_string.clone(),
                }
            })?;

        Ok(FeaturesListOutput {
            status,
            stdout: stdout_string,
            stderr: stderr_string,
            features,
            format,
        })
    }

    /// Starts the `codex responses-api-proxy` helper with a supplied API key.
    ///
    /// Forwards optional `--port`, `--server-info`, `--http-shutdown`, and `--upstream-url` flags.
    /// The API key is written to stdin immediately after spawn, stdout/stderr remain piped for callers
    /// to drain, and the returned handle owns the child process plus any `--server-info` path used.
    pub async fn start_responses_api_proxy(
        &self,
        request: ResponsesApiProxyRequest,
    ) -> Result<ResponsesApiProxyHandle, CodexError> {
        let ResponsesApiProxyRequest {
            api_key,
            port,
            server_info_path,
            http_shutdown,
            upstream_url,
        } = request;

        let api_key = api_key.trim().to_string();
        if api_key.is_empty() {
            return Err(CodexError::EmptyApiKey);
        }

        let working_dir = self.sandbox_working_dir(None)?;

        let mut command = Command::new(self.command_env.binary_path());
        command
            .arg("responses-api-proxy")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .current_dir(&working_dir);

        if let Some(port) = port {
            command.arg("--port").arg(port.to_string());
        }

        if let Some(path) = server_info_path.as_ref() {
            command.arg("--server-info").arg(path);
        }

        if http_shutdown {
            command.arg("--http-shutdown");
        }

        if let Some(url) = upstream_url.as_ref() {
            if !url.trim().is_empty() {
                command.arg("--upstream-url").arg(url);
            }
        }

        self.command_env.apply(&mut command)?;

        let mut child = spawn_with_retry(&mut command, self.command_env.binary_path())?;

        let mut stdin = child.stdin.take().ok_or(CodexError::StdinUnavailable)?;
        stdin
            .write_all(api_key.as_bytes())
            .await
            .map_err(CodexError::StdinWrite)?;
        stdin
            .write_all(b"\n")
            .await
            .map_err(CodexError::StdinWrite)?;
        stdin.shutdown().await.map_err(CodexError::StdinWrite)?;

        Ok(ResponsesApiProxyHandle {
            child,
            server_info_path,
        })
    }

    /// Spawns `codex stdio-to-uds <SOCKET_PATH>` with piped stdio for manual relays.
    ///
    /// Returns the child process so callers can write to stdin/read from stdout (e.g., to bridge a
    /// JSON-RPC transport over a Unix domain socket). Fails fast on empty socket paths and inherits
    /// the builder working directory when none is provided on the request.
    pub fn stdio_to_uds(
        &self,
        request: StdioToUdsRequest,
    ) -> Result<tokio::process::Child, CodexError> {
        let StdioToUdsRequest {
            socket_path,
            working_dir,
        } = request;

        if socket_path.as_os_str().is_empty() {
            return Err(CodexError::EmptySocketPath);
        }

        let mut command = Command::new(self.command_env.binary_path());
        command
            .arg("stdio-to-uds")
            .arg(&socket_path)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .current_dir(self.sandbox_working_dir(working_dir)?);

        self.command_env.apply(&mut command)?;

        spawn_with_retry(&mut command, self.command_env.binary_path())
    }

    /// Runs `codex sandbox <platform> [--full-auto|--log-denials] [--config/--enable/--disable] -- <COMMAND...>`.
    ///
    /// Captures stdout/stderr and mirrors them according to the builder (`mirror_stdout` / `quiet`). Unlike
    /// `apply`/`diff`, non-zero exit codes are returned in [`SandboxRun::status`] without being wrapped in
    /// [`CodexError::NonZeroExit`]. macOS denial logging is enabled via [`SandboxCommandRequest::log_denials`]
    /// and ignored on other platforms. Linux uses the bundled `codex-linux-sandbox` helper; Windows sandboxing
    /// is experimental and relies on the upstream helper. The wrapper does not gate availability—unsupported
    /// installs will surface as non-zero statuses.
    pub async fn run_sandbox(
        &self,
        request: SandboxCommandRequest,
    ) -> Result<SandboxRun, CodexError> {
        if request.command.is_empty() {
            return Err(CodexError::EmptySandboxCommand);
        }

        let SandboxCommandRequest {
            platform,
            command,
            full_auto,
            log_denials,
            config_overrides,
            feature_toggles,
            working_dir,
        } = request;

        let working_dir = self.sandbox_working_dir(working_dir)?;

        let mut process = Command::new(self.command_env.binary_path());
        process
            .arg("sandbox")
            .arg(platform.subcommand())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .current_dir(&working_dir);

        if full_auto {
            process.arg("--full-auto");
        }

        if log_denials && matches!(platform, SandboxPlatform::Macos) {
            process.arg("--log-denials");
        }

        for override_ in config_overrides {
            process.arg("--config");
            process.arg(format!("{}={}", override_.key, override_.value));
        }

        for feature in feature_toggles.enable {
            process.arg("--enable");
            process.arg(feature);
        }

        for feature in feature_toggles.disable {
            process.arg("--disable");
            process.arg(feature);
        }

        process.arg("--");
        process.args(&command);

        self.command_env.apply(&mut process)?;

        let mut child = spawn_with_retry(&mut process, self.command_env.binary_path())?;

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

        Ok(SandboxRun {
            status,
            stdout: String::from_utf8(stdout_bytes)?,
            stderr: String::from_utf8(stderr_bytes)?,
        })
    }

    /// Evaluates a command against Starlark execpolicy files via `codex execpolicy check`.
    ///
    /// Forwards repeatable `--policy` paths, optional `--pretty`, and builder/request CLI overrides
    /// (config/profile/approval/sandbox/local-provider/cd/search). Captures stdout/stderr according to the
    /// builder, returns parsed JSON, and surfaces non-zero exits as [`CodexError::NonZeroExit`].
    /// Empty command argv returns [`CodexError::EmptyExecPolicyCommand`].
    pub async fn check_execpolicy(
        &self,
        request: ExecPolicyCheckRequest,
    ) -> Result<ExecPolicyCheckResult, CodexError> {
        if request.command.is_empty() {
            return Err(CodexError::EmptyExecPolicyCommand);
        }

        let ExecPolicyCheckRequest {
            policies,
            pretty,
            command,
            overrides,
        } = request;

        let dir_ctx = self.directory_context()?;
        let resolved_overrides =
            resolve_cli_overrides(&self.cli_overrides, &overrides, self.model.as_deref());

        let mut process = Command::new(self.command_env.binary_path());
        process
            .arg("execpolicy")
            .arg("check")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .current_dir(dir_ctx.path());

        for policy in policies {
            process.arg("--policy").arg(policy);
        }

        if pretty {
            process.arg("--pretty");
        }

        apply_cli_overrides(&mut process, &resolved_overrides, true);

        process.arg("--");
        process.args(&command);

        self.command_env.apply(&mut process)?;

        let mut child = spawn_with_retry(&mut process, self.command_env.binary_path())?;

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

        let stdout_string = String::from_utf8(stdout_bytes)?;
        let stderr_string = String::from_utf8(stderr_bytes)?;

        if !status.success() {
            return Err(CodexError::NonZeroExit {
                status,
                stderr: stderr_string,
            });
        }

        let evaluation: ExecPolicyEvaluation =
            serde_json::from_str(&stdout_string).map_err(|source| CodexError::ExecPolicyParse {
                stdout: stdout_string.clone(),
                source,
            })?;

        Ok(ExecPolicyCheckResult {
            status,
            stdout: stdout_string,
            stderr: stderr_string,
            evaluation,
        })
    }

    /// Probes the configured binary for version/build metadata and supported feature flags.
    ///
    /// Results are cached per canonical binary path and invalidated when file metadata changes.
    /// Caller-supplied overrides (see [`CodexClientBuilder::capability_overrides`]) can
    /// short-circuit probes or layer hints; snapshots are still cached against the current
    /// binary fingerprint so changes on disk trigger revalidation. Missing fingerprints skip
    /// cache reuse to force a re-probe. Cache interaction follows the policy configured on
    /// the builder (see [`CodexClientBuilder::capability_cache_policy`]).
    /// Failures are logged and return conservative defaults so callers can gate optional flags.
    pub async fn probe_capabilities(&self) -> CodexCapabilities {
        self.probe_capabilities_with_policy(self.capability_cache_policy)
            .await
    }

    /// Probes capabilities with an explicit cache policy.
    pub async fn probe_capabilities_with_policy(
        &self,
        cache_policy: CapabilityCachePolicy,
    ) -> CodexCapabilities {
        let cache_key = capability_cache_key(self.command_env.binary_path());
        let fingerprint = current_fingerprint(&cache_key);
        let overrides = &self.capability_overrides;

        let cache_reads_enabled = matches!(cache_policy, CapabilityCachePolicy::PreferCache)
            && has_fingerprint_metadata(&fingerprint);
        let cache_writes_enabled = !matches!(cache_policy, CapabilityCachePolicy::Bypass)
            && has_fingerprint_metadata(&fingerprint);

        if let Some(snapshot) = overrides.snapshot.clone() {
            let capabilities = finalize_capabilities_with_overrides(
                snapshot,
                overrides,
                cache_key.clone(),
                fingerprint.clone(),
                true,
            );
            if cache_writes_enabled {
                update_capability_cache(capabilities.clone());
            }
            return capabilities;
        }

        if cache_reads_enabled {
            if let Some(cached) = cached_capabilities(&cache_key, &fingerprint) {
                if overrides.is_empty() {
                    return cached;
                }
                let merged = finalize_capabilities_with_overrides(
                    cached,
                    overrides,
                    cache_key.clone(),
                    fingerprint.clone(),
                    false,
                );
                if cache_writes_enabled {
                    update_capability_cache(merged.clone());
                }
                return merged;
            }
        }

        let probed = self
            .probe_capabilities_uncached(&cache_key, fingerprint.clone())
            .await;

        let capabilities =
            finalize_capabilities_with_overrides(probed, overrides, cache_key, fingerprint, false);

        if cache_writes_enabled {
            update_capability_cache(capabilities.clone());
        }

        capabilities
    }

    async fn probe_capabilities_uncached(
        &self,
        cache_key: &CapabilityCacheKey,
        fingerprint: Option<BinaryFingerprint>,
    ) -> CodexCapabilities {
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

        CodexCapabilities {
            cache_key: cache_key.clone(),
            fingerprint,
            version,
            features,
            probe_plan: plan,
            collected_at: SystemTime::now(),
        }
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

    async fn invoke_codex_exec(&self, request: ExecRequest) -> Result<String, CodexError> {
        let ExecRequest { prompt, overrides } = request;
        let dir_ctx = self.directory_context()?;
        let needs_capabilities = self.output_schema || !self.add_dirs.is_empty();
        let capabilities = if needs_capabilities {
            Some(self.probe_capabilities().await)
        } else {
            None
        };

        let resolved_overrides =
            resolve_cli_overrides(&self.cli_overrides, &overrides, self.model.as_deref());
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

        apply_cli_overrides(&mut command, &resolved_overrides, true);

        let send_prompt_via_stdin = self.json_output;
        if !send_prompt_via_stdin {
            command.arg(&prompt);
        }
        let stdin_mode = if send_prompt_via_stdin {
            std::process::Stdio::piped()
        } else {
            std::process::Stdio::null()
        };
        command.stdin(stdin_mode);

        if let Some(model) = &self.model {
            command.arg("--model").arg(model);
        }

        if let Some(capabilities) = &capabilities {
            if self.output_schema {
                let guard = capabilities.guard_output_schema();
                if guard_is_supported(&guard) {
                    command.arg("--output-schema");
                } else {
                    log_guard_skip(&guard);
                }
            }

            if !self.add_dirs.is_empty() {
                let guard = capabilities.guard_add_dir();
                if guard_is_supported(&guard) {
                    for dir in &self.add_dirs {
                        command.arg("--add-dir").arg(dir);
                    }
                } else {
                    log_guard_skip(&guard);
                }
            }
        }

        for image in &self.images {
            command.arg("--image").arg(image);
        }

        if self.json_output {
            command.arg("--json");
        }

        self.command_env.apply(&mut command)?;

        let mut child = spawn_with_retry(&mut command, self.command_env.binary_path())?;

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

    fn sandbox_working_dir(&self, request_dir: Option<PathBuf>) -> Result<PathBuf, CodexError> {
        if let Some(dir) = request_dir {
            return Ok(dir);
        }

        if let Some(dir) = &self.working_dir {
            return Ok(dir.clone());
        }

        env::current_dir().map_err(|source| CodexError::WorkingDirectory { source })
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

        let mut child = spawn_with_retry(&mut command, self.command_env.binary_path())?;

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
///
/// Workstream I1 (CLI parity) documents planned builder/request extensions for missing
/// CLI flags and config overrides in `_archived/workstreams/I_cli_parity/design_notes/I1-design-parity-apis.md`.
#[derive(Clone, Debug)]
pub struct CodexClientBuilder {
    binary: PathBuf,
    codex_home: Option<PathBuf>,
    create_home_dirs: bool,
    model: Option<String>,
    timeout: Duration,
    color_mode: ColorMode,
    working_dir: Option<PathBuf>,
    add_dirs: Vec<PathBuf>,
    images: Vec<PathBuf>,
    json_output: bool,
    output_schema: bool,
    quiet: bool,
    mirror_stdout: bool,
    json_event_log: Option<PathBuf>,
    cli_overrides: CliOverrides,
    capability_overrides: CapabilityOverrides,
    capability_cache_policy: CapabilityCachePolicy,
}

impl CodexClientBuilder {
    /// Starts a new builder with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the path to the Codex binary.
    ///
    /// Defaults to `CODEX_BINARY` when present or `codex` on `PATH`. Use this to pin a packaged
    /// binary, e.g. the path returned from [`resolve_bundled_binary`] when your app ships Codex
    /// inside an isolated bundle.
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

    /// Requests that `codex exec` include one or more `--add-dir` flags when the
    /// probed binary supports them. Unsupported or unknown capability results
    /// skip the flag to avoid CLI errors.
    pub fn add_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.add_dirs.push(path.into());
        self
    }

    /// Replaces the current add-dir list with the provided collection.
    pub fn add_dirs<I, P>(mut self, dirs: I) -> Self
    where
        I: IntoIterator<Item = P>,
        P: Into<PathBuf>,
    {
        self.add_dirs = dirs.into_iter().map(Into::into).collect();
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
    ///
    /// Prompts are piped via stdin when enabled. Events include `thread.started`
    /// (or `thread.resumed` when continuing), `turn.started`/`turn.completed`/`turn.failed`,
    /// and `item.created`/`item.updated` with `item.type` such as `agent_message` or `reasoning`.
    /// Pair with `.mirror_stdout(false)` if you plan to parse the stream instead of just mirroring it.
    pub fn json(mut self, enable: bool) -> Self {
        self.json_output = enable;
        self
    }

    /// Requests the `--output-schema` flag when the probed binary reports
    /// support. When capability detection is inconclusive, the flag is skipped
    /// to maintain compatibility with older releases.
    pub fn output_schema(mut self, enable: bool) -> Self {
        self.output_schema = enable;
        self
    }

    /// Suppresses mirroring Codex stderr to the console.
    pub fn quiet(mut self, enable: bool) -> Self {
        self.quiet = enable;
        self
    }

    /// Controls whether Codex stdout should be mirrored to the console while
    /// also being captured. Disable this when you plan to parse JSONL output or
    /// tee the stream to a log file (see `crates/codex/examples/stream_with_log.rs`).
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

    /// Adds a `--config key=value` override that will be applied to every Codex invocation.
    pub fn config_override(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.cli_overrides
            .config_overrides
            .push(ConfigOverride::new(key, value));
        self
    }

    /// Adds a preformatted `--config key=value` override without parsing the input.
    pub fn config_override_raw(mut self, raw: impl Into<String>) -> Self {
        self.cli_overrides
            .config_overrides
            .push(ConfigOverride::from_raw(raw));
        self
    }

    /// Replaces the config overrides with the provided collection.
    pub fn config_overrides<I, K, V>(mut self, overrides: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        self.cli_overrides.config_overrides = overrides
            .into_iter()
            .map(|(key, value)| ConfigOverride::new(key, value))
            .collect();
        self
    }

    /// Selects a Codex config profile (`--profile`).
    pub fn profile(mut self, profile: impl Into<String>) -> Self {
        let profile = profile.into();
        self.cli_overrides.profile = (!profile.trim().is_empty()).then_some(profile);
        self
    }

    /// Sets `model_reasoning_effort` via `--config`.
    pub fn reasoning_effort(mut self, effort: ReasoningEffort) -> Self {
        self.cli_overrides.reasoning.effort = Some(effort);
        self
    }

    /// Sets `model_reasoning_summary` via `--config`.
    pub fn reasoning_summary(mut self, summary: ReasoningSummary) -> Self {
        self.cli_overrides.reasoning.summary = Some(summary);
        self
    }

    /// Sets `model_verbosity` via `--config`.
    pub fn reasoning_verbosity(mut self, verbosity: ModelVerbosity) -> Self {
        self.cli_overrides.reasoning.verbosity = Some(verbosity);
        self
    }

    /// Sets `model_reasoning_summary_format` via `--config`.
    pub fn reasoning_summary_format(mut self, format: ReasoningSummaryFormat) -> Self {
        self.cli_overrides.reasoning.summary_format = Some(format);
        self
    }

    /// Sets `model_supports_reasoning_summaries` via `--config`.
    pub fn supports_reasoning_summaries(mut self, enable: bool) -> Self {
        self.cli_overrides.reasoning.supports_summaries = Some(enable);
        self
    }

    /// Controls whether GPT-5* reasoning defaults should be injected automatically.
    pub fn auto_reasoning_defaults(mut self, enable: bool) -> Self {
        self.cli_overrides.auto_reasoning_defaults = enable;
        self
    }

    /// Sets the approval policy for Codex subprocesses.
    pub fn approval_policy(mut self, policy: ApprovalPolicy) -> Self {
        self.cli_overrides.approval_policy = Some(policy);
        self
    }

    /// Sets the sandbox mode for Codex subprocesses.
    pub fn sandbox_mode(mut self, mode: SandboxMode) -> Self {
        self.cli_overrides.sandbox_mode = Some(mode);
        self
    }

    /// Applies the `--full-auto` safety override unless explicit sandbox/approval options are set.
    pub fn full_auto(mut self, enable: bool) -> Self {
        self.cli_overrides.safety_override = if enable {
            SafetyOverride::FullAuto
        } else {
            SafetyOverride::Inherit
        };
        self
    }

    /// Applies the `--dangerously-bypass-approvals-and-sandbox` override.
    pub fn dangerously_bypass_approvals_and_sandbox(mut self, enable: bool) -> Self {
        self.cli_overrides.safety_override = if enable {
            SafetyOverride::DangerouslyBypass
        } else {
            SafetyOverride::Inherit
        };
        self
    }

    /// Applies `--cd <dir>` to Codex invocations while keeping the process cwd set to `working_dir`.
    pub fn cd(mut self, dir: impl Into<PathBuf>) -> Self {
        self.cli_overrides.cd = Some(dir.into());
        self
    }

    /// Selects a local provider backend (`--local-provider`).
    pub fn local_provider(mut self, provider: LocalProvider) -> Self {
        self.cli_overrides.local_provider = Some(provider);
        self
    }

    /// Requests the CLI `--oss` flag to favor OSS/local backends when available.
    pub fn oss(mut self, enable: bool) -> Self {
        self.cli_overrides.oss = if enable {
            FlagState::Enable
        } else {
            FlagState::Disable
        };
        self
    }

    /// Adds a `--enable <feature>` toggle to Codex invocations.
    pub fn enable_feature(mut self, name: impl Into<String>) -> Self {
        self.cli_overrides.feature_toggles.enable.push(name.into());
        self
    }

    /// Adds a `--disable <feature>` toggle to Codex invocations.
    pub fn disable_feature(mut self, name: impl Into<String>) -> Self {
        self.cli_overrides.feature_toggles.disable.push(name.into());
        self
    }

    /// Controls whether `--search` is passed through to Codex.
    pub fn search(mut self, enable: bool) -> Self {
        self.cli_overrides.search = if enable {
            FlagState::Enable
        } else {
            FlagState::Disable
        };
        self
    }

    /// Supplies manual capability data to skip probes or adjust feature flags.
    pub fn capability_overrides(mut self, overrides: CapabilityOverrides) -> Self {
        self.capability_overrides = overrides;
        self
    }

    /// Convenience to apply feature overrides or vendor hints without touching versions.
    pub fn capability_feature_overrides(mut self, overrides: CapabilityFeatureOverrides) -> Self {
        self.capability_overrides.features = overrides;
        self
    }

    /// Convenience to opt into specific feature flags while leaving other probes intact.
    pub fn capability_feature_hints(mut self, features: CodexFeatureFlags) -> Self {
        self.capability_overrides.features = CapabilityFeatureOverrides::enabling(features);
        self
    }

    /// Supplies a precomputed capability snapshot for pinned or bundled Codex builds.
    /// Combine with `write_capabilities_snapshot` / `read_capabilities_snapshot`
    /// to persist probe results between processes.
    pub fn capability_snapshot(mut self, snapshot: CodexCapabilities) -> Self {
        self.capability_overrides.snapshot = Some(snapshot);
        self
    }

    /// Overrides the probed version data with caller-provided metadata.
    pub fn capability_version_override(mut self, version: CodexVersionInfo) -> Self {
        self.capability_overrides.version = Some(version);
        self
    }

    /// Controls how capability probes interact with the in-process cache.
    /// Use [`CapabilityCachePolicy::Refresh`] to enforce a TTL/backoff when
    /// binaries are hot-swapped without changing fingerprints.
    pub fn capability_cache_policy(mut self, policy: CapabilityCachePolicy) -> Self {
        self.capability_cache_policy = policy;
        self
    }

    /// Convenience to bypass the capability cache when a fresh snapshot is required.
    /// Bypass skips cache reads and writes for the probe.
    pub fn bypass_capability_cache(mut self, bypass: bool) -> Self {
        self.capability_cache_policy = if bypass {
            CapabilityCachePolicy::Bypass
        } else {
            CapabilityCachePolicy::PreferCache
        };
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
            add_dirs: self.add_dirs,
            images: self.images,
            json_output: self.json_output,
            output_schema: self.output_schema,
            quiet: self.quiet,
            mirror_stdout: self.mirror_stdout,
            json_event_log: self.json_event_log,
            cli_overrides: self.cli_overrides,
            capability_overrides: self.capability_overrides,
            capability_cache_policy: self.capability_cache_policy,
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
            add_dirs: Vec::new(),
            images: Vec::new(),
            json_output: false,
            output_schema: false,
            quiet: false,
            mirror_stdout: true,
            json_event_log: None,
            cli_overrides: CliOverrides::default(),
            capability_overrides: CapabilityOverrides::default(),
            capability_cache_policy: CapabilityCachePolicy::default(),
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

/// Approval policy used by `--ask-for-approval`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApprovalPolicy {
    Untrusted,
    OnFailure,
    OnRequest,
    Never,
}

impl ApprovalPolicy {
    const fn as_str(self) -> &'static str {
        match self {
            ApprovalPolicy::Untrusted => "untrusted",
            ApprovalPolicy::OnFailure => "on-failure",
            ApprovalPolicy::OnRequest => "on-request",
            ApprovalPolicy::Never => "never",
        }
    }
}

/// Sandbox isolation level.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SandboxMode {
    ReadOnly,
    WorkspaceWrite,
    DangerFullAccess,
}

impl SandboxMode {
    const fn as_str(self) -> &'static str {
        match self {
            SandboxMode::ReadOnly => "read-only",
            SandboxMode::WorkspaceWrite => "workspace-write",
            SandboxMode::DangerFullAccess => "danger-full-access",
        }
    }
}

/// Safety overrides that collapse approval/sandbox behavior.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SafetyOverride {
    #[default]
    Inherit,
    FullAuto,
    DangerouslyBypass,
}

/// Local provider selection for OSS backends.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalProvider {
    LmStudio,
    Ollama,
    Custom,
}

impl LocalProvider {
    const fn as_str(self) -> &'static str {
        match self {
            LocalProvider::LmStudio => "lmstudio",
            LocalProvider::Ollama => "ollama",
            LocalProvider::Custom => "custom",
        }
    }
}

/// Three-state flag used when requests can override builder defaults.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum FlagState {
    #[default]
    Inherit,
    Enable,
    Disable,
}

/// Feature toggles forwarded to `--enable/--disable`.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FeatureToggles {
    pub enable: Vec<String>,
    pub disable: Vec<String>,
}

/// Config values for `model_reasoning_effort`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReasoningEffort {
    Minimal,
    Low,
    Medium,
    High,
}

impl ReasoningEffort {
    const fn as_str(self) -> &'static str {
        match self {
            ReasoningEffort::Minimal => "minimal",
            ReasoningEffort::Low => "low",
            ReasoningEffort::Medium => "medium",
            ReasoningEffort::High => "high",
        }
    }
}

/// Config values for `model_reasoning_summary`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReasoningSummary {
    Auto,
    Concise,
    Detailed,
    None,
}

impl ReasoningSummary {
    const fn as_str(self) -> &'static str {
        match self {
            ReasoningSummary::Auto => "auto",
            ReasoningSummary::Concise => "concise",
            ReasoningSummary::Detailed => "detailed",
            ReasoningSummary::None => "none",
        }
    }
}

/// Config values for `model_verbosity`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelVerbosity {
    Low,
    Medium,
    High,
}

impl ModelVerbosity {
    const fn as_str(self) -> &'static str {
        match self {
            ModelVerbosity::Low => "low",
            ModelVerbosity::Medium => "medium",
            ModelVerbosity::High => "high",
        }
    }
}

/// Config values for `model_reasoning_summary_format`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReasoningSummaryFormat {
    None,
    Experimental,
}

impl ReasoningSummaryFormat {
    const fn as_str(self) -> &'static str {
        match self {
            ReasoningSummaryFormat::None => "none",
            ReasoningSummaryFormat::Experimental => "experimental",
        }
    }
}

/// Represents a single `--config key=value` override.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfigOverride {
    pub key: String,
    pub value: String,
}

impl ConfigOverride {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
        }
    }

    pub fn from_raw(raw: impl Into<String>) -> Self {
        let raw = raw.into();
        let (key, value) = raw
            .split_once('=')
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .unwrap_or_else(|| (raw.clone(), String::new()));
        ConfigOverride { key, value }
    }

    fn is_reasoning_key(&self) -> bool {
        REASONING_CONFIG_KEYS.contains(&self.key.as_str())
    }
}

/// Structured reasoning overrides converted into config entries.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ReasoningOverrides {
    pub effort: Option<ReasoningEffort>,
    pub summary: Option<ReasoningSummary>,
    pub verbosity: Option<ModelVerbosity>,
    pub summary_format: Option<ReasoningSummaryFormat>,
    pub supports_summaries: Option<bool>,
}

impl ReasoningOverrides {
    fn has_overrides(&self) -> bool {
        self.effort.is_some()
            || self.summary.is_some()
            || self.verbosity.is_some()
            || self.summary_format.is_some()
            || self.supports_summaries.is_some()
    }

    fn append_overrides(&self, configs: &mut Vec<ConfigOverride>) {
        if let Some(value) = self.effort {
            configs.push(ConfigOverride::new(
                "model_reasoning_effort",
                value.as_str(),
            ));
        }
        if let Some(value) = self.summary {
            configs.push(ConfigOverride::new(
                "model_reasoning_summary",
                value.as_str(),
            ));
        }
        if let Some(value) = self.verbosity {
            configs.push(ConfigOverride::new("model_verbosity", value.as_str()));
        }
        if let Some(value) = self.summary_format {
            configs.push(ConfigOverride::new(
                "model_reasoning_summary_format",
                value.as_str(),
            ));
        }
        if let Some(value) = self.supports_summaries {
            configs.push(ConfigOverride::new(
                "model_supports_reasoning_summaries",
                value.to_string(),
            ));
        }
    }
}

/// Builder-scoped CLI overrides.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CliOverrides {
    pub config_overrides: Vec<ConfigOverride>,
    pub feature_toggles: FeatureToggles,
    pub reasoning: ReasoningOverrides,
    pub approval_policy: Option<ApprovalPolicy>,
    pub sandbox_mode: Option<SandboxMode>,
    pub safety_override: SafetyOverride,
    pub profile: Option<String>,
    pub cd: Option<PathBuf>,
    pub local_provider: Option<LocalProvider>,
    pub oss: FlagState,
    pub search: FlagState,
    pub auto_reasoning_defaults: bool,
}

impl Default for CliOverrides {
    fn default() -> Self {
        Self {
            config_overrides: Vec::new(),
            feature_toggles: FeatureToggles::default(),
            reasoning: ReasoningOverrides::default(),
            approval_policy: None,
            sandbox_mode: None,
            safety_override: SafetyOverride::Inherit,
            profile: None,
            cd: None,
            local_provider: None,
            oss: FlagState::Inherit,
            search: FlagState::Inherit,
            auto_reasoning_defaults: true,
        }
    }
}

/// Request-level overlay of builder overrides.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CliOverridesPatch {
    pub config_overrides: Vec<ConfigOverride>,
    pub feature_toggles: FeatureToggles,
    pub reasoning: ReasoningOverrides,
    pub approval_policy: Option<ApprovalPolicy>,
    pub sandbox_mode: Option<SandboxMode>,
    pub safety_override: Option<SafetyOverride>,
    pub profile: Option<String>,
    pub cd: Option<PathBuf>,
    pub local_provider: Option<LocalProvider>,
    pub oss: FlagState,
    pub search: FlagState,
    pub auto_reasoning_defaults: Option<bool>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ResolvedCliOverrides {
    config_overrides: Vec<ConfigOverride>,
    feature_toggles: FeatureToggles,
    approval_policy: Option<ApprovalPolicy>,
    sandbox_mode: Option<SandboxMode>,
    safety_override: SafetyOverride,
    profile: Option<String>,
    cd: Option<PathBuf>,
    local_provider: Option<LocalProvider>,
    oss: bool,
    search: FlagState,
}

impl ResolvedCliOverrides {
    fn search_enabled(&self) -> bool {
        matches!(self.search, FlagState::Enable)
    }
}

const REASONING_CONFIG_KEYS: &[&str] = &[
    "model_reasoning_effort",
    "model_reasoning_summary",
    "model_verbosity",
    "model_reasoning_summary_format",
    "model_supports_reasoning_summaries",
];

fn reasoning_config_for(model: Option<&str>) -> Option<&'static [(&'static str, &'static str)]> {
    let name = model.map(|value| value.to_ascii_lowercase())?;
    match name.as_str() {
        name if name.starts_with("gpt-5.1-codex") => Some(DEFAULT_REASONING_CONFIG_GPT5_1),
        name if name.starts_with("gpt-5.1") => Some(DEFAULT_REASONING_CONFIG_GPT5_1),
        "gpt-5-codex" => Some(DEFAULT_REASONING_CONFIG_GPT5_CODEX),
        name if name.starts_with("gpt-5") => Some(DEFAULT_REASONING_CONFIG_GPT5),
        _ => None,
    }
}

fn has_reasoning_config_override(overrides: &[ConfigOverride]) -> bool {
    overrides.iter().any(ConfigOverride::is_reasoning_key)
}

fn resolve_cli_overrides(
    builder: &CliOverrides,
    patch: &CliOverridesPatch,
    model: Option<&str>,
) -> ResolvedCliOverrides {
    let auto_reasoning_defaults = patch
        .auto_reasoning_defaults
        .unwrap_or(builder.auto_reasoning_defaults);

    let has_reasoning_overrides = builder.reasoning.has_overrides()
        || patch.reasoning.has_overrides()
        || has_reasoning_config_override(&builder.config_overrides)
        || has_reasoning_config_override(&patch.config_overrides);

    let mut config_overrides = Vec::new();
    if auto_reasoning_defaults && !has_reasoning_overrides {
        if let Some(defaults) = reasoning_config_for(model) {
            for (key, value) in defaults {
                config_overrides.push(ConfigOverride::new(*key, *value));
            }
        }
    }

    config_overrides.extend(builder.config_overrides.clone());
    builder.reasoning.append_overrides(&mut config_overrides);
    config_overrides.extend(patch.config_overrides.clone());
    patch.reasoning.append_overrides(&mut config_overrides);

    let approval_policy = patch.approval_policy.or(builder.approval_policy);
    let sandbox_mode = patch.sandbox_mode.or(builder.sandbox_mode);
    let safety_override = patch.safety_override.unwrap_or(builder.safety_override);
    let profile = patch.profile.clone().or_else(|| builder.profile.clone());
    let cd = patch.cd.clone().or_else(|| builder.cd.clone());
    let local_provider = patch.local_provider.or(builder.local_provider);
    let search = match patch.search {
        FlagState::Inherit => builder.search,
        other => other,
    };
    let oss = match patch.oss {
        FlagState::Inherit => builder.oss,
        other => other,
    };
    let mut feature_toggles = builder.feature_toggles.clone();
    feature_toggles
        .enable
        .extend(patch.feature_toggles.enable.iter().cloned());
    feature_toggles
        .disable
        .extend(patch.feature_toggles.disable.iter().cloned());

    ResolvedCliOverrides {
        config_overrides,
        feature_toggles,
        approval_policy,
        sandbox_mode,
        safety_override,
        profile,
        cd,
        local_provider,
        oss: matches!(oss, FlagState::Enable),
        search,
    }
}

fn cli_override_args(resolved: &ResolvedCliOverrides, include_search: bool) -> Vec<OsString> {
    let mut args = Vec::new();
    for config in &resolved.config_overrides {
        args.push(OsString::from("--config"));
        args.push(OsString::from(format!("{}={}", config.key, config.value)));
    }

    for feature in &resolved.feature_toggles.enable {
        args.push(OsString::from("--enable"));
        args.push(OsString::from(feature));
    }

    for feature in &resolved.feature_toggles.disable {
        args.push(OsString::from("--disable"));
        args.push(OsString::from(feature));
    }

    if let Some(profile) = &resolved.profile {
        args.push(OsString::from("--profile"));
        args.push(OsString::from(profile));
    }

    match resolved.safety_override {
        SafetyOverride::DangerouslyBypass => {
            args.push(OsString::from("--dangerously-bypass-approvals-and-sandbox"));
        }
        other => {
            if let Some(policy) = resolved.approval_policy {
                args.push(OsString::from("--ask-for-approval"));
                args.push(OsString::from(policy.as_str()));
            }

            if let Some(mode) = resolved.sandbox_mode {
                args.push(OsString::from("--sandbox"));
                args.push(OsString::from(mode.as_str()));
            } else if resolved.approval_policy.is_none()
                && matches!(other, SafetyOverride::FullAuto)
            {
                args.push(OsString::from("--full-auto"));
            }
        }
    }

    if let Some(cd) = &resolved.cd {
        args.push(OsString::from("--cd"));
        args.push(cd.as_os_str().to_os_string());
    }

    if let Some(provider) = resolved.local_provider {
        args.push(OsString::from("--local-provider"));
        args.push(OsString::from(provider.as_str()));
    }

    if resolved.oss {
        args.push(OsString::from("--oss"));
    }

    if include_search && resolved.search_enabled() {
        args.push(OsString::from("--search"));
    }

    args
}

fn apply_cli_overrides(
    command: &mut Command,
    resolved: &ResolvedCliOverrides,
    include_search: bool,
) {
    for arg in cli_override_args(resolved, include_search) {
        command.arg(arg);
    }
}

fn spawn_with_retry(
    command: &mut Command,
    binary: &Path,
) -> Result<tokio::process::Child, CodexError> {
    let mut backoff = Duration::from_millis(2);
    for attempt in 0..5 {
        match command.spawn() {
            Ok(child) => return Ok(child),
            Err(source) => {
                let is_busy = matches!(source.kind(), std::io::ErrorKind::ExecutableFileBusy)
                    || source.raw_os_error() == Some(26);
                if is_busy && attempt < 4 {
                    std::thread::sleep(backoff);
                    backoff = std::cmp::min(backoff * 2, Duration::from_millis(50));
                    continue;
                }
                return Err(CodexError::Spawn {
                    binary: binary.to_path_buf(),
                    source,
                });
            }
        }
    }

    unreachable!("spawn_with_retry should return before exhausting retries")
}

#[derive(Clone, Debug)]
struct CommandEnvironment {
    binary: PathBuf,
    codex_home: Option<CodexHomeLayout>,
    create_home_dirs: bool,
}

impl CommandEnvironment {
    fn new(binary: PathBuf, codex_home: Option<PathBuf>, create_home_dirs: bool) -> Self {
        Self {
            binary,
            codex_home: codex_home.map(CodexHomeLayout::new),
            create_home_dirs,
        }
    }

    fn binary_path(&self) -> &Path {
        &self.binary
    }

    fn codex_home_layout(&self) -> Option<CodexHomeLayout> {
        self.codex_home.clone()
    }

    fn environment_overrides(&self) -> Result<Vec<(OsString, OsString)>, CodexError> {
        if let Some(home) = &self.codex_home {
            home.materialize(self.create_home_dirs)?;
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

        if let Some(value) = default_rust_log_value() {
            envs.push((OsString::from(RUST_LOG_ENV), OsString::from(value)));
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

/// Describes the on-disk layout used by the Codex CLI when `CODEX_HOME` is set.
///
/// Files are rooted next to `config.toml`, `auth.json`, `.credentials.json`, and
/// `history.jsonl`; `conversations/` holds transcript JSONL files and `logs/`
/// holds `codex-*.log` outputs. Call [`Self::materialize`] to create the
/// directories when standing up an app-scoped home.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodexHomeLayout {
    root: PathBuf,
}

impl CodexHomeLayout {
    /// Creates a new layout description rooted at `root`.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Returns the `CODEX_HOME` root.
    pub fn root(&self) -> &Path {
        self.root.as_path()
    }

    /// Path to `config.toml` under `CODEX_HOME`.
    pub fn config_path(&self) -> PathBuf {
        self.root.join("config.toml")
    }

    /// Path to `auth.json` under `CODEX_HOME`.
    pub fn auth_path(&self) -> PathBuf {
        self.root.join("auth.json")
    }

    /// Path to `.credentials.json` under `CODEX_HOME`.
    pub fn credentials_path(&self) -> PathBuf {
        self.root.join(".credentials.json")
    }

    /// Path to `history.jsonl` under `CODEX_HOME`.
    pub fn history_path(&self) -> PathBuf {
        self.root.join("history.jsonl")
    }

    /// Directory containing conversation transcripts.
    pub fn conversations_dir(&self) -> PathBuf {
        self.root.join("conversations")
    }

    /// Directory containing Codex log files.
    pub fn logs_dir(&self) -> PathBuf {
        self.root.join("logs")
    }

    /// Creates the `CODEX_HOME` root and its known subdirectories when
    /// `create_home_dirs` is `true`. No-op when disabled.
    pub fn materialize(&self, create_home_dirs: bool) -> Result<(), CodexError> {
        if !create_home_dirs {
            return Ok(());
        }

        let conversations = self.conversations_dir();
        let logs = self.logs_dir();
        for path in [self.root(), conversations.as_path(), logs.as_path()] {
            std_fs::create_dir_all(path).map_err(|source| CodexError::PrepareCodexHome {
                path: path.to_path_buf(),
                source,
            })?;
        }
        Ok(())
    }

    /// Copies login artifacts (`auth.json` and `.credentials.json`) from a trusted seed home into
    /// this layout. History and logs are intentionally excluded.
    ///
    /// This is opt-in and leaves defaults untouched. Missing files raise errors only when marked
    /// as required in `options`; otherwise they are skipped. Target directories are created when
    /// `create_target_dirs` is `true`.
    pub fn seed_auth_from(
        &self,
        seed_home: impl AsRef<Path>,
        options: AuthSeedOptions,
    ) -> Result<AuthSeedOutcome, AuthSeedError> {
        let seed_home = seed_home.as_ref();
        let seed_meta =
            std_fs::metadata(seed_home).map_err(|source| AuthSeedError::SeedHomeUnreadable {
                seed_home: seed_home.to_path_buf(),
                source,
            })?;
        if !seed_meta.is_dir() {
            return Err(AuthSeedError::SeedHomeNotDirectory {
                seed_home: seed_home.to_path_buf(),
            });
        }

        let mut outcome = AuthSeedOutcome::default();
        let targets = [
            (
                "auth.json",
                options.require_auth,
                &mut outcome.copied_auth,
                self.auth_path(),
            ),
            (
                ".credentials.json",
                options.require_credentials,
                &mut outcome.copied_credentials,
                self.credentials_path(),
            ),
        ];

        for (name, required, copied, destination) in targets {
            let source = seed_home.join(name);
            match std_fs::metadata(&source) {
                Ok(metadata) => {
                    if !metadata.is_file() {
                        return Err(AuthSeedError::SeedFileNotFile { path: source });
                    }

                    if options.create_target_dirs {
                        if let Some(parent) = destination.parent() {
                            std_fs::create_dir_all(parent).map_err(|source_err| {
                                AuthSeedError::CreateTargetDir {
                                    path: parent.to_path_buf(),
                                    source: source_err,
                                }
                            })?;
                        }
                    }

                    std_fs::copy(&source, &destination).map_err(|error| AuthSeedError::Copy {
                        source: source.clone(),
                        destination: destination.to_path_buf(),
                        error,
                    })?;
                    *copied = true;
                }
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                    if required {
                        return Err(AuthSeedError::SeedFileMissing { path: source });
                    }
                }
                Err(err) => {
                    return Err(AuthSeedError::SeedFileUnreadable {
                        path: source,
                        source: err,
                    })
                }
            }
        }

        Ok(outcome)
    }
}

/// Options controlling how auth files are seeded from a trusted home.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthSeedOptions {
    /// Whether missing `auth.json` is an error (default: false, skip when missing).
    pub require_auth: bool,
    /// Whether missing `.credentials.json` is an error (default: false, skip when missing).
    pub require_credentials: bool,
    /// Create destination directories when needed (default: true).
    pub create_target_dirs: bool,
}

impl Default for AuthSeedOptions {
    fn default() -> Self {
        Self {
            require_auth: false,
            require_credentials: false,
            create_target_dirs: true,
        }
    }
}

/// Result of seeding Codex auth files into a target home.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AuthSeedOutcome {
    /// `true` when `auth.json` was copied.
    pub copied_auth: bool,
    /// `true` when `.credentials.json` was copied.
    pub copied_credentials: bool,
}

/// Errors that may occur while seeding Codex auth files into a target home.
#[derive(Debug, Error)]
pub enum AuthSeedError {
    #[error("seed CODEX_HOME `{seed_home}` does not exist or is unreadable")]
    SeedHomeUnreadable {
        seed_home: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("seed CODEX_HOME `{seed_home}` is not a directory")]
    SeedHomeNotDirectory { seed_home: PathBuf },
    #[error("seed file `{path}` is missing")]
    SeedFileMissing { path: PathBuf },
    #[error("seed file `{path}` is not a file")]
    SeedFileNotFile { path: PathBuf },
    #[error("seed file `{path}` is unreadable")]
    SeedFileUnreadable {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to create target directory `{path}`")]
    CreateTargetDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to copy `{source}` to `{destination}`")]
    Copy {
        source: PathBuf,
        destination: PathBuf,
        #[source]
        error: std::io::Error,
    },
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
    #[error("failed to parse execpolicy JSON output: {source}")]
    ExecPolicyParse {
        stdout: String,
        #[source]
        source: serde_json::Error,
    },
    #[error("failed to parse features list output: {reason}")]
    FeatureListParse { reason: String, stdout: String },
    #[error("failed to read responses-api-proxy server info from `{path}`: {source}")]
    ResponsesApiProxyInfoRead {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse responses-api-proxy server info from `{path}`: {source}")]
    ResponsesApiProxyInfoParse {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("prompt must not be empty")]
    EmptyPrompt,
    #[error("sandbox command must not be empty")]
    EmptySandboxCommand,
    #[error("execpolicy command must not be empty")]
    EmptyExecPolicyCommand,
    #[error("API key must not be empty")]
    EmptyApiKey,
    #[error("socket path must not be empty")]
    EmptySocketPath,
    #[error("failed to create temporary working directory: {0}")]
    TempDir(#[source] std::io::Error),
    #[error("failed to resolve working directory: {source}")]
    WorkingDirectory {
        #[source]
        source: std::io::Error,
    },
    #[error("failed to prepare app-server output directory `{path}`: {source}")]
    PrepareOutputDirectory {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
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
    #[serde(rename = "thread.started", alias = "thread.resumed")]
    ThreadStarted(ThreadStarted),
    #[serde(rename = "turn.started")]
    TurnStarted(TurnStarted),
    #[serde(rename = "turn.completed")]
    TurnCompleted(TurnCompleted),
    #[serde(rename = "turn.failed")]
    TurnFailed(TurnFailed),
    #[serde(rename = "item.started", alias = "item.created")]
    ItemStarted(ItemEnvelope<ItemSnapshot>),
    #[serde(rename = "item.delta", alias = "item.updated")]
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
    #[serde(rename = "item_id", alias = "id")]
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
#[derive(Clone, Debug, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ItemStatus {
    #[default]
    InProgress,
    Completed,
    Failed,
    #[serde(other)]
    Unknown,
}

/// Human-readable content emitted by the agent.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TextContent {
    pub text: String,
    #[serde(flatten, default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, Value>,
}

/// Incremental content fragment for streaming items.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TextDelta {
    #[serde(rename = "text_delta", alias = "text")]
    pub text_delta: String,
    #[serde(flatten, default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, Value>,
}

/// Snapshot of a command execution, including accumulated stdout/stderr.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CommandExecutionState {
    pub command: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    #[serde(
        default,
        skip_serializing_if = "String::is_empty",
        alias = "aggregated_output",
        alias = "output"
    )]
    pub stdout: String,
    #[serde(
        default,
        skip_serializing_if = "String::is_empty",
        alias = "error_output",
        alias = "err"
    )]
    pub stderr: String,
    #[serde(flatten, default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, Value>,
}

/// Streaming delta for command execution.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CommandExecutionDelta {
    #[serde(
        default,
        skip_serializing_if = "String::is_empty",
        alias = "aggregated_output",
        alias = "output"
    )]
    pub stdout: String,
    #[serde(
        default,
        skip_serializing_if = "String::is_empty",
        alias = "error_output",
        alias = "err"
    )]
    pub stderr: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    #[serde(flatten, default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, Value>,
}

/// File change or diff applied by the agent.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FileChangeState {
    #[serde(alias = "file_path")]
    pub path: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub change: Option<FileChangeKind>,
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "patch")]
    pub diff: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    #[serde(
        default,
        skip_serializing_if = "String::is_empty",
        alias = "aggregated_output",
        alias = "output"
    )]
    pub stdout: String,
    #[serde(
        default,
        skip_serializing_if = "String::is_empty",
        alias = "error_output",
        alias = "err"
    )]
    pub stderr: String,
    #[serde(flatten, default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, Value>,
}

/// Streaming delta describing a file change.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FileChangeDelta {
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "patch")]
    pub diff: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "String::is_empty",
        alias = "aggregated_output",
        alias = "output"
    )]
    pub stdout: String,
    #[serde(
        default,
        skip_serializing_if = "String::is_empty",
        alias = "error_output",
        alias = "err"
    )]
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
    #[serde(alias = "server")]
    pub server_name: String,
    #[serde(alias = "tool")]
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
#[derive(Clone, Debug, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ToolCallStatus {
    #[default]
    Pending,
    Running,
    Completed,
    Failed,
    #[serde(other)]
    Unknown,
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
#[derive(Clone, Debug, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum WebSearchStatus {
    #[default]
    Pending,
    Running,
    Completed,
    Failed,
    #[serde(other)]
    Unknown,
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

/// Options configuring a single exec request.
#[derive(Clone, Debug)]
pub struct ExecRequest {
    pub prompt: String,
    pub overrides: CliOverridesPatch,
}

impl ExecRequest {
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            overrides: CliOverridesPatch::default(),
        }
    }

    pub fn with_overrides(mut self, overrides: CliOverridesPatch) -> Self {
        self.overrides = overrides;
        self
    }

    pub fn config_override(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.overrides
            .config_overrides
            .push(ConfigOverride::new(key, value));
        self
    }

    pub fn config_override_raw(mut self, raw: impl Into<String>) -> Self {
        self.overrides
            .config_overrides
            .push(ConfigOverride::from_raw(raw));
        self
    }

    pub fn profile(mut self, profile: impl Into<String>) -> Self {
        let profile = profile.into();
        self.overrides.profile = (!profile.trim().is_empty()).then_some(profile);
        self
    }

    pub fn oss(mut self, enable: bool) -> Self {
        self.overrides.oss = if enable {
            FlagState::Enable
        } else {
            FlagState::Disable
        };
        self
    }

    pub fn enable_feature(mut self, name: impl Into<String>) -> Self {
        self.overrides.feature_toggles.enable.push(name.into());
        self
    }

    pub fn disable_feature(mut self, name: impl Into<String>) -> Self {
        self.overrides.feature_toggles.disable.push(name.into());
        self
    }

    pub fn search(mut self, enable: bool) -> Self {
        self.overrides.search = if enable {
            FlagState::Enable
        } else {
            FlagState::Disable
        };
        self
    }
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

/// Selector for `codex resume` targets.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResumeSelector {
    Id(String),
    Last,
    All,
}

/// Options configuring a streaming resume invocation.
#[derive(Clone, Debug)]
pub struct ResumeRequest {
    pub selector: ResumeSelector,
    pub prompt: Option<String>,
    pub idle_timeout: Option<Duration>,
    pub output_last_message: Option<PathBuf>,
    pub output_schema: Option<PathBuf>,
    pub json_event_log: Option<PathBuf>,
    pub overrides: CliOverridesPatch,
}

impl ResumeRequest {
    pub fn new(selector: ResumeSelector) -> Self {
        Self {
            selector,
            prompt: None,
            idle_timeout: None,
            output_last_message: None,
            output_schema: None,
            json_event_log: None,
            overrides: CliOverridesPatch::default(),
        }
    }

    pub fn with_id(id: impl Into<String>) -> Self {
        Self::new(ResumeSelector::Id(id.into()))
    }

    pub fn last() -> Self {
        Self::new(ResumeSelector::Last)
    }

    pub fn all() -> Self {
        Self::new(ResumeSelector::All)
    }

    pub fn prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = Some(prompt.into());
        self
    }

    pub fn idle_timeout(mut self, idle_timeout: Duration) -> Self {
        self.idle_timeout = Some(idle_timeout);
        self
    }

    pub fn config_override(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.overrides
            .config_overrides
            .push(ConfigOverride::new(key, value));
        self
    }

    pub fn config_override_raw(mut self, raw: impl Into<String>) -> Self {
        self.overrides
            .config_overrides
            .push(ConfigOverride::from_raw(raw));
        self
    }

    pub fn profile(mut self, profile: impl Into<String>) -> Self {
        let profile = profile.into();
        self.overrides.profile = (!profile.trim().is_empty()).then_some(profile);
        self
    }

    pub fn oss(mut self, enable: bool) -> Self {
        self.overrides.oss = if enable {
            FlagState::Enable
        } else {
            FlagState::Disable
        };
        self
    }

    pub fn enable_feature(mut self, name: impl Into<String>) -> Self {
        self.overrides.feature_toggles.enable.push(name.into());
        self
    }

    pub fn disable_feature(mut self, name: impl Into<String>) -> Self {
        self.overrides.feature_toggles.disable.push(name.into());
        self
    }

    pub fn search(mut self, enable: bool) -> Self {
        self.overrides.search = if enable {
            FlagState::Enable
        } else {
            FlagState::Disable
        };
        self
    }
}

/// Sandbox platform variant; maps to platform subcommands of `codex sandbox`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SandboxPlatform {
    Macos,
    Linux,
    Windows,
}

impl SandboxPlatform {
    fn subcommand(self) -> &'static str {
        match self {
            SandboxPlatform::Macos => "macos",
            SandboxPlatform::Linux => "linux",
            SandboxPlatform::Windows => "windows",
        }
    }
}

/// Request to run an arbitrary command inside a Codex-provided sandbox.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SandboxCommandRequest {
    /// Target platform subcommand; maps to `macos` (alias `seatbelt`), `linux` (alias `landlock`), or `windows`.
    pub platform: SandboxPlatform,
    /// Trailing command arguments to execute. Must be non-empty to avoid the upstream CLI panic.
    pub command: Vec<OsString>,
    /// Request the workspace-write sandbox preset (`--full-auto`).
    pub full_auto: bool,
    /// Stream macOS sandbox denials after the child process exits (no-op on other platforms).
    pub log_denials: bool,
    /// Additional `--config key=value` overrides to pass through.
    pub config_overrides: Vec<ConfigOverride>,
    /// Feature toggles forwarded to `--enable`/`--disable`.
    pub feature_toggles: FeatureToggles,
    /// Working directory for the spawned command; falls back to the builder value, then the current process directory.
    pub working_dir: Option<PathBuf>,
}

impl SandboxCommandRequest {
    pub fn new<I, S>(platform: SandboxPlatform, command: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        Self {
            platform,
            command: command.into_iter().map(Into::into).collect(),
            full_auto: false,
            log_denials: false,
            config_overrides: Vec::new(),
            feature_toggles: FeatureToggles::default(),
            working_dir: None,
        }
    }

    pub fn full_auto(mut self, enable: bool) -> Self {
        self.full_auto = enable;
        self
    }

    pub fn log_denials(mut self, enable: bool) -> Self {
        self.log_denials = enable;
        self
    }

    pub fn config_override(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.config_overrides.push(ConfigOverride::new(key, value));
        self
    }

    pub fn config_override_raw(mut self, raw: impl Into<String>) -> Self {
        self.config_overrides.push(ConfigOverride::from_raw(raw));
        self
    }

    pub fn enable_feature(mut self, name: impl Into<String>) -> Self {
        self.feature_toggles.enable.push(name.into());
        self
    }

    pub fn disable_feature(mut self, name: impl Into<String>) -> Self {
        self.feature_toggles.disable.push(name.into());
        self
    }

    pub fn working_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.working_dir = Some(dir.into());
        self
    }
}

/// Captured output from `codex sandbox <platform>`.
#[derive(Clone, Debug)]
pub struct SandboxRun {
    /// Exit status returned by the inner command (mirrors the sandbox helper).
    pub status: ExitStatus,
    /// Captured stdout (mirrored to the console when `mirror_stdout` is true).
    pub stdout: String,
    /// Captured stderr (mirrored unless `quiet` is set).
    pub stderr: String,
}

/// Request for `codex responses-api-proxy`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResponsesApiProxyRequest {
    /// API key to write to stdin on startup.
    pub api_key: String,
    /// Optional port to bind; falls back to an OS-assigned ephemeral port when omitted.
    pub port: Option<u16>,
    /// Optional path passed to `--server-info` for `{port,pid}` JSON output.
    pub server_info_path: Option<PathBuf>,
    /// Enables the HTTP shutdown endpoint (`GET /shutdown`).
    pub http_shutdown: bool,
    /// Optional upstream URL passed to `--upstream-url` (defaults to `https://api.openai.com/v1/responses`).
    pub upstream_url: Option<String>,
}

impl ResponsesApiProxyRequest {
    /// Creates a request with the API key provided via stdin.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            port: None,
            server_info_path: None,
            http_shutdown: false,
            upstream_url: None,
        }
    }

    /// Sets the listening port (`--port`).
    pub fn port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    /// Writes `{port,pid}` JSON to the provided path via `--server-info`.
    pub fn server_info(mut self, path: impl Into<PathBuf>) -> Self {
        self.server_info_path = Some(path.into());
        self
    }

    /// Enables the `--http-shutdown` flag (GET /shutdown).
    pub fn http_shutdown(mut self, enable: bool) -> Self {
        self.http_shutdown = enable;
        self
    }

    /// Overrides the upstream responses endpoint URL.
    pub fn upstream_url(mut self, url: impl Into<String>) -> Self {
        let url = url.into();
        self.upstream_url = (!url.trim().is_empty()).then_some(url);
        self
    }
}

/// Running responses proxy process and metadata.
#[derive(Debug)]
pub struct ResponsesApiProxyHandle {
    /// Spawned `codex responses-api-proxy` child (inherits kill-on-drop).
    pub child: tokio::process::Child,
    /// Optional `--server-info` path that may contain `{port,pid}` JSON.
    pub server_info_path: Option<PathBuf>,
}

impl ResponsesApiProxyHandle {
    /// Reads and parses the `{port,pid}` JSON written by `--server-info`.
    ///
    /// Returns `Ok(None)` when no server info path was configured.
    pub async fn read_server_info(&self) -> Result<Option<ResponsesApiProxyInfo>, CodexError> {
        let Some(path) = &self.server_info_path else {
            return Ok(None);
        };

        let contents = fs::read_to_string(path).await.map_err(|source| {
            CodexError::ResponsesApiProxyInfoRead {
                path: path.clone(),
                source,
            }
        })?;
        let info: ResponsesApiProxyInfo = serde_json::from_str(&contents).map_err(|source| {
            CodexError::ResponsesApiProxyInfoParse {
                path: path.clone(),
                source,
            }
        })?;
        Ok(Some(info))
    }
}

/// Parsed `{port,pid}` emitted by `codex responses-api-proxy --server-info`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ResponsesApiProxyInfo {
    pub port: u16,
    pub pid: u32,
    #[serde(flatten, default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, Value>,
}

/// Request for `codex stdio-to-uds <SOCKET_PATH>`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StdioToUdsRequest {
    /// Path to the Unix domain socket to connect to.
    pub socket_path: PathBuf,
    /// Optional working directory override for the spawned process.
    pub working_dir: Option<PathBuf>,
}

impl StdioToUdsRequest {
    pub fn new(socket_path: impl Into<PathBuf>) -> Self {
        Self {
            socket_path: socket_path.into(),
            working_dir: None,
        }
    }

    /// Sets the working directory used to resolve the socket path.
    pub fn working_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.working_dir = Some(dir.into());
        self
    }
}

/// Stage labels reported by `codex features list`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub enum CodexFeatureStage {
    Experimental,
    Beta,
    Stable,
    Deprecated,
    Removed,
    Unknown(String),
}

impl CodexFeatureStage {
    fn parse(raw: &str) -> Self {
        let normalized = raw.trim();
        match normalized.to_ascii_lowercase().as_str() {
            "experimental" => CodexFeatureStage::Experimental,
            "beta" => CodexFeatureStage::Beta,
            "stable" => CodexFeatureStage::Stable,
            "deprecated" => CodexFeatureStage::Deprecated,
            "removed" => CodexFeatureStage::Removed,
            _ => CodexFeatureStage::Unknown(normalized.to_string()),
        }
    }

    /// Returns the normalized label for this stage.
    pub fn as_str(&self) -> &str {
        match self {
            CodexFeatureStage::Experimental => "experimental",
            CodexFeatureStage::Beta => "beta",
            CodexFeatureStage::Stable => "stable",
            CodexFeatureStage::Deprecated => "deprecated",
            CodexFeatureStage::Removed => "removed",
            CodexFeatureStage::Unknown(label) => label.as_str(),
        }
    }
}

impl From<String> for CodexFeatureStage {
    fn from(value: String) -> Self {
        CodexFeatureStage::parse(&value)
    }
}

impl From<CodexFeatureStage> for String {
    fn from(stage: CodexFeatureStage) -> Self {
        String::from(&stage)
    }
}

impl From<&CodexFeatureStage> for String {
    fn from(stage: &CodexFeatureStage) -> Self {
        stage.as_str().to_string()
    }
}

/// Single feature entry reported by `codex features list`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CodexFeature {
    /// Feature name as reported by the CLI.
    pub name: String,
    /// Feature stage (experimental/beta/stable/deprecated/removed) when provided.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage: Option<CodexFeatureStage>,
    /// Whether the feature is enabled for the current config/profile.
    pub enabled: bool,
    /// Unrecognized fields from JSON output are preserved here.
    #[serde(flatten, default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, Value>,
}

impl CodexFeature {
    /// Convenience helper mirroring the `enabled` flag.
    pub const fn is_enabled(&self) -> bool {
        self.enabled
    }
}

/// Format used to parse `codex features list` output.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FeaturesListFormat {
    Json,
    Text,
}

/// Parsed output from `codex features list`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeaturesListOutput {
    /// Exit status returned by the subcommand.
    pub status: ExitStatus,
    /// Captured stdout (mirrored to the console when `mirror_stdout` is true).
    pub stdout: String,
    /// Captured stderr (mirrored unless `quiet` is set).
    pub stderr: String,
    /// Parsed feature entries.
    pub features: Vec<CodexFeature>,
    /// Indicates whether JSON or text parsing was used.
    pub format: FeaturesListFormat,
}

/// Request for `codex features list`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeaturesListRequest {
    /// Request JSON output via `--json` (falls back to text parsing when JSON is absent).
    pub json: bool,
    /// Per-call CLI overrides layered on top of the builder.
    pub overrides: CliOverridesPatch,
}

impl FeaturesListRequest {
    /// Creates a request with JSON disabled by default for compatibility with older binaries.
    pub fn new() -> Self {
        Self {
            json: false,
            overrides: CliOverridesPatch::default(),
        }
    }

    /// Controls whether `--json` is passed to `codex features list`.
    pub fn json(mut self, enable: bool) -> Self {
        self.json = enable;
        self
    }

    /// Replaces the default CLI overrides for this request.
    pub fn with_overrides(mut self, overrides: CliOverridesPatch) -> Self {
        self.overrides = overrides;
        self
    }

    /// Adds a `--config key=value` override for this request.
    pub fn config_override(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.overrides
            .config_overrides
            .push(ConfigOverride::new(key, value));
        self
    }

    /// Adds a raw `--config key=value` override without validation.
    pub fn config_override_raw(mut self, raw: impl Into<String>) -> Self {
        self.overrides
            .config_overrides
            .push(ConfigOverride::from_raw(raw));
        self
    }

    /// Sets the config profile (`--profile`) for this request.
    pub fn profile(mut self, profile: impl Into<String>) -> Self {
        let profile = profile.into();
        self.overrides.profile = (!profile.trim().is_empty()).then_some(profile);
        self
    }

    /// Requests the CLI `--oss` flag for this call.
    pub fn oss(mut self, enable: bool) -> Self {
        self.overrides.oss = if enable {
            FlagState::Enable
        } else {
            FlagState::Disable
        };
        self
    }

    /// Adds a `--enable <feature>` toggle for this call.
    pub fn enable_feature(mut self, name: impl Into<String>) -> Self {
        self.overrides.feature_toggles.enable.push(name.into());
        self
    }

    /// Adds a `--disable <feature>` toggle for this call.
    pub fn disable_feature(mut self, name: impl Into<String>) -> Self {
        self.overrides.feature_toggles.disable.push(name.into());
        self
    }

    /// Controls whether `--search` is passed through to Codex.
    pub fn search(mut self, enable: bool) -> Self {
        self.overrides.search = if enable {
            FlagState::Enable
        } else {
            FlagState::Disable
        };
        self
    }
}

impl Default for FeaturesListRequest {
    fn default() -> Self {
        Self::new()
    }
}

/// Decision returned by execpolicy evaluation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ExecPolicyDecision {
    Allow,
    Prompt,
    Forbidden,
}

/// Matched rule entry returned by `codex execpolicy check`.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecPolicyRuleMatch {
    /// Optional rule name/identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Human-readable description when provided by the policy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Decision attached to the rule. Defaults to [`ExecPolicyDecision::Allow`] when omitted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decision: Option<ExecPolicyDecision>,
    #[serde(flatten, default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, Value>,
}

/// Matched execpolicy summary with the merged decision and contributing rules.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecPolicyMatch {
    pub decision: ExecPolicyDecision,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rules: Vec<ExecPolicyRuleMatch>,
    #[serde(flatten, default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, Value>,
}

/// Response returned when no rules matched.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecPolicyNoMatch {
    #[serde(flatten, default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, Value>,
}

/// Parsed output from `codex execpolicy check`.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecPolicyEvaluation {
    #[serde(rename = "match", default, skip_serializing_if = "Option::is_none")]
    pub match_result: Option<ExecPolicyMatch>,
    #[serde(rename = "noMatch", default, skip_serializing_if = "Option::is_none")]
    pub no_match: Option<ExecPolicyNoMatch>,
}

impl ExecPolicyEvaluation {
    /// Returns the top-level decision when a policy matched.
    pub fn decision(&self) -> Option<ExecPolicyDecision> {
        self.match_result.as_ref().map(|result| result.decision)
    }
}

/// Captured output from `codex execpolicy check`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecPolicyCheckResult {
    /// Exit status returned by the subcommand.
    pub status: ExitStatus,
    /// Captured stdout (mirrored to the console when `mirror_stdout` is true).
    pub stdout: String,
    /// Captured stderr (mirrored unless `quiet` is set).
    pub stderr: String,
    /// Parsed decision JSON.
    pub evaluation: ExecPolicyEvaluation,
}

impl ExecPolicyCheckResult {
    /// Convenience accessor for the matched decision (if any).
    pub fn decision(&self) -> Option<ExecPolicyDecision> {
        self.evaluation.decision()
    }
}

/// Request to evaluate a command against Starlark execpolicy files.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecPolicyCheckRequest {
    /// One or more `.codexpolicy` files to merge with repeatable `--policy` flags.
    pub policies: Vec<PathBuf>,
    /// Pretty-print JSON output (`--pretty`).
    pub pretty: bool,
    /// Command argv forwarded after `--`. Must not be empty.
    pub command: Vec<OsString>,
    /// Per-call CLI overrides layered on top of the builder.
    pub overrides: CliOverridesPatch,
}

impl ExecPolicyCheckRequest {
    pub fn new<I, S>(command: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        Self {
            policies: Vec::new(),
            pretty: false,
            command: command.into_iter().map(Into::into).collect(),
            overrides: CliOverridesPatch::default(),
        }
    }

    /// Adds a single `--policy` path.
    pub fn policy(mut self, policy: impl Into<PathBuf>) -> Self {
        self.policies.push(policy.into());
        self
    }

    /// Adds multiple `--policy` paths.
    pub fn policies<I, P>(mut self, policies: I) -> Self
    where
        I: IntoIterator<Item = P>,
        P: Into<PathBuf>,
    {
        self.policies
            .extend(policies.into_iter().map(|policy| policy.into()));
        self
    }

    /// Controls whether `--pretty` is forwarded.
    pub fn pretty(mut self, enable: bool) -> Self {
        self.pretty = enable;
        self
    }

    /// Replaces the default CLI overrides for this request.
    pub fn with_overrides(mut self, overrides: CliOverridesPatch) -> Self {
        self.overrides = overrides;
        self
    }

    /// Adds a `--config key=value` override for this request.
    pub fn config_override(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.overrides
            .config_overrides
            .push(ConfigOverride::new(key, value));
        self
    }

    /// Adds a raw `--config key=value` override without validation.
    pub fn config_override_raw(mut self, raw: impl Into<String>) -> Self {
        self.overrides
            .config_overrides
            .push(ConfigOverride::from_raw(raw));
        self
    }

    /// Sets the config profile (`--profile`) for this request.
    pub fn profile(mut self, profile: impl Into<String>) -> Self {
        let profile = profile.into();
        self.overrides.profile = (!profile.trim().is_empty()).then_some(profile);
        self
    }

    /// Requests the CLI `--oss` flag for this call.
    pub fn oss(mut self, enable: bool) -> Self {
        self.overrides.oss = if enable {
            FlagState::Enable
        } else {
            FlagState::Disable
        };
        self
    }

    /// Adds a `--enable <feature>` toggle for this call.
    pub fn enable_feature(mut self, name: impl Into<String>) -> Self {
        self.overrides.feature_toggles.enable.push(name.into());
        self
    }

    /// Adds a `--disable <feature>` toggle for this call.
    pub fn disable_feature(mut self, name: impl Into<String>) -> Self {
        self.overrides.feature_toggles.disable.push(name.into());
        self
    }

    /// Controls whether `--search` is passed through to Codex.
    pub fn search(mut self, enable: bool) -> Self {
        self.overrides.search = if enable {
            FlagState::Enable
        } else {
            FlagState::Disable
        };
        self
    }
}

/// Target for app-server code generation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AppServerCodegenTarget {
    /// Emits TypeScript bindings for the app-server protocol. Optionally formats the output with Prettier.
    TypeScript { prettier: Option<PathBuf> },
    /// Emits a JSON schema bundle for the app-server protocol.
    JsonSchema,
}

impl AppServerCodegenTarget {
    fn subcommand(&self) -> &'static str {
        match self {
            AppServerCodegenTarget::TypeScript { .. } => "generate-ts",
            AppServerCodegenTarget::JsonSchema => "generate-json-schema",
        }
    }

    fn prettier(&self) -> Option<&PathBuf> {
        match self {
            AppServerCodegenTarget::TypeScript { prettier } => prettier.as_ref(),
            AppServerCodegenTarget::JsonSchema => None,
        }
    }
}

/// Request for `codex app-server generate-ts` or `generate-json-schema`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppServerCodegenRequest {
    /// Codegen target and optional Prettier path (TypeScript only).
    pub target: AppServerCodegenTarget,
    /// Output directory passed to `--out`; created if missing.
    pub out_dir: PathBuf,
    /// Per-call CLI overrides layered on top of the builder.
    pub overrides: CliOverridesPatch,
}

impl AppServerCodegenRequest {
    /// Generates TypeScript bindings into `out_dir`.
    pub fn typescript(out_dir: impl Into<PathBuf>) -> Self {
        Self {
            target: AppServerCodegenTarget::TypeScript { prettier: None },
            out_dir: out_dir.into(),
            overrides: CliOverridesPatch::default(),
        }
    }

    /// Generates a JSON schema bundle into `out_dir`.
    pub fn json_schema(out_dir: impl Into<PathBuf>) -> Self {
        Self {
            target: AppServerCodegenTarget::JsonSchema,
            out_dir: out_dir.into(),
            overrides: CliOverridesPatch::default(),
        }
    }

    /// Formats TypeScript output with the provided Prettier executable (no-op for JSON schema).
    pub fn prettier(mut self, prettier: impl Into<PathBuf>) -> Self {
        if let AppServerCodegenTarget::TypeScript { prettier: slot } = &mut self.target {
            *slot = Some(prettier.into());
        }
        self
    }

    /// Replaces the default CLI overrides for this request.
    pub fn with_overrides(mut self, overrides: CliOverridesPatch) -> Self {
        self.overrides = overrides;
        self
    }

    /// Adds a `--config key=value` override for this request.
    pub fn config_override(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.overrides
            .config_overrides
            .push(ConfigOverride::new(key, value));
        self
    }

    /// Adds a raw `--config key=value` override without validation.
    pub fn config_override_raw(mut self, raw: impl Into<String>) -> Self {
        self.overrides
            .config_overrides
            .push(ConfigOverride::from_raw(raw));
        self
    }

    /// Sets the config profile (`--profile`) for this request.
    pub fn profile(mut self, profile: impl Into<String>) -> Self {
        let profile = profile.into();
        self.overrides.profile = (!profile.trim().is_empty()).then_some(profile);
        self
    }

    /// Requests the CLI `--oss` flag for this codegen call.
    pub fn oss(mut self, enable: bool) -> Self {
        self.overrides.oss = if enable {
            FlagState::Enable
        } else {
            FlagState::Disable
        };
        self
    }

    /// Adds a `--enable <feature>` toggle for this codegen call.
    pub fn enable_feature(mut self, name: impl Into<String>) -> Self {
        self.overrides.feature_toggles.enable.push(name.into());
        self
    }

    /// Adds a `--disable <feature>` toggle for this codegen call.
    pub fn disable_feature(mut self, name: impl Into<String>) -> Self {
        self.overrides.feature_toggles.disable.push(name.into());
        self
    }

    /// Controls whether `--search` is passed through to Codex.
    pub fn search(mut self, enable: bool) -> Self {
        self.overrides.search = if enable {
            FlagState::Enable
        } else {
            FlagState::Disable
        };
        self
    }
}

/// Captured output from app-server codegen commands.
#[derive(Clone, Debug)]
pub struct AppServerCodegenOutput {
    /// Exit status returned by the subcommand.
    pub status: ExitStatus,
    /// Captured stdout (mirrored to the console when `mirror_stdout` is true).
    pub stdout: String,
    /// Captured stderr (mirrored unless `quiet` is set).
    pub stderr: String,
    /// Output directory passed to `--out`.
    pub out_dir: PathBuf,
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
    /// Exit status returned by the subcommand.
    pub status: ExitStatus,
    /// Captured stdout (mirrored to the console when `mirror_stdout` is true).
    pub stdout: String,
    /// Captured stderr (mirrored unless `quiet` is set).
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
    #[error("codex JSONL event missing required context: {message}: `{line}`")]
    Normalize { line: String, message: String },
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
    let mut context = StreamContext::default();
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

        let event = normalize_thread_event(&line, &mut context);
        if sender.send(event).await.is_err() {
            break;
        }
    }

    Ok(())
}

#[derive(Default)]
struct StreamContext {
    current_thread_id: Option<String>,
    current_turn_id: Option<String>,
    next_synthetic_turn: u32,
}

fn normalize_thread_event(
    line: &str,
    context: &mut StreamContext,
) -> Result<ThreadEvent, ExecStreamError> {
    let mut value: serde_json::Value =
        serde_json::from_str(line).map_err(|source| ExecStreamError::Parse {
            line: line.to_string(),
            source,
        })?;

    let event_type = value
        .get("type")
        .and_then(|t| t.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| ExecStreamError::Normalize {
            line: line.to_string(),
            message: "event missing `type`".to_string(),
        })?;

    match event_type.as_str() {
        "thread.started" | "thread.resumed" => {
            let thread_id = extract_str_from_keys(&value, &["thread_id", "conversation_id", "id"])
                .ok_or_else(|| missing(&event_type, "thread_id", line))?;
            context.current_thread_id = Some(thread_id.to_string());
            context.current_turn_id = None;
        }
        "turn.started" => {
            let turn_id = extract_str_from_keys(&value, &["turn_id", "id"])
                .map(|s| s.to_string())
                .unwrap_or_else(|| {
                    let id = format!("synthetic-turn-{}", context.next_synthetic_turn.max(1));
                    context.next_synthetic_turn = context.next_synthetic_turn.saturating_add(1);
                    id
                });
            let thread_id = extract_str_from_keys(&value, &["thread_id", "conversation_id"])
                .map(|s| s.to_string())
                .or_else(|| context.current_thread_id.clone())
                .ok_or_else(|| missing("turn.started", "thread_id", line))?;
            set_str(&mut value, "turn_id", turn_id.clone());
            set_str(&mut value, "thread_id", thread_id.clone());
            context.current_thread_id = Some(thread_id);
            context.current_turn_id = Some(turn_id);
        }
        "turn.completed" | "turn.failed" => {
            let turn_id = extract_str_from_keys(&value, &["turn_id", "id"])
                .map(|s| s.to_string())
                .or_else(|| context.current_turn_id.clone())
                .ok_or_else(|| missing(&event_type, "turn_id", line))?;
            let thread_id = extract_str_from_keys(&value, &["thread_id", "conversation_id"])
                .map(|s| s.to_string())
                .or_else(|| context.current_thread_id.clone())
                .ok_or_else(|| missing(&event_type, "thread_id", line))?;
            set_str(&mut value, "turn_id", turn_id.clone());
            set_str(&mut value, "thread_id", thread_id.clone());
            context.current_turn_id = None;
            context.current_thread_id = Some(thread_id);
        }
        t if t.starts_with("item.") => {
            normalize_item_payload(&mut value);
            if event_type == "item.delta" || event_type == "item.updated" {
                normalize_item_delta_payload(&mut value);
            }
            let turn_id = extract_str(&value, "turn_id")
                .map(|s| s.to_string())
                .or_else(|| context.current_turn_id.clone())
                .ok_or_else(|| missing(&event_type, "turn_id", line))?;
            let thread_id = extract_str_from_keys(&value, &["thread_id", "conversation_id"])
                .map(|s| s.to_string())
                .or_else(|| context.current_thread_id.clone())
                .ok_or_else(|| missing(&event_type, "thread_id", line))?;
            set_str(&mut value, "turn_id", turn_id);
            set_str(&mut value, "thread_id", thread_id);
        }
        _ => {}
    }

    serde_json::from_value::<ThreadEvent>(value).map_err(|source| ExecStreamError::Parse {
        line: line.to_string(),
        source,
    })
}

fn extract_str<'a>(value: &'a serde_json::Value, key: &str) -> Option<&'a str> {
    value
        .get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
}

fn extract_str_from_keys<'a>(value: &'a serde_json::Value, keys: &[&str]) -> Option<&'a str> {
    for key in keys {
        if let Some(found) = extract_str(value, key) {
            return Some(found);
        }
    }
    None
}

fn set_str(value: &mut serde_json::Value, key: &str, new_value: String) {
    if let Some(map) = value.as_object_mut() {
        map.insert(key.to_string(), serde_json::Value::String(new_value));
    }
}

fn normalize_item_delta_payload(value: &mut serde_json::Value) {
    let Some(map) = value.as_object_mut() else {
        return;
    };

    if !map.contains_key("delta") {
        if let Some(content) = map.remove("content") {
            map.insert("delta".to_string(), content);
        }
    }

    let Some(item_type) = map.get("item_type").and_then(|value| value.as_str()) else {
        return;
    };

    if !matches!(item_type, "agent_message" | "reasoning") {
        return;
    }

    let Some(delta) = map.get_mut("delta") else {
        return;
    };

    if let Some(text_delta) = delta.as_str() {
        *delta = serde_json::json!({ "text_delta": text_delta });
    }
}

fn normalize_item_payload(value: &mut serde_json::Value) {
    let mut item_object = match value
        .get_mut("item")
        .and_then(|item| item.as_object_mut())
        .map(|map| map.clone())
    {
        Some(map) => map,
        None => return,
    };

    if !item_object.contains_key("item_type") {
        if let Some(item_type) = item_object.remove("type") {
            item_object.insert("item_type".to_string(), item_type);
        }
    }

    if !item_object.contains_key("content") {
        let mut content: Option<serde_json::Value> = None;
        if let Some(text) = item_object.remove("text") {
            if let Some(text_str) = text.as_str() {
                content = Some(serde_json::json!({ "text": text_str }));
            } else {
                content = Some(text);
            }
        } else if let Some(command) = item_object.get("command").cloned() {
            let mut map = serde_json::Map::new();
            map.insert("command".to_string(), command);
            if let Some(stdout) = item_object.remove("aggregated_output") {
                map.insert("stdout".to_string(), stdout);
            }
            if let Some(exit_code) = item_object.remove("exit_code") {
                map.insert("exit_code".to_string(), exit_code);
            }
            if let Some(stderr) = item_object.remove("stderr") {
                map.insert("stderr".to_string(), stderr);
            }
            content = Some(serde_json::Value::Object(map));
        }

        if let Some(content_value) = content {
            item_object.insert("content".to_string(), content_value);
        }
    }

    let item_type = item_object
        .get("item_type")
        .and_then(|value| value.as_str())
        .or_else(|| item_object.get("type").and_then(|value| value.as_str()))
        .map(|value| value.to_string());

    if matches!(item_type.as_deref(), Some("agent_message" | "reasoning")) {
        if let Some(content) = item_object.get_mut("content") {
            if let Some(text) = content.as_str() {
                *content = serde_json::json!({ "text": text });
            }
        }
    }

    if let Some(root) = value.as_object_mut() {
        for (mut key, mut v) in item_object {
            if key == "type" {
                key = "item_type".to_string();
            }
            root.insert(key, v.take());
        }
        root.remove("item");
    }
}

fn missing(event: &str, field: &str, line: &str) -> ExecStreamError {
    ExecStreamError::Normalize {
        line: line.to_string(),
        message: format!("{event} missing `{field}` and no prior context to infer it"),
    }
}

async fn read_last_message(path: &Path) -> Option<String> {
    (fs::read_to_string(path).await).ok()
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

fn parse_feature_list_output(
    stdout: &str,
    prefer_json: bool,
) -> Result<(Vec<CodexFeature>, FeaturesListFormat), String> {
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return Err("features list output was empty".to_string());
    }

    if prefer_json {
        if let Some(features) = parse_feature_list_json(trimmed) {
            if !features.is_empty() {
                return Ok((features, FeaturesListFormat::Json));
            }
        }
        if let Some(features) = parse_feature_list_text(trimmed) {
            if !features.is_empty() {
                return Ok((features, FeaturesListFormat::Text));
            }
        }
    } else {
        if let Some(features) = parse_feature_list_text(trimmed) {
            if !features.is_empty() {
                return Ok((features, FeaturesListFormat::Text));
            }
        }
        if let Some(features) = parse_feature_list_json(trimmed) {
            if !features.is_empty() {
                return Ok((features, FeaturesListFormat::Json));
            }
        }
    }

    Err("could not parse JSON or text feature rows".to_string())
}

fn parse_feature_list_json(output: &str) -> Option<Vec<CodexFeature>> {
    let parsed: Value = serde_json::from_str(output).ok()?;
    parse_feature_list_json_value(&parsed)
}

fn parse_feature_list_json_value(value: &Value) -> Option<Vec<CodexFeature>> {
    match value {
        Value::Array(items) => Some(
            items
                .iter()
                .filter_map(|item| match item {
                    Value::Object(map) => feature_from_json_fields(None, map),
                    Value::String(name) => Some(CodexFeature {
                        name: name.clone(),
                        stage: None,
                        enabled: true,
                        extra: BTreeMap::new(),
                    }),
                    _ => None,
                })
                .collect(),
        ),
        Value::Object(map) => {
            if let Some(features) = map.get("features") {
                return parse_feature_list_json_value(features);
            }
            if map.contains_key("name") || map.contains_key("enabled") || map.contains_key("stage")
            {
                return feature_from_json_fields(None, map).map(|feature| vec![feature]);
            }
            Some(
                map.iter()
                    .filter_map(|(name, value)| match value {
                        Value::Object(inner) => {
                            feature_from_json_fields(Some(name.as_str()), inner)
                        }
                        Value::Bool(flag) => Some(CodexFeature {
                            name: name.clone(),
                            stage: None,
                            enabled: *flag,
                            extra: BTreeMap::new(),
                        }),
                        Value::String(flag) => parse_feature_enabled_str(flag)
                            .map(|enabled| CodexFeature {
                                name: name.clone(),
                                stage: None,
                                enabled,
                                extra: BTreeMap::new(),
                            })
                            .or_else(|| {
                                Some(CodexFeature {
                                    name: name.clone(),
                                    stage: Some(CodexFeatureStage::parse(flag)),
                                    enabled: true,
                                    extra: BTreeMap::new(),
                                })
                            }),
                        _ => None,
                    })
                    .collect(),
            )
        }
        _ => None,
    }
}

fn parse_feature_list_text(output: &str) -> Option<Vec<CodexFeature>> {
    let mut features = Vec::new();
    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed
            .chars()
            .all(|c| matches!(c, '-' | '=' | '+' | '*' | '|'))
        {
            continue;
        }

        let tokens: Vec<&str> = trimmed.split_whitespace().collect();
        if tokens.len() < 3 {
            continue;
        }
        if tokens[0].eq_ignore_ascii_case("feature")
            && tokens[1].eq_ignore_ascii_case("stage")
            && tokens[2].eq_ignore_ascii_case("enabled")
        {
            continue;
        }

        let enabled_token = tokens.last().copied().unwrap_or_default();
        let enabled = match parse_feature_enabled_str(enabled_token) {
            Some(value) => value,
            None => continue,
        };
        let stage_token = tokens.get(tokens.len() - 2).copied().unwrap_or_default();
        let name = tokens[..tokens.len() - 2].join(" ");
        if name.is_empty() {
            continue;
        }
        let stage = (!stage_token.is_empty()).then(|| CodexFeatureStage::parse(stage_token));
        features.push(CodexFeature {
            name,
            stage,
            enabled,
            extra: BTreeMap::new(),
        });
    }

    if features.is_empty() {
        None
    } else {
        Some(features)
    }
}

fn parse_feature_enabled_value(value: &Value) -> Option<bool> {
    match value {
        Value::Bool(flag) => Some(*flag),
        Value::String(raw) => parse_feature_enabled_str(raw),
        _ => None,
    }
}

fn parse_feature_enabled_str(raw: &str) -> Option<bool> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "true" | "yes" | "y" | "on" | "1" | "enabled" => Some(true),
        "false" | "no" | "n" | "off" | "0" | "disabled" => Some(false),
        _ => None,
    }
}

fn feature_from_json_fields(
    name_hint: Option<&str>,
    map: &serde_json::Map<String, Value>,
) -> Option<CodexFeature> {
    let name = map
        .get("name")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| name_hint.map(str::to_string))?;
    let enabled = map
        .get("enabled")
        .and_then(parse_feature_enabled_value)
        .or_else(|| map.get("value").and_then(parse_feature_enabled_value))?;
    let stage = map
        .get("stage")
        .or_else(|| map.get("status"))
        .and_then(Value::as_str)
        .map(CodexFeatureStage::parse);

    let mut extra = BTreeMap::new();
    for (key, value) in map {
        if matches!(
            key.as_str(),
            "name" | "stage" | "status" | "enabled" | "value"
        ) {
            continue;
        }
        extra.insert(key.clone(), value.clone());
    }

    Some(CodexFeature {
        name,
        stage,
        enabled,
        extra,
    })
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
    use futures_util::{pin_mut, StreamExt};
    use serde_json::json;
    use std::collections::HashMap;
    use std::fs as std_fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use std::sync::OnceLock;
    use std::time::{Duration, SystemTime};
    use tokio::{fs, io::AsyncWriteExt};

    fn env_mutex() -> &'static tokio::sync::Mutex<()> {
        static ENV_MUTEX: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
        ENV_MUTEX.get_or_init(|| tokio::sync::Mutex::new(()))
    }

    fn env_guard() -> tokio::sync::MutexGuard<'static, ()> {
        env_mutex().blocking_lock()
    }

    async fn env_guard_async() -> tokio::sync::MutexGuard<'static, ()> {
        env_mutex().lock().await
    }

    #[tokio::test]
    async fn json_stream_preserves_order_and_parses_tool_calls() {
        let lines = [
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
        let lines = [
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

    fn write_executable(dir: &Path, name: &str, script: &str) -> PathBuf {
        let path = dir.join(name);
        std_fs::write(&path, script).unwrap();
        let mut perms = std_fs::metadata(&path).unwrap().permissions();
        #[cfg(unix)]
        {
            perms.set_mode(0o755);
        }
        std_fs::set_permissions(&path, perms).unwrap();
        path
    }

    fn write_fake_codex(dir: &Path, script: &str) -> PathBuf {
        write_executable(dir, "codex", script)
    }

    fn write_fake_bundled_codex(dir: &Path, platform: &str, script: &str) -> PathBuf {
        write_executable(dir, bundled_binary_filename(platform), script)
    }

    #[test]
    fn resolve_bundled_binary_defaults_to_runtime_platform() {
        let temp = tempfile::tempdir().unwrap();
        let platform = default_bundled_platform_label();
        let version = "1.2.3";
        let version_dir = temp.path().join(&platform).join(version);
        std_fs::create_dir_all(&version_dir).unwrap();
        let binary =
            write_fake_bundled_codex(&version_dir, &platform, "#!/usr/bin/env bash\necho ok");

        let resolved = resolve_bundled_binary(BundledBinarySpec {
            bundle_root: temp.path(),
            version,
            platform: None,
        })
        .unwrap();

        assert_eq!(resolved.platform, platform);
        assert_eq!(resolved.version, version);
        assert_eq!(resolved.binary_path, std_fs::canonicalize(&binary).unwrap());
    }

    #[test]
    fn resolve_bundled_binary_honors_platform_override() {
        let temp = tempfile::tempdir().unwrap();
        let platform = "windows-x64";
        let version = "5.6.7";
        let version_dir = temp.path().join(platform).join(version);
        std_fs::create_dir_all(&version_dir).unwrap();
        let binary =
            write_fake_bundled_codex(&version_dir, platform, "#!/usr/bin/env bash\necho win");

        let resolved = resolve_bundled_binary(BundledBinarySpec {
            bundle_root: temp.path(),
            version,
            platform: Some(platform),
        })
        .unwrap();

        assert_eq!(resolved.platform, platform);
        assert_eq!(resolved.version, version);
        assert_eq!(resolved.binary_path, std_fs::canonicalize(&binary).unwrap());
        assert_eq!(
            resolved
                .binary_path
                .file_name()
                .and_then(|name| name.to_str()),
            Some("codex.exe")
        );
    }

    #[test]
    fn resolve_bundled_binary_errors_when_binary_missing() {
        let temp = tempfile::tempdir().unwrap();
        let platform = default_bundled_platform_label();
        let version = "0.0.1";
        let version_dir = temp.path().join(&platform).join(version);
        std_fs::create_dir_all(&version_dir).unwrap();

        let err = resolve_bundled_binary(BundledBinarySpec {
            bundle_root: temp.path(),
            version,
            platform: None,
        })
        .unwrap_err();

        match err {
            BundledBinaryError::BinaryUnreadable { binary, .. }
            | BundledBinaryError::BinaryNotFile { binary }
            | BundledBinaryError::BinaryNotExecutable { binary } => {
                assert_eq!(binary, version_dir.join(bundled_binary_filename(&platform)));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn resolve_bundled_binary_rejects_empty_version() {
        let temp = tempfile::tempdir().unwrap();
        let err = resolve_bundled_binary(BundledBinarySpec {
            bundle_root: temp.path(),
            version: "  ",
            platform: None,
        })
        .unwrap_err();
        assert!(matches!(err, BundledBinaryError::EmptyVersion));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn sandbox_maps_platform_flags_and_command() {
        let dir = tempfile::tempdir().unwrap();
        let script_path = write_fake_codex(
            dir.path(),
            r#"#!/usr/bin/env bash
echo "$PWD"
printf "%s\n" "$@"
"#,
        );

        let client = CodexClient::builder()
            .binary(&script_path)
            .mirror_stdout(false)
            .quiet(true)
            .build();

        let request = SandboxCommandRequest::new(
            SandboxPlatform::Linux,
            [OsString::from("echo"), OsString::from("hello world")],
        )
        .full_auto(true)
        .log_denials(true)
        .config_override("foo", "bar")
        .enable_feature("alpha")
        .disable_feature("beta");

        let run = client.run_sandbox(request).await.unwrap();
        let mut lines = run.stdout.lines();
        let pwd = lines.next().unwrap();
        assert_eq!(Path::new(pwd), env::current_dir().unwrap().as_path());

        let args: Vec<_> = lines.map(str::to_string).collect();
        assert!(!args.contains(&"--log-denials".to_string()));
        assert_eq!(
            args,
            vec![
                "sandbox",
                "linux",
                "--full-auto",
                "--config",
                "foo=bar",
                "--enable",
                "alpha",
                "--disable",
                "beta",
                "--",
                "echo",
                "hello world"
            ]
        );
        assert!(run.status.success());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn sandbox_includes_log_denials_on_macos() {
        let dir = tempfile::tempdir().unwrap();
        let script_path = write_fake_codex(
            dir.path(),
            r#"#!/usr/bin/env bash
printf "%s\n" "$@"
"#,
        );

        let client = CodexClient::builder()
            .binary(&script_path)
            .mirror_stdout(false)
            .quiet(true)
            .build();

        let run = client
            .run_sandbox(
                SandboxCommandRequest::new(SandboxPlatform::Macos, ["ls"]).log_denials(true),
            )
            .await
            .unwrap();
        let args: Vec<_> = run.stdout.lines().collect();
        assert!(args.contains(&"--log-denials"));
        assert_eq!(args[0], "sandbox");
        assert_eq!(args[1], "macos");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn sandbox_honors_working_dir_precedence() {
        let dir = tempfile::tempdir().unwrap();
        let script_path = write_fake_codex(
            dir.path(),
            r#"#!/usr/bin/env bash
echo "$PWD"
"#,
        );

        let request_dir = dir.path().join("request_cwd");
        let builder_dir = dir.path().join("builder_cwd");
        std_fs::create_dir_all(&request_dir).unwrap();
        std_fs::create_dir_all(&builder_dir).unwrap();

        let client = CodexClient::builder()
            .binary(&script_path)
            .mirror_stdout(false)
            .quiet(true)
            .working_dir(&builder_dir)
            .build();

        let run_request = client
            .run_sandbox(
                SandboxCommandRequest::new(SandboxPlatform::Windows, ["echo", "cwd"])
                    .working_dir(&request_dir),
            )
            .await
            .unwrap();
        let request_pwd = run_request.stdout.lines().next().unwrap();
        assert_eq!(Path::new(request_pwd), request_dir.as_path());

        let run_builder = client
            .run_sandbox(SandboxCommandRequest::new(
                SandboxPlatform::Windows,
                ["echo", "builder"],
            ))
            .await
            .unwrap();
        let builder_pwd = run_builder.stdout.lines().next().unwrap();
        assert_eq!(Path::new(builder_pwd), builder_dir.as_path());

        let client_default = CodexClient::builder()
            .binary(&script_path)
            .mirror_stdout(false)
            .quiet(true)
            .build();
        let run_default = client_default
            .run_sandbox(SandboxCommandRequest::new(
                SandboxPlatform::Windows,
                ["echo", "default"],
            ))
            .await
            .unwrap();
        let default_pwd = run_default.stdout.lines().next().unwrap();
        assert_eq!(
            Path::new(default_pwd),
            env::current_dir().unwrap().as_path()
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn sandbox_returns_non_zero_status_without_error() {
        let dir = tempfile::tempdir().unwrap();
        let script_path = write_fake_codex(
            dir.path(),
            r#"#!/usr/bin/env bash
echo "failing"
exit 7
"#,
        );

        let client = CodexClient::builder()
            .binary(&script_path)
            .mirror_stdout(false)
            .quiet(true)
            .build();
        let run = client
            .run_sandbox(SandboxCommandRequest::new(
                SandboxPlatform::Linux,
                ["false"],
            ))
            .await
            .unwrap();

        assert!(!run.status.success());
        assert_eq!(run.status.code(), Some(7));
        assert_eq!(run.stdout.trim(), "failing");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn execpolicy_maps_policies_and_overrides() {
        let dir = tempfile::tempdir().unwrap();
        let script_path = dir.path().join("codex-execpolicy");
        std_fs::write(
            &script_path,
            r#"#!/usr/bin/env bash
printf "%s\n" "$PWD" "$@" 1>&2
cat <<'JSON'
{"match":{"decision":"prompt","rules":[{"name":"rule1","decision":"forbidden"}]}}
JSON
"#,
        )
        .unwrap();
        let mut perms = std_fs::metadata(&script_path).unwrap().permissions();
        perms.set_mode(0o755);
        std_fs::set_permissions(&script_path, perms).unwrap();

        let workdir = dir.path().join("workdir");
        std_fs::create_dir_all(&workdir).unwrap();
        let policy_one = dir.path().join("policy_a.codexpolicy");
        let policy_two = dir.path().join("policy_b.codexpolicy");
        std_fs::write(&policy_one, "").unwrap();
        std_fs::write(&policy_two, "").unwrap();

        let client = CodexClient::builder()
            .binary(&script_path)
            .mirror_stdout(false)
            .quiet(true)
            .working_dir(&workdir)
            .approval_policy(ApprovalPolicy::OnRequest)
            .build();

        let result = client
            .check_execpolicy(
                ExecPolicyCheckRequest::new([
                    OsString::from("bash"),
                    OsString::from("-lc"),
                    OsString::from("echo ok"),
                ])
                .policies([&policy_one, &policy_two])
                .pretty(true)
                .profile("dev")
                .config_override("features.execpolicy", "true"),
            )
            .await
            .unwrap();

        assert_eq!(result.decision(), Some(ExecPolicyDecision::Prompt));
        let match_result = result.evaluation.match_result.unwrap();
        assert_eq!(match_result.rules.len(), 1);
        assert_eq!(match_result.rules[0].name.as_deref(), Some("rule1"));
        assert_eq!(
            match_result.rules[0].decision,
            Some(ExecPolicyDecision::Forbidden)
        );

        let mut lines = result.stderr.lines();
        let pwd = lines.next().unwrap();
        assert_eq!(Path::new(pwd), workdir.as_path());

        let args: Vec<_> = lines.map(str::to_string).collect();
        assert_eq!(
            args,
            vec![
                "execpolicy",
                "check",
                "--policy",
                policy_one.to_string_lossy().as_ref(),
                "--policy",
                policy_two.to_string_lossy().as_ref(),
                "--pretty",
                "--config",
                "features.execpolicy=true",
                "--profile",
                "dev",
                "--ask-for-approval",
                "on-request",
                "--",
                "bash",
                "-lc",
                "echo ok"
            ]
        );
    }

    #[tokio::test]
    async fn execpolicy_rejects_empty_command() {
        let client = CodexClient::builder().build();
        let request = ExecPolicyCheckRequest::new(Vec::<OsString>::new());
        let err = client.check_execpolicy(request).await.unwrap_err();
        assert!(matches!(err, CodexError::EmptyExecPolicyCommand));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn execpolicy_surfaces_parse_errors() {
        let dir = tempfile::tempdir().unwrap();
        let script_path = dir.path().join("codex-execpolicy-bad");
        std_fs::write(
            &script_path,
            r#"#!/usr/bin/env bash
echo "not-json"
"#,
        )
        .unwrap();
        let mut perms = std_fs::metadata(&script_path).unwrap().permissions();
        perms.set_mode(0o755);
        std_fs::set_permissions(&script_path, perms).unwrap();

        let client = CodexClient::builder()
            .binary(&script_path)
            .mirror_stdout(false)
            .quiet(true)
            .build();

        let err = client
            .check_execpolicy(
                ExecPolicyCheckRequest::new([OsString::from("echo"), OsString::from("noop")])
                    .policy(dir.path().join("policy.codexpolicy")),
            )
            .await
            .unwrap_err();

        match err {
            CodexError::ExecPolicyParse { stdout, .. } => assert!(stdout.contains("not-json")),
            other => panic!("expected ExecPolicyParse, got {other:?}"),
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn features_list_maps_overrides_and_json_flag() {
        let dir = tempfile::tempdir().unwrap();
        let script_path = write_fake_codex(
            dir.path(),
            r#"#!/usr/bin/env bash
echo "$PWD" 1>&2
printf "%s\n" "$@" 1>&2
cat <<'JSON'
[{"name":"json-stream","stage":"stable","enabled":true},{"name":"cloud-exec","stage":"experimental","enabled":false}]
JSON
"#,
        );

        let workdir = dir.path().join("features-workdir");
        std_fs::create_dir_all(&workdir).unwrap();

        let client = CodexClient::builder()
            .binary(&script_path)
            .mirror_stdout(false)
            .quiet(true)
            .working_dir(&workdir)
            .approval_policy(ApprovalPolicy::OnRequest)
            .search(true)
            .build();

        let output = client
            .list_features(
                FeaturesListRequest::new()
                    .json(true)
                    .profile("dev")
                    .config_override("features.extras", "true"),
            )
            .await
            .unwrap();

        assert_eq!(output.format, FeaturesListFormat::Json);
        assert_eq!(output.features.len(), 2);
        assert_eq!(output.features[0].stage, Some(CodexFeatureStage::Stable));
        assert!(output.features[0].enabled);
        assert!(!output.features[1].enabled);

        let mut lines = output.stderr.lines();
        let pwd = lines.next().unwrap();
        assert_eq!(Path::new(pwd), workdir.as_path());

        let args: Vec<_> = lines.map(str::to_string).collect();
        assert_eq!(
            args,
            vec![
                "features",
                "list",
                "--config",
                "features.extras=true",
                "--profile",
                "dev",
                "--ask-for-approval",
                "on-request",
                "--search",
                "--json"
            ]
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn app_server_codegen_maps_overrides_and_prettier() {
        let dir = tempfile::tempdir().unwrap();
        let script_path = write_fake_codex(
            dir.path(),
            r#"#!/usr/bin/env bash
echo "$PWD"
printf "%s\n" "$@"
"#,
        );

        let workdir = dir.path().join("workdir");
        std_fs::create_dir_all(&workdir).unwrap();
        let out_dir = dir.path().join("out/ts");
        let prettier = dir.path().join("bin/prettier.js");

        let client = CodexClient::builder()
            .binary(&script_path)
            .mirror_stdout(false)
            .quiet(true)
            .working_dir(&workdir)
            .approval_policy(ApprovalPolicy::OnRequest)
            .search(true)
            .build();

        let result = client
            .generate_app_server_bindings(
                AppServerCodegenRequest::typescript(&out_dir)
                    .prettier(&prettier)
                    .profile("dev")
                    .config_override("features.codegen", "true"),
            )
            .await
            .unwrap();

        let mut lines = result.stdout.lines();
        let pwd = lines.next().unwrap();
        assert_eq!(Path::new(pwd), workdir.as_path());

        let args: Vec<_> = lines.map(str::to_string).collect();
        assert_eq!(
            args,
            vec![
                "app-server",
                "generate-ts",
                "--out",
                out_dir.to_string_lossy().as_ref(),
                "--config",
                "features.codegen=true",
                "--profile",
                "dev",
                "--ask-for-approval",
                "on-request",
                "--search",
                "--prettier",
                prettier.to_string_lossy().as_ref(),
            ]
        );
        assert!(out_dir.is_dir());
        assert_eq!(result.out_dir, out_dir);
        assert!(result.status.success());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn app_server_codegen_surfaces_non_zero_exit() {
        let dir = tempfile::tempdir().unwrap();
        let script_path = write_fake_codex(
            dir.path(),
            r#"#!/usr/bin/env bash
echo "ts error"
echo "bad format" 1>&2
exit 5
"#,
        );

        let client = CodexClient::builder()
            .binary(&script_path)
            .mirror_stdout(false)
            .quiet(true)
            .build();

        let out_dir = dir.path().join("schema");
        let err = client
            .generate_app_server_bindings(AppServerCodegenRequest::json_schema(&out_dir))
            .await
            .unwrap_err();

        match err {
            CodexError::NonZeroExit { status, stderr } => {
                assert_eq!(status.code(), Some(5));
                assert!(stderr.contains("bad format"));
            }
            other => panic!("expected NonZeroExit, got {other:?}"),
        }
        assert!(out_dir.is_dir());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn responses_api_proxy_maps_flags_and_parses_server_info() {
        let dir = tempfile::tempdir().unwrap();
        let server_info = dir.path().join("server-info.json");
        let script_path = write_fake_codex(
            dir.path(),
            r#"#!/usr/bin/env bash
echo "$PWD"
printf "%s\n" "$@"
info_path=""
while [[ $# -gt 0 ]]; do
  if [[ $1 == "--server-info" ]]; then
    info_path=$2
  fi
  shift
done
read -r key || exit 1
echo "key:${key}"
if [[ -n "$info_path" ]]; then
  printf '{"port":4567,"pid":1234}\n' > "$info_path"
fi
"#,
        );

        let workdir = dir.path().join("responses-workdir");
        std_fs::create_dir_all(&workdir).unwrap();

        let client = CodexClient::builder()
            .binary(&script_path)
            .mirror_stdout(false)
            .quiet(true)
            .working_dir(&workdir)
            .build();

        let mut proxy = client
            .start_responses_api_proxy(
                ResponsesApiProxyRequest::new("sk-test-123")
                    .port(8080)
                    .server_info(&server_info)
                    .http_shutdown(true)
                    .upstream_url("https://example.com/v1/responses"),
            )
            .await
            .unwrap();

        assert_eq!(
            proxy.server_info_path.as_deref(),
            Some(server_info.as_path())
        );

        let stdout = proxy.child.stdout.take().unwrap();
        let mut lines = BufReader::new(stdout).lines();

        let pwd = lines.next_line().await.unwrap().unwrap();
        assert_eq!(Path::new(&pwd), workdir.as_path());

        let mut args = Vec::new();
        for _ in 0..8 {
            args.push(lines.next_line().await.unwrap().unwrap());
        }
        assert_eq!(
            args,
            vec![
                "responses-api-proxy",
                "--port",
                "8080",
                "--server-info",
                server_info.to_string_lossy().as_ref(),
                "--http-shutdown",
                "--upstream-url",
                "https://example.com/v1/responses",
            ]
        );

        let api_key_line = lines.next_line().await.unwrap().unwrap();
        assert_eq!(api_key_line, "key:sk-test-123");

        let info = proxy.read_server_info().await.unwrap().unwrap();
        assert_eq!(info.port, 4567);
        assert_eq!(info.pid, 1234);

        let status = proxy.child.wait().await.unwrap();
        assert!(status.success());
    }

    #[tokio::test]
    async fn responses_api_proxy_rejects_empty_api_key() {
        let client = CodexClient::builder().build();
        let err = client
            .start_responses_api_proxy(ResponsesApiProxyRequest::new("  "))
            .await
            .unwrap_err();
        assert!(matches!(err, CodexError::EmptyApiKey));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn stdio_to_uds_maps_args_and_pipes_stdio() {
        let dir = tempfile::tempdir().unwrap();
        let socket_path = dir.path().join("bridge.sock");
        let script_path = write_fake_codex(
            dir.path(),
            r#"#!/usr/bin/env bash
echo "$PWD"
printf "%s\n" "$@"
while read -r line; do
  echo "relay:${line}"
done
"#,
        );

        let workdir = dir.path().join("uds-workdir");
        std_fs::create_dir_all(&workdir).unwrap();

        let client = CodexClient::builder()
            .binary(&script_path)
            .mirror_stdout(false)
            .quiet(true)
            .working_dir(&workdir)
            .build();

        let request = StdioToUdsRequest::new(&socket_path).working_dir(&workdir);
        let mut child = match client.stdio_to_uds(request.clone()) {
            Ok(child) => child,
            Err(CodexError::Spawn { source, .. }) if source.raw_os_error() == Some(26) => {
                time::sleep(Duration::from_millis(25)).await;
                client.stdio_to_uds(request).unwrap()
            }
            Err(other) => panic!("unexpected spawn error: {other:?}"),
        };

        let stdout = child.stdout.take().unwrap();
        let mut lines = BufReader::new(stdout).lines();

        let pwd = lines.next_line().await.unwrap().unwrap();
        assert_eq!(Path::new(&pwd), workdir.as_path());

        let arg_one = lines.next_line().await.unwrap().unwrap();
        let arg_two = lines.next_line().await.unwrap().unwrap();
        assert_eq!(arg_one, "stdio-to-uds");
        assert_eq!(arg_two, socket_path.to_string_lossy().as_ref());

        let mut stdin = child.stdin.take().unwrap();
        stdin.write_all(b"ping\n").await.unwrap();
        stdin.shutdown().await.unwrap();
        drop(stdin);

        let echoed = lines.next_line().await.unwrap().unwrap();
        assert_eq!(echoed, "relay:ping");

        let status = time::timeout(Duration::from_secs(5), child.wait())
            .await
            .expect("stdio-to-uds wait timed out")
            .unwrap();
        assert!(status.success());
    }

    #[tokio::test]
    async fn stdio_to_uds_rejects_empty_socket_path() {
        let client = CodexClient::builder().build();
        let err = client
            .stdio_to_uds(StdioToUdsRequest::new(PathBuf::new()))
            .unwrap_err();
        assert!(matches!(err, CodexError::EmptySocketPath));
    }

    #[tokio::test]
    async fn sandbox_rejects_empty_command() {
        let client = CodexClient::builder().build();
        let request = SandboxCommandRequest::new(SandboxPlatform::Linux, Vec::<OsString>::new());
        let err = client.run_sandbox(request).await.unwrap_err();
        assert!(matches!(err, CodexError::EmptySandboxCommand));
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

    fn capabilities_with_feature_flags(features: CodexFeatureFlags) -> CodexCapabilities {
        CodexCapabilities {
            cache_key: CapabilityCacheKey {
                binary_path: PathBuf::from("codex"),
            },
            fingerprint: None,
            version: None,
            features,
            probe_plan: CapabilityProbePlan::default(),
            collected_at: SystemTime::now(),
        }
    }

    fn sample_capabilities_snapshot() -> CodexCapabilities {
        CodexCapabilities {
            cache_key: CapabilityCacheKey {
                binary_path: PathBuf::from("/tmp/codex"),
            },
            fingerprint: Some(BinaryFingerprint {
                canonical_path: Some(PathBuf::from("/tmp/codex")),
                modified: Some(SystemTime::UNIX_EPOCH + Duration::from_secs(5)),
                len: Some(1234),
            }),
            version: Some(CodexVersionInfo {
                raw: "codex 3.4.5-beta (commit cafe)".to_string(),
                semantic: Some((3, 4, 5)),
                commit: Some("cafe".to_string()),
                channel: CodexReleaseChannel::Beta,
            }),
            features: CodexFeatureFlags {
                supports_features_list: true,
                supports_output_schema: true,
                supports_add_dir: false,
                supports_mcp_login: true,
            },
            probe_plan: CapabilityProbePlan {
                steps: vec![
                    CapabilityProbeStep::VersionFlag,
                    CapabilityProbeStep::FeaturesListJson,
                    CapabilityProbeStep::ManualOverride,
                ],
            },
            collected_at: SystemTime::UNIX_EPOCH + Duration::from_secs(10),
        }
    }

    fn sample_capability_overrides() -> CapabilityOverrides {
        CapabilityOverrides {
            snapshot: Some(sample_capabilities_snapshot()),
            version: Some(parse_version_output("codex 9.9.9-nightly")),
            features: CapabilityFeatureOverrides {
                supports_features_list: Some(true),
                supports_output_schema: Some(true),
                supports_add_dir: Some(true),
                supports_mcp_login: None,
            },
        }
    }

    fn capability_snapshot_with_metadata(
        collected_at: SystemTime,
        fingerprint: Option<BinaryFingerprint>,
    ) -> CodexCapabilities {
        CodexCapabilities {
            cache_key: CapabilityCacheKey {
                binary_path: PathBuf::from("/tmp/codex"),
            },
            fingerprint,
            version: None,
            features: CodexFeatureFlags::default(),
            probe_plan: CapabilityProbePlan::default(),
            collected_at,
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
        assert!(builder.json_event_log.is_none());
        assert!(builder.cli_overrides.config_overrides.is_empty());
        assert!(!builder.cli_overrides.reasoning.has_overrides());
        assert!(builder.cli_overrides.approval_policy.is_none());
        assert!(builder.cli_overrides.sandbox_mode.is_none());
        assert_eq!(
            builder.cli_overrides.safety_override,
            SafetyOverride::Inherit
        );
        assert!(builder.cli_overrides.cd.is_none());
        assert!(builder.cli_overrides.local_provider.is_none());
        assert_eq!(builder.cli_overrides.search, FlagState::Inherit);
        assert!(builder.cli_overrides.auto_reasoning_defaults);
        assert!(builder.capability_overrides.is_empty());
        assert_eq!(
            builder.capability_cache_policy,
            CapabilityCachePolicy::PreferCache
        );
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
    fn default_binary_falls_back_when_env_missing() {
        let _guard = env_guard();
        let key = CODEX_BINARY_ENV;
        let original = env::var_os(key);
        env::remove_var(key);

        assert_eq!(default_binary_path(), PathBuf::from("codex"));

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
    fn command_env_applies_home_and_binary_per_command() {
        let _guard = env_guard();
        let binary_key = CODEX_BINARY_ENV;
        let home_key = CODEX_HOME_ENV;
        let rust_log_key = RUST_LOG_ENV;
        let original_binary = env::var_os(binary_key);
        let original_home = env::var_os(home_key);
        let original_rust_log = env::var_os(rust_log_key);

        env::set_var(binary_key, "/tmp/ignored_codex");
        env::set_var(home_key, "/tmp/ambient_home");
        env::remove_var(rust_log_key);

        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().join("scoped_home");
        let env_prep = CommandEnvironment::new(
            PathBuf::from("/app/bundled/codex"),
            Some(home.clone()),
            true,
        );

        let mut command = Command::new("echo");
        env_prep.apply(&mut command).unwrap();

        let envs: HashMap<OsString, Option<OsString>> = command
            .as_std()
            .get_envs()
            .map(|(key, value)| (key.to_os_string(), value.map(|v| v.to_os_string())))
            .collect();

        assert_eq!(
            envs.get(&OsString::from(binary_key)),
            Some(&Some(OsString::from("/app/bundled/codex")))
        );
        assert_eq!(
            envs.get(&OsString::from(home_key)),
            Some(&Some(home.as_os_str().to_os_string()))
        );
        assert_eq!(
            envs.get(&OsString::from(rust_log_key)),
            Some(&Some(OsString::from(DEFAULT_RUST_LOG)))
        );
        assert_eq!(
            env::var_os(home_key),
            Some(OsString::from("/tmp/ambient_home"))
        );
        assert!(home.is_dir());
        assert!(home.join("conversations").is_dir());
        assert!(home.join("logs").is_dir());

        match original_binary {
            Some(value) => env::set_var(binary_key, value),
            None => env::remove_var(binary_key),
        }
        match original_home {
            Some(value) => env::set_var(home_key, value),
            None => env::remove_var(home_key),
        }
        match original_rust_log {
            Some(value) => env::set_var(rust_log_key, value),
            None => env::remove_var(rust_log_key),
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
        let _guard = env_guard_async().await;
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
    fn codex_home_layout_exposes_paths() {
        let root = PathBuf::from("/tmp/codex_layout_root");
        let layout = CodexHomeLayout::new(&root);

        assert_eq!(layout.root(), root.as_path());
        assert_eq!(layout.config_path(), root.join("config.toml"));
        assert_eq!(layout.auth_path(), root.join("auth.json"));
        assert_eq!(layout.credentials_path(), root.join(".credentials.json"));
        assert_eq!(layout.history_path(), root.join("history.jsonl"));
        assert_eq!(layout.conversations_dir(), root.join("conversations"));
        assert_eq!(layout.logs_dir(), root.join("logs"));
    }

    #[test]
    fn codex_home_layout_respects_materialization_flag() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("codex_home_layout");
        let layout = CodexHomeLayout::new(&root);

        layout.materialize(false).unwrap();
        assert!(!root.exists());

        layout.materialize(true).unwrap();
        assert!(root.is_dir());
        assert!(layout.conversations_dir().is_dir());
        assert!(layout.logs_dir().is_dir());
    }

    #[test]
    fn seed_auth_copies_files_and_creates_targets() {
        let temp = tempfile::tempdir().unwrap();
        let seed = temp.path().join("seed_home");
        std::fs::create_dir_all(&seed).unwrap();
        std::fs::write(seed.join("auth.json"), "auth").unwrap();
        std::fs::write(seed.join(".credentials.json"), "creds").unwrap();

        let target_root = temp.path().join("target_home");
        let layout = CodexHomeLayout::new(&target_root);
        let outcome = layout
            .seed_auth_from(&seed, AuthSeedOptions::default())
            .unwrap();

        assert!(outcome.copied_auth);
        assert!(outcome.copied_credentials);
        assert_eq!(std::fs::read_to_string(layout.auth_path()).unwrap(), "auth");
        assert_eq!(
            std::fs::read_to_string(layout.credentials_path()).unwrap(),
            "creds"
        );
    }

    #[test]
    fn seed_auth_skips_optional_files() {
        let temp = tempfile::tempdir().unwrap();
        let seed = temp.path().join("seed_home");
        std::fs::create_dir_all(&seed).unwrap();
        std::fs::write(seed.join("auth.json"), "auth").unwrap();

        let target_root = temp.path().join("target_home");
        let layout = CodexHomeLayout::new(&target_root);
        let outcome = layout
            .seed_auth_from(&seed, AuthSeedOptions::default())
            .unwrap();

        assert!(outcome.copied_auth);
        assert!(!outcome.copied_credentials);
        assert_eq!(std::fs::read_to_string(layout.auth_path()).unwrap(), "auth");
        assert!(!layout.credentials_path().exists());
    }

    #[test]
    fn seed_auth_errors_when_required_missing() {
        let temp = tempfile::tempdir().unwrap();
        let seed = temp.path().join("seed_home");
        std::fs::create_dir_all(&seed).unwrap();

        let target_root = temp.path().join("target_home");
        let layout = CodexHomeLayout::new(&target_root);
        let err = layout
            .seed_auth_from(
                &seed,
                AuthSeedOptions {
                    require_auth: true,
                    require_credentials: true,
                    ..Default::default()
                },
            )
            .unwrap_err();

        match err {
            AuthSeedError::SeedFileMissing { path } => {
                assert!(path.ends_with("auth.json"), "{path:?}")
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn codex_client_returns_configured_home_layout() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("app_codex_home");
        let client = CodexClient::builder().codex_home(&root).build();

        let layout = client.codex_home_layout().expect("layout missing");
        assert_eq!(layout.root(), root.as_path());
        assert!(!root.exists());

        let client_without_home = CodexClient::builder().build();
        assert!(client_without_home.codex_home_layout().is_none());
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
    fn normalize_stream_infers_missing_thread_and_turn() {
        let mut context = StreamContext::default();
        // thread.started establishes thread context
        let thread_line = r#"{"type":"thread.started","thread_id":"thread-1"}"#;
        let thread_event = normalize_thread_event(thread_line, &mut context).unwrap();
        match thread_event {
            ThreadEvent::ThreadStarted(t) => assert_eq!(t.thread_id, "thread-1"),
            other => panic!("unexpected event: {other:?}"),
        }
        // turn.started without thread_id should inherit
        let turn_line = r#"{"type":"turn.started","turn_id":"turn-1"}"#;
        let turn_event = normalize_thread_event(turn_line, &mut context).unwrap();
        match turn_event {
            ThreadEvent::TurnStarted(t) => {
                assert_eq!(t.thread_id, "thread-1");
                assert_eq!(t.turn_id, "turn-1");
            }
            other => panic!("unexpected event: {other:?}"),
        }
        // item.completed without ids should inherit both
        let item_line =
            r#"{"type":"item.completed","item":{"id":"msg-1","type":"agent_message","text":"hi"}}"#;
        let item_event = normalize_thread_event(item_line, &mut context).unwrap();
        match item_event {
            ThreadEvent::ItemCompleted(item) => {
                assert_eq!(item.turn_id, "turn-1");
                assert_eq!(item.thread_id, "thread-1");
                assert_eq!(item.item.item_id, "msg-1");
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[test]
    fn normalize_stream_errors_without_context() {
        let mut context = StreamContext::default();
        let line = r#"{"type":"turn.started"}"#;
        let err = normalize_thread_event(line, &mut context).unwrap_err();
        match err {
            ExecStreamError::Normalize { .. } => {}
            other => panic!("unexpected error: {other:?}"),
        }
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
    fn update_advisory_handles_local_newer_than_known() {
        let capabilities = capabilities_with_version("codex 2.0.0");
        let latest = CodexLatestReleases {
            stable: Some(Version::parse("1.9.9").unwrap()),
            ..Default::default()
        };
        let advisory = update_advisory_from_capabilities(&capabilities, &latest);
        assert_eq!(advisory.status, CodexUpdateStatus::LocalNewerThanKnown);
        assert!(!advisory.is_update_recommended());
        assert!(advisory
            .notes
            .iter()
            .any(|note| note.contains("newer than provided")));
    }

    #[test]
    fn update_advisory_handles_missing_latest_metadata() {
        let capabilities = capabilities_with_version("codex 1.0.0");
        let latest = CodexLatestReleases::default();
        let advisory = update_advisory_from_capabilities(&capabilities, &latest);
        assert_eq!(advisory.status, CodexUpdateStatus::UnknownLatestVersion);
        assert!(!advisory.is_update_recommended());
        assert!(advisory
            .notes
            .iter()
            .any(|note| note.contains("advisory unavailable")));
    }

    #[test]
    fn capability_snapshots_serialize_to_json_and_toml() {
        let snapshot = sample_capabilities_snapshot();

        let json = serialize_capabilities_snapshot(&snapshot, CapabilitySnapshotFormat::Json)
            .expect("serialize json");
        let parsed_json = deserialize_capabilities_snapshot(&json, CapabilitySnapshotFormat::Json)
            .expect("parse json");
        assert_eq!(parsed_json, snapshot);

        let toml = serialize_capabilities_snapshot(&snapshot, CapabilitySnapshotFormat::Toml)
            .expect("serialize toml");
        let parsed_toml = deserialize_capabilities_snapshot(&toml, CapabilitySnapshotFormat::Toml)
            .expect("parse toml");
        assert_eq!(parsed_toml, snapshot);
    }

    #[test]
    fn capability_snapshots_and_overrides_round_trip_via_files() {
        let snapshot = sample_capabilities_snapshot();
        let overrides = sample_capability_overrides();
        let temp = tempfile::tempdir().unwrap();

        let snapshot_path = temp.path().join("capabilities.toml");
        write_capabilities_snapshot(&snapshot_path, &snapshot, None).unwrap();
        let loaded_snapshot = read_capabilities_snapshot(&snapshot_path, None).unwrap();
        assert_eq!(loaded_snapshot, snapshot);

        let overrides_path = temp.path().join("overrides.json");
        write_capability_overrides(&overrides_path, &overrides, None).unwrap();
        let loaded_overrides = read_capability_overrides(&overrides_path, None).unwrap();
        assert_eq!(loaded_overrides, overrides);
    }

    #[test]
    fn capability_snapshot_match_checks_fingerprint() {
        let temp = tempfile::tempdir().unwrap();
        let script = "#!/bin/bash\necho ok";
        let binary = write_fake_codex(temp.path(), script);
        let cache_key = capability_cache_key(&binary);
        let fingerprint = current_fingerprint(&cache_key);

        let snapshot = CodexCapabilities {
            cache_key: cache_key.clone(),
            fingerprint: fingerprint.clone(),
            version: None,
            features: CodexFeatureFlags::default(),
            probe_plan: CapabilityProbePlan::default(),
            collected_at: SystemTime::UNIX_EPOCH,
        };

        assert!(capability_snapshot_matches_binary(&snapshot, &binary));
        let mut missing_fingerprint = snapshot.clone();
        missing_fingerprint.fingerprint = None;
        assert!(!capability_snapshot_matches_binary(
            &missing_fingerprint,
            &binary
        ));

        std_fs::write(&binary, "#!/bin/bash\necho changed").unwrap();
        let mut perms = std_fs::metadata(&binary).unwrap().permissions();
        perms.set_mode(0o755);
        std_fs::set_permissions(&binary, perms).unwrap();

        assert!(!capability_snapshot_matches_binary(&snapshot, &binary));
    }

    #[test]
    fn capability_cache_entries_exposes_cache_state() {
        let _guard = env_guard();
        clear_capability_cache();

        let temp = tempfile::tempdir().unwrap();
        let binary = write_fake_codex(temp.path(), "#!/bin/bash\necho ok");
        let cache_key = capability_cache_key(&binary);
        let fingerprint = current_fingerprint(&cache_key);

        let snapshot = CodexCapabilities {
            cache_key: cache_key.clone(),
            fingerprint: fingerprint.clone(),
            version: Some(parse_version_output("codex 0.0.1")),
            features: CodexFeatureFlags {
                supports_features_list: true,
                supports_output_schema: true,
                supports_add_dir: false,
                supports_mcp_login: false,
            },
            probe_plan: CapabilityProbePlan {
                steps: vec![CapabilityProbeStep::VersionFlag],
            },
            collected_at: SystemTime::UNIX_EPOCH,
        };

        update_capability_cache(snapshot.clone());

        let entries = capability_cache_entries();
        assert!(entries.iter().any(|entry| entry.cache_key == cache_key));

        let fetched = capability_cache_entry(&binary).expect("expected cache entry");
        assert_eq!(fetched.cache_key, cache_key);
        assert!(clear_capability_cache_entry(&binary));
        assert!(capability_cache_entry(&binary).is_none());
        assert!(capability_cache_entries().is_empty());
        clear_capability_cache();
    }

    #[test]
    fn capability_ttl_decision_reuses_fresh_snapshot() {
        let collected_at = SystemTime::UNIX_EPOCH + Duration::from_secs(10);
        let snapshot = capability_snapshot_with_metadata(
            collected_at,
            Some(BinaryFingerprint {
                canonical_path: Some(PathBuf::from("/tmp/codex")),
                modified: Some(SystemTime::UNIX_EPOCH + Duration::from_secs(1)),
                len: Some(123),
            }),
        );

        let decision = capability_cache_ttl_decision(
            Some(&snapshot),
            Duration::from_secs(300),
            SystemTime::UNIX_EPOCH + Duration::from_secs(100),
        );
        assert!(!decision.should_probe);
        assert_eq!(decision.policy, CapabilityCachePolicy::PreferCache);
    }

    #[test]
    fn capability_ttl_decision_refreshes_after_ttl_with_fingerprint() {
        let collected_at = SystemTime::UNIX_EPOCH + Duration::from_secs(1);
        let snapshot = capability_snapshot_with_metadata(
            collected_at,
            Some(BinaryFingerprint {
                canonical_path: Some(PathBuf::from("/tmp/codex")),
                modified: Some(SystemTime::UNIX_EPOCH + Duration::from_secs(1)),
                len: Some(321),
            }),
        );

        let decision = capability_cache_ttl_decision(
            Some(&snapshot),
            Duration::from_secs(5),
            SystemTime::UNIX_EPOCH + Duration::from_secs(10),
        );
        assert!(decision.should_probe);
        assert_eq!(decision.policy, CapabilityCachePolicy::Refresh);
    }

    #[test]
    fn capability_ttl_decision_bypasses_when_metadata_missing() {
        let collected_at = SystemTime::UNIX_EPOCH + Duration::from_secs(2);
        let snapshot = capability_snapshot_with_metadata(collected_at, None);

        let decision = capability_cache_ttl_decision(
            Some(&snapshot),
            Duration::from_secs(5),
            SystemTime::UNIX_EPOCH + Duration::from_secs(10),
        );
        assert!(decision.should_probe);
        assert_eq!(decision.policy, CapabilityCachePolicy::Bypass);
    }

    #[tokio::test]
    async fn probe_reprobes_when_metadata_missing() {
        let _guard = env_guard_async().await;
        clear_capability_cache();

        let temp = tempfile::tempdir().unwrap();
        let binary = temp.path().join("missing_codex");
        let cache_key = capability_cache_key(&binary);

        {
            let mut cache = capability_cache().lock().unwrap();
            cache.insert(
                cache_key.clone(),
                CodexCapabilities {
                    cache_key: cache_key.clone(),
                    fingerprint: None,
                    version: Some(parse_version_output("codex 9.9.9")),
                    features: CodexFeatureFlags {
                        supports_features_list: true,
                        supports_output_schema: true,
                        supports_add_dir: true,
                        supports_mcp_login: true,
                    },
                    probe_plan: CapabilityProbePlan::default(),
                    collected_at: SystemTime::UNIX_EPOCH,
                },
            );
        }

        let client = CodexClient::builder()
            .binary(&binary)
            .timeout(Duration::from_secs(1))
            .build();

        let capabilities = client.probe_capabilities().await;
        assert!(!capabilities.features.supports_output_schema);
        assert!(capabilities
            .probe_plan
            .steps
            .contains(&CapabilityProbeStep::VersionFlag));

        clear_capability_cache();
    }

    #[tokio::test]
    async fn probe_refresh_policy_forces_new_snapshot() {
        let _guard = env_guard_async().await;
        clear_capability_cache();

        let temp = tempfile::tempdir().unwrap();
        let log_path = temp.path().join("probe.log");
        let script = format!(
            r#"#!/bin/bash
echo "$@" >> "{log}"
if [[ "$1" == "--version" ]]; then
  echo "codex 1.0.0"
elif [[ "$1" == "features" && "$2" == "list" && "$3" == "--json" ]]; then
  echo '{{"features":["output_schema"]}}'
elif [[ "$1" == "features" && "$2" == "list" ]]; then
  echo "output_schema"
fi
"#,
            log = log_path.display()
        );
        let binary = write_fake_codex(temp.path(), &script);
        let client = CodexClient::builder()
            .binary(&binary)
            .timeout(Duration::from_secs(5))
            .build();

        let first = client.probe_capabilities().await;
        assert!(first.features.supports_output_schema);
        let first_lines = std_fs::read_to_string(&log_path).unwrap().lines().count();
        assert!(first_lines >= 2);

        let refreshed = client
            .probe_capabilities_with_policy(CapabilityCachePolicy::Refresh)
            .await;
        assert!(refreshed.features.supports_output_schema);
        let refreshed_lines = std_fs::read_to_string(&log_path).unwrap().lines().count();
        assert!(
            refreshed_lines > first_lines,
            "expected refresh policy to re-run probes"
        );
        clear_capability_cache();
    }

    #[tokio::test]
    async fn probe_bypass_policy_skips_cache_writes() {
        let _guard = env_guard_async().await;
        clear_capability_cache();

        let temp = tempfile::tempdir().unwrap();
        let script = r#"#!/bin/bash
if [[ "$1" == "--version" ]]; then
  echo "codex 1.0.0"
elif [[ "$1" == "features" && "$2" == "list" && "$3" == "--json" ]]; then
  echo '{"features":["output_schema"]}'
elif [[ "$1" == "features" && "$2" == "list" ]]; then
  echo "output_schema"
fi
"#;
        let binary = write_fake_codex(temp.path(), script);

        let client = CodexClient::builder()
            .binary(&binary)
            .timeout(Duration::from_secs(5))
            .build();

        let capabilities = client
            .probe_capabilities_with_policy(CapabilityCachePolicy::Bypass)
            .await;
        assert!(capabilities.features.supports_output_schema);
        assert!(capability_cache_entry(&binary).is_none());
        clear_capability_cache();
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
    fn parses_feature_list_json_and_text_tables() {
        let json = r#"{"features":[{"name":"json-stream","stage":"stable","enabled":true,"notes":"keep"},{"name":"cloud-exec","stage":"experimental","enabled":false}]}"#;
        let (json_features, json_format) = parse_feature_list_output(json, true).unwrap();
        assert_eq!(json_format, FeaturesListFormat::Json);
        assert_eq!(json_features.len(), 2);
        assert_eq!(json_features[0].name, "json-stream");
        assert_eq!(json_features[0].stage, Some(CodexFeatureStage::Stable));
        assert!(json_features[0].enabled);
        assert!(json_features[0].extra.contains_key("notes"));
        assert_eq!(
            json_features[1].stage,
            Some(CodexFeatureStage::Experimental)
        );
        assert!(!json_features[1].enabled);

        let text = r#"
Feature   Stage         Enabled
json-stream stable      true
cloud-exec experimental false
"#;
        let (text_features, text_format) = parse_feature_list_output(text, false).unwrap();
        assert_eq!(text_format, FeaturesListFormat::Text);
        assert_eq!(text_features.len(), 2);
        assert_eq!(
            text_features[1].stage,
            Some(CodexFeatureStage::Experimental)
        );
        assert!(!text_features[1].enabled);

        let (fallback_features, fallback_format) = parse_feature_list_output(text, true).unwrap();
        assert_eq!(fallback_format, FeaturesListFormat::Text);
        assert_eq!(fallback_features.len(), 2);
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

    #[test]
    fn capability_guard_reports_detected_support() {
        let flags = CodexFeatureFlags {
            supports_features_list: true,
            supports_output_schema: true,
            supports_add_dir: true,
            supports_mcp_login: true,
        };
        let capabilities = capabilities_with_feature_flags(flags);

        let output_schema = capabilities.guard_output_schema();
        assert_eq!(output_schema.support, CapabilitySupport::Supported);
        assert!(output_schema.is_supported());

        let add_dir = capabilities.guard_add_dir();
        assert_eq!(add_dir.support, CapabilitySupport::Supported);
        assert!(add_dir.is_supported());

        let mcp_login = capabilities.guard_mcp_login();
        assert_eq!(mcp_login.support, CapabilitySupport::Supported);

        let features_list = capabilities.guard_features_list();
        assert_eq!(features_list.support, CapabilitySupport::Supported);
    }

    #[test]
    fn capability_guard_marks_absent_feature_as_unsupported() {
        let flags = CodexFeatureFlags {
            supports_features_list: true,
            supports_output_schema: false,
            supports_add_dir: false,
            supports_mcp_login: false,
        };
        let capabilities = capabilities_with_feature_flags(flags);

        let output_schema = capabilities.guard_output_schema();
        assert_eq!(output_schema.support, CapabilitySupport::Unsupported);
        assert!(!output_schema.is_supported());
        assert!(output_schema
            .notes
            .iter()
            .any(|note| note.contains("features list")));

        let mcp_login = capabilities.guard_mcp_login();
        assert_eq!(mcp_login.support, CapabilitySupport::Unsupported);
    }

    #[test]
    fn capability_guard_returns_unknown_without_feature_list() {
        let capabilities = capabilities_with_feature_flags(CodexFeatureFlags::default());

        let add_dir = capabilities.guard_add_dir();
        assert_eq!(add_dir.support, CapabilitySupport::Unknown);
        assert!(add_dir.is_unknown());
        assert!(add_dir
            .notes
            .iter()
            .any(|note| note.contains("unknown") || note.contains("unavailable")));

        let features_list = capabilities.guard_features_list();
        assert_eq!(features_list.support, CapabilitySupport::Unknown);
    }

    #[tokio::test]
    async fn capability_snapshot_short_circuits_probes() {
        let _guard = env_guard_async().await;
        clear_capability_cache();

        let temp = tempfile::tempdir().unwrap();
        let log_path = temp.path().join("probe.log");
        let script = format!(
            r#"#!/bin/bash
echo "$@" >> "{log}"
exit 99
"#,
            log = log_path.display()
        );
        let binary = write_fake_codex(temp.path(), &script);

        let snapshot = CodexCapabilities {
            cache_key: CapabilityCacheKey {
                binary_path: PathBuf::from("codex"),
            },
            fingerprint: None,
            version: Some(parse_version_output("codex 9.9.9-custom")),
            features: CodexFeatureFlags {
                supports_features_list: true,
                supports_output_schema: true,
                supports_add_dir: false,
                supports_mcp_login: true,
            },
            probe_plan: CapabilityProbePlan::default(),
            collected_at: SystemTime::now(),
        };

        let client = CodexClient::builder()
            .binary(&binary)
            .capability_snapshot(snapshot)
            .timeout(Duration::from_secs(5))
            .build();

        let capabilities = client.probe_capabilities().await;
        assert_eq!(
            capabilities.cache_key.binary_path,
            std_fs::canonicalize(&binary).unwrap()
        );
        assert!(capabilities.fingerprint.is_some());
        assert!(capabilities.features.supports_output_schema);
        assert!(capabilities.features.supports_mcp_login);
        assert_eq!(
            capabilities.version.as_ref().and_then(|v| v.semantic),
            Some((9, 9, 9))
        );
        assert!(capabilities
            .probe_plan
            .steps
            .contains(&CapabilityProbeStep::ManualOverride));
        assert!(!log_path.exists());
    }

    #[tokio::test]
    async fn capability_feature_overrides_apply_to_cached_entries() {
        let _guard = env_guard_async().await;
        clear_capability_cache();

        let temp = tempfile::tempdir().unwrap();
        let script = r#"#!/bin/bash
if [[ "$1" == "--version" ]]; then
  echo "codex 1.0.0"
elif [[ "$1" == "features" && "$2" == "list" && "$3" == "--json" ]]; then
  echo '{"features":[]}'
elif [[ "$1" == "features" && "$2" == "list" ]]; then
  echo "features list"
elif [[ "$1" == "--help" ]]; then
  echo "Usage: codex exec"
fi
"#;
        let binary = write_fake_codex(temp.path(), script);

        let base_client = CodexClient::builder()
            .binary(&binary)
            .timeout(Duration::from_secs(5))
            .build();
        let base_capabilities = base_client.probe_capabilities().await;
        assert!(base_capabilities.features.supports_features_list);
        assert!(!base_capabilities.features.supports_output_schema);

        let overrides = CapabilityFeatureOverrides::enabling(CodexFeatureFlags {
            supports_features_list: false,
            supports_output_schema: true,
            supports_add_dir: false,
            supports_mcp_login: true,
        });

        let client = CodexClient::builder()
            .binary(&binary)
            .capability_feature_overrides(overrides)
            .timeout(Duration::from_secs(5))
            .build();

        let capabilities = client.probe_capabilities().await;
        assert!(capabilities.features.supports_output_schema);
        assert!(capabilities.features.supports_mcp_login);
        assert!(capabilities
            .probe_plan
            .steps
            .contains(&CapabilityProbeStep::ManualOverride));
        assert_eq!(
            capabilities.guard_output_schema().support,
            CapabilitySupport::Supported
        );
    }

    #[tokio::test]
    async fn capability_version_override_replaces_probe_version() {
        let _guard = env_guard_async().await;
        clear_capability_cache();

        let temp = tempfile::tempdir().unwrap();
        let script = r#"#!/bin/bash
if [[ "$1" == "--version" ]]; then
  echo "codex 0.1.0"
elif [[ "$1" == "features" && "$2" == "list" && "$3" == "--json" ]]; then
  echo '{"features":["add_dir"]}'
elif [[ "$1" == "features" && "$2" == "list" ]]; then
  echo "add_dir"
elif [[ "$1" == "--help" ]]; then
  echo "Usage: codex add-dir"
fi
"#;
        let binary = write_fake_codex(temp.path(), script);
        let version_override = parse_version_output("codex 9.9.9-nightly (commit beefcafe)");

        let client = CodexClient::builder()
            .binary(&binary)
            .timeout(Duration::from_secs(5))
            .capability_version_override(version_override)
            .build();

        let capabilities = client.probe_capabilities().await;
        assert_eq!(
            capabilities.version.as_ref().and_then(|v| v.semantic),
            Some((9, 9, 9))
        );
        assert!(matches!(
            capabilities.version.as_ref().map(|v| v.channel),
            Some(CodexReleaseChannel::Nightly)
        ));
        assert!(capabilities.features.supports_add_dir);
        assert!(capabilities
            .probe_plan
            .steps
            .contains(&CapabilityProbeStep::ManualOverride));
        assert_eq!(
            capabilities.guard_add_dir().support,
            CapabilitySupport::Supported
        );
    }

    #[tokio::test]
    async fn exec_applies_guarded_flags_when_supported() {
        let _guard = env_guard_async().await;
        clear_capability_cache();

        let temp = tempfile::tempdir().unwrap();
        let log_path = temp.path().join("exec.log");
        let script = format!(
            r#"#!/bin/bash
log="{log}"
if [[ "$1" == "--version" ]]; then
  echo "codex 1.2.3"
elif [[ "$1" == "features" && "$2" == "list" && "$3" == "--json" ]]; then
  echo '{{"features":["output_schema","add_dir","mcp_login"]}}'
elif [[ "$1" == "features" && "$2" == "list" ]]; then
  echo "output_schema add_dir login --mcp"
elif [[ "$1" == "--help" ]]; then
  echo "Usage: codex --output-schema add-dir login --mcp"
elif [[ "$1" == "exec" ]]; then
  echo "$@" >> "$log"
  echo "ok"
fi
"#,
            log = log_path.display()
        );
        let binary = write_fake_codex(temp.path(), &script);
        let client = CodexClient::builder()
            .binary(&binary)
            .timeout(Duration::from_secs(5))
            .add_dir("src")
            .output_schema(true)
            .quiet(true)
            .mirror_stdout(false)
            .build();

        let response = client.send_prompt("hello").await.unwrap();
        assert_eq!(response.trim(), "ok");

        let logged = std_fs::read_to_string(&log_path).unwrap();
        assert!(logged.contains("--add-dir"));
        assert!(logged.contains("src"));
        assert!(logged.contains("--output-schema"));
    }

    #[tokio::test]
    async fn exec_skips_guarded_flags_when_unknown() {
        let _guard = env_guard_async().await;
        clear_capability_cache();

        let temp = tempfile::tempdir().unwrap();
        let log_path = temp.path().join("exec.log");
        let script = format!(
            r#"#!/bin/bash
log="{log}"
if [[ "$1" == "--version" ]]; then
  echo "codex 0.9.0"
elif [[ "$1" == "features" && "$2" == "list" && "$3" == "--json" ]]; then
  echo "feature list unavailable" >&2
  exit 1
elif [[ "$1" == "features" && "$2" == "list" ]]; then
  echo "feature list unavailable" >&2
  exit 1
elif [[ "$1" == "--help" ]]; then
  echo "Usage: codex exec"
elif [[ "$1" == "exec" ]]; then
  echo "$@" >> "$log"
  echo "ok"
fi
"#,
            log = log_path.display()
        );
        let binary = write_fake_codex(temp.path(), &script);
        let client = CodexClient::builder()
            .binary(&binary)
            .timeout(Duration::from_secs(5))
            .add_dir("src")
            .output_schema(true)
            .quiet(true)
            .mirror_stdout(false)
            .build();

        let response = client.send_prompt("hello").await.unwrap();
        assert_eq!(response.trim(), "ok");

        let logged = std_fs::read_to_string(&log_path).unwrap();
        assert!(!logged.contains("--add-dir"));
        assert!(!logged.contains("--output-schema"));
    }

    #[tokio::test]
    async fn mcp_login_skips_when_unsupported() {
        let _guard = env_guard_async().await;
        clear_capability_cache();

        let temp = tempfile::tempdir().unwrap();
        let log_path = temp.path().join("login.log");
        let script = format!(
            r#"#!/bin/bash
log="{log}"
if [[ "$1" == "--version" ]]; then
  echo "codex 1.0.0"
elif [[ "$1" == "features" && "$2" == "list" && "$3" == "--json" ]]; then
  echo '{{"features":["output_schema","add_dir"]}}'
elif [[ "$1" == "features" && "$2" == "list" ]]; then
  echo "output_schema add-dir"
elif [[ "$1" == "--help" ]]; then
  echo "Usage: codex exec"
elif [[ "$1" == "login" ]]; then
  echo "$@" >> "$log"
  echo "login invoked"
fi
"#,
            log = log_path.display()
        );
        let binary = write_fake_codex(temp.path(), &script);
        let client = CodexClient::builder()
            .binary(&binary)
            .timeout(Duration::from_secs(5))
            .build();

        let login = client.spawn_mcp_login_process().await.unwrap();
        assert!(login.is_none());
        assert!(!log_path.exists());
    }

    #[tokio::test]
    async fn mcp_login_runs_when_supported() {
        let _guard = env_guard_async().await;
        clear_capability_cache();

        let temp = tempfile::tempdir().unwrap();
        let log_path = temp.path().join("login.log");
        let script = format!(
            r#"#!/bin/bash
log="{log}"
if [[ "$1" == "--version" ]]; then
  echo "codex 2.0.0"
elif [[ "$1" == "features" && "$2" == "list" && "$3" == "--json" ]]; then
  echo '{{"features":["output_schema","add_dir"],"mcp_login":true}}'
elif [[ "$1" == "features" && "$2" == "list" ]]; then
  echo "output_schema add_dir login --mcp"
elif [[ "$1" == "--help" ]]; then
  echo "Usage: codex --output-schema add-dir login --mcp"
elif [[ "$1" == "login" ]]; then
  echo "$@" >> "$log"
  echo "login invoked"
fi
"#,
            log = log_path.display()
        );
        let binary = write_fake_codex(temp.path(), &script);
        let client = CodexClient::builder()
            .binary(&binary)
            .timeout(Duration::from_secs(5))
            .build();

        let login = client
            .spawn_mcp_login_process()
            .await
            .unwrap()
            .expect("expected login child");
        let output = login.wait_with_output().await.unwrap();
        assert!(output.status.success());

        let logged = std_fs::read_to_string(&log_path).unwrap();
        assert!(logged.contains("login --mcp"));
    }

    #[tokio::test]
    async fn probe_capabilities_caches_and_invalidates() {
        let _guard = env_guard_async().await;
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
        std_fs::write(&binary, script_v2).unwrap();
        let mut perms = std_fs::metadata(&binary).unwrap().permissions();
        perms.set_mode(0o755);
        std_fs::set_permissions(&binary, perms).unwrap();

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
            reasoning_config_for(Some("gpt-5.1-codex-max")).unwrap(),
            DEFAULT_REASONING_CONFIG_GPT5_1
        );
        assert_eq!(
            reasoning_config_for(Some("gpt-5-codex")).unwrap(),
            DEFAULT_REASONING_CONFIG_GPT5_CODEX
        );
        assert!(reasoning_config_for(None).is_none());
        assert!(reasoning_config_for(Some("gpt-4.1-mini")).is_none());
    }

    #[test]
    fn resolve_cli_overrides_respects_reasoning_defaults() {
        let builder = CliOverrides::default();
        let patch = CliOverridesPatch::default();

        let resolved = resolve_cli_overrides(&builder, &patch, Some("gpt-5"));
        let keys: Vec<_> = resolved
            .config_overrides
            .iter()
            .map(|override_| override_.key.as_str())
            .collect();
        assert!(keys.contains(&"model_reasoning_effort"));
        assert!(keys.contains(&"model_reasoning_summary"));
        assert!(keys.contains(&"model_verbosity"));

        let resolved_without_model = resolve_cli_overrides(&builder, &patch, None);
        assert!(resolved_without_model.config_overrides.is_empty());
    }

    #[test]
    fn explicit_reasoning_overrides_disable_defaults() {
        let mut builder = CliOverrides::default();
        builder
            .config_overrides
            .push(ConfigOverride::new("model_reasoning_effort", "high"));

        let resolved =
            resolve_cli_overrides(&builder, &CliOverridesPatch::default(), Some("gpt-5"));
        assert_eq!(resolved.config_overrides.len(), 1);
        assert_eq!(resolved.config_overrides[0].value, "high");
    }

    #[test]
    fn request_can_disable_auto_reasoning_defaults() {
        let builder = CliOverrides::default();
        let patch = CliOverridesPatch {
            auto_reasoning_defaults: Some(false),
            ..Default::default()
        };

        let resolved = resolve_cli_overrides(&builder, &patch, Some("gpt-5"));
        assert!(resolved.config_overrides.is_empty());
    }

    #[test]
    fn request_config_overrides_follow_builder_order() {
        let mut builder_overrides = CliOverrides {
            auto_reasoning_defaults: false,
            ..Default::default()
        };
        builder_overrides
            .config_overrides
            .push(ConfigOverride::new("foo", "bar"));

        let mut patch = CliOverridesPatch::default();
        patch
            .config_overrides
            .push(ConfigOverride::new("foo", "baz"));

        let resolved = resolve_cli_overrides(&builder_overrides, &patch, None);
        let values: Vec<_> = resolved
            .config_overrides
            .iter()
            .map(|override_| override_.value.as_str())
            .collect();
        assert_eq!(values, vec!["bar", "baz"]);
    }

    #[test]
    fn request_search_override_can_disable_builder_flag() {
        let builder_overrides = CliOverrides {
            search: FlagState::Enable,
            ..Default::default()
        };

        let patch = CliOverridesPatch {
            search: FlagState::Disable,
            ..Default::default()
        };

        let resolved = resolve_cli_overrides(&builder_overrides, &patch, None);
        let args = cli_override_args(&resolved, true);
        let args: Vec<_> = args
            .iter()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        assert!(!args.contains(&"--search".to_string()));
    }

    #[test]
    fn request_profile_override_replaces_builder_value() {
        let builder_overrides = CliOverrides {
            profile: Some("builder".to_string()),
            ..Default::default()
        };

        let patch = CliOverridesPatch {
            profile: Some("request".to_string()),
            ..Default::default()
        };

        let resolved = resolve_cli_overrides(&builder_overrides, &patch, None);
        let args: Vec<_> = cli_override_args(&resolved, true)
            .iter()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        assert!(args.windows(2).any(|window| {
            window.first().map(String::as_str) == Some("--profile")
                && window.get(1).map(String::as_str) == Some("request")
        }));
        assert!(!args.contains(&"builder".to_string()));
    }

    #[test]
    fn request_oss_override_can_disable_builder_flag() {
        let builder_overrides = CliOverrides {
            oss: FlagState::Enable,
            ..Default::default()
        };

        let resolved =
            resolve_cli_overrides(&builder_overrides, &CliOverridesPatch::default(), None);
        let args: Vec<_> = cli_override_args(&resolved, true)
            .iter()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        assert!(args.contains(&"--oss".to_string()));

        let patch = CliOverridesPatch {
            oss: FlagState::Disable,
            ..Default::default()
        };
        let resolved = resolve_cli_overrides(&builder_overrides, &patch, None);
        let args: Vec<_> = cli_override_args(&resolved, true)
            .iter()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        assert!(!args.contains(&"--oss".to_string()));
    }

    #[test]
    fn feature_toggles_merge_builder_and_request() {
        let mut builder_overrides = CliOverrides::default();
        builder_overrides
            .feature_toggles
            .enable
            .push("builder-enable".to_string());
        builder_overrides
            .feature_toggles
            .disable
            .push("builder-disable".to_string());

        let mut patch = CliOverridesPatch::default();
        patch
            .feature_toggles
            .enable
            .push("request-enable".to_string());
        patch
            .feature_toggles
            .disable
            .push("request-disable".to_string());

        let resolved = resolve_cli_overrides(&builder_overrides, &patch, None);
        let args: Vec<_> = cli_override_args(&resolved, true)
            .iter()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();

        assert!(args.windows(2).any(|window| {
            window.first().map(String::as_str) == Some("--enable")
                && window.get(1).map(String::as_str) == Some("builder-enable")
        }));
        assert!(args.windows(2).any(|window| {
            window.first().map(String::as_str) == Some("--enable")
                && window.get(1).map(String::as_str) == Some("request-enable")
        }));
        assert!(args.windows(2).any(|window| {
            window.first().map(String::as_str) == Some("--disable")
                && window.get(1).map(String::as_str) == Some("builder-disable")
        }));
        assert!(args.windows(2).any(|window| {
            window.first().map(String::as_str) == Some("--disable")
                && window.get(1).map(String::as_str) == Some("request-disable")
        }));
    }

    #[test]
    fn cli_override_args_apply_safety_precedence() {
        let mut resolved = ResolvedCliOverrides {
            config_overrides: Vec::new(),
            feature_toggles: FeatureToggles::default(),
            approval_policy: None,
            sandbox_mode: None,
            safety_override: SafetyOverride::FullAuto,
            profile: None,
            cd: None,
            local_provider: None,
            oss: false,
            search: FlagState::Enable,
        };
        let args = cli_override_args(&resolved, true);
        let args: Vec<_> = args
            .iter()
            .map(|value| value.to_string_lossy().into_owned())
            .collect();
        assert!(args.contains(&"--full-auto".to_string()));
        assert!(args.contains(&"--search".to_string()));
        assert!(!args.contains(&"--ask-for-approval".to_string()));

        resolved.approval_policy = Some(ApprovalPolicy::OnRequest);
        let args_with_policy = cli_override_args(&resolved, true);
        let args_with_policy: Vec<_> = args_with_policy
            .iter()
            .map(|value| value.to_string_lossy().into_owned())
            .collect();
        assert!(!args_with_policy.contains(&"--full-auto".to_string()));
        assert!(args_with_policy.contains(&"--ask-for-approval".to_string()));

        let resolved = ResolvedCliOverrides {
            config_overrides: vec![ConfigOverride::new("foo", "bar")],
            feature_toggles: FeatureToggles::default(),
            approval_policy: Some(ApprovalPolicy::OnRequest),
            sandbox_mode: Some(SandboxMode::WorkspaceWrite),
            safety_override: SafetyOverride::DangerouslyBypass,
            profile: Some("team".to_string()),
            cd: Some(PathBuf::from("/tmp/worktree")),
            local_provider: Some(LocalProvider::Ollama),
            oss: false,
            search: FlagState::Enable,
        };
        let args = cli_override_args(&resolved, true);
        let args: Vec<_> = args
            .iter()
            .map(|value| value.to_string_lossy().into_owned())
            .collect();
        assert!(args.contains(&"--config".to_string()));
        assert!(args.contains(&"foo=bar".to_string()));
        assert!(args.contains(&"--dangerously-bypass-approvals-and-sandbox".to_string()));
        assert!(args.contains(&"--profile".to_string()));
        assert!(args.contains(&"team".to_string()));
        assert!(args.contains(&"--cd".to_string()));
        assert!(args.contains(&"/tmp/worktree".to_string()));
        assert!(args.contains(&"--local-provider".to_string()));
        assert!(args.contains(&"ollama".to_string()));
        assert!(args.contains(&"--search".to_string()));
        assert!(!args.contains(&"--ask-for-approval".to_string()));
        assert!(!args.contains(&"--sandbox".to_string()));

        let args_without_search = cli_override_args(&resolved, false);
        let args_without_search: Vec<_> = args_without_search
            .iter()
            .map(|value| value.to_string_lossy().into_owned())
            .collect();
        assert!(!args_without_search.contains(&"--search".to_string()));
    }

    #[tokio::test]
    async fn exec_applies_cli_overrides_and_request_patch() {
        let _guard = env_guard_async().await;
        clear_capability_cache();

        let temp = tempfile::tempdir().unwrap();
        let log_path = temp.path().join("exec.log");
        let builder_cd = temp.path().join("builder-cd");
        let request_cd = temp.path().join("request-cd");
        let script = format!(
            r#"#!/bin/bash
echo "$@" >> "{log}"
if [[ "$1" == "exec" ]]; then
  echo "ok"
fi
"#,
            log = log_path.display()
        );
        let binary = write_fake_codex(temp.path(), &script);
        let client = CodexClient::builder()
            .binary(&binary)
            .timeout(Duration::from_secs(5))
            .mirror_stdout(false)
            .quiet(true)
            .auto_reasoning_defaults(false)
            .config_override("foo", "bar")
            .reasoning_summary(ReasoningSummary::Concise)
            .approval_policy(ApprovalPolicy::OnRequest)
            .sandbox_mode(SandboxMode::WorkspaceWrite)
            .cd(&builder_cd)
            .local_provider(LocalProvider::Custom)
            .oss(true)
            .enable_feature("builder-on")
            .disable_feature("builder-off")
            .search(true)
            .build();

        let mut request = ExecRequest::new("list flags")
            .config_override("extra", "value")
            .oss(false)
            .enable_feature("request-on")
            .disable_feature("request-off")
            .search(false);
        request.overrides.cd = Some(request_cd.clone());
        request.overrides.safety_override = Some(SafetyOverride::DangerouslyBypass);

        let response = client.send_prompt_with(request).await.unwrap();
        assert_eq!(response.trim(), "ok");

        let logged = std_fs::read_to_string(&log_path).unwrap();
        assert!(logged.contains("--config"));
        assert!(logged.contains("foo=bar"));
        assert!(logged.contains("extra=value"));
        assert!(logged.contains("model_reasoning_summary=concise"));
        assert!(logged.contains("--dangerously-bypass-approvals-and-sandbox"));
        assert!(logged.contains(&request_cd.display().to_string()));
        assert!(!logged.contains(&builder_cd.display().to_string()));
        assert!(logged.contains("--local-provider"));
        assert!(logged.contains("custom"));
        assert!(logged.contains("--enable"));
        assert!(logged.contains("builder-on"));
        assert!(logged.contains("request-on"));
        assert!(logged.contains("--disable"));
        assert!(logged.contains("builder-off"));
        assert!(logged.contains("request-off"));
        assert!(!logged.contains("--oss"));
        assert!(!logged.contains("--ask-for-approval"));
        assert!(!logged.contains("--sandbox"));
        assert!(!logged.contains("--search"));
    }

    #[tokio::test]
    async fn resume_applies_search_and_selector_overrides() {
        let _guard = env_guard_async().await;
        clear_capability_cache();

        let temp = tempfile::tempdir().unwrap();
        let log_path = temp.path().join("resume.log");
        let builder_cd = temp.path().join("builder-cd");
        let request_cd = temp.path().join("request-cd");
        let script = format!(
            r#"#!/bin/bash
echo "$@" >> "{log}"
if [[ "$1" == "resume" ]]; then
  echo '{{"type":"thread.started","thread_id":"thread-1"}}'
  echo '{{"type":"turn.started","thread_id":"thread-1","turn_id":"turn-1"}}'
  echo '{{"type":"turn.completed","thread_id":"thread-1","turn_id":"turn-1"}}'
fi
"#,
            log = log_path.display()
        );
        let binary = write_fake_codex(temp.path(), &script);
        let client = CodexClient::builder()
            .binary(&binary)
            .timeout(Duration::from_secs(5))
            .mirror_stdout(false)
            .quiet(true)
            .config_override("resume_hint", "enabled")
            .approval_policy(ApprovalPolicy::OnRequest)
            .sandbox_mode(SandboxMode::WorkspaceWrite)
            .local_provider(LocalProvider::Ollama)
            .cd(&builder_cd)
            .search(true)
            .build();

        let request_last = ResumeRequest::last().prompt("continue");
        let stream = client.stream_resume(request_last).await.unwrap();
        let events: Vec<_> = stream.events.collect().await;
        assert_eq!(events.len(), 3);
        stream.completion.await.unwrap();

        let mut request_all = ResumeRequest::all().prompt("summarize");
        request_all.overrides.search = FlagState::Disable;
        request_all.overrides.safety_override = Some(SafetyOverride::DangerouslyBypass);
        request_all.overrides.cd = Some(request_cd.clone());
        let stream_all = client.stream_resume(request_all).await.unwrap();
        let _ = stream_all.events.collect::<Vec<_>>().await;
        stream_all.completion.await.unwrap();

        let logged: Vec<_> = std_fs::read_to_string(&log_path)
            .unwrap()
            .lines()
            .map(str::to_string)
            .collect();
        assert!(logged.len() >= 2);

        assert!(logged[0].contains("--last"));
        assert!(logged[0].contains("--search"));
        assert!(logged[0].contains("resume_hint=enabled"));
        assert!(logged[0].contains("--ask-for-approval"));
        assert!(logged[0].contains("--sandbox"));
        assert!(logged[0].contains(&builder_cd.display().to_string()));
        assert!(logged[0].contains("ollama"));

        assert!(logged[1].contains("--all"));
        assert!(logged[1].contains("--dangerously-bypass-approvals-and-sandbox"));
        assert!(logged[1].contains(&request_cd.display().to_string()));
        assert!(!logged[1].contains(&builder_cd.display().to_string()));
        assert!(!logged[1].contains("--ask-for-approval"));
        assert!(!logged[1].contains("--sandbox"));
        assert!(!logged[1].contains("--search"));
    }

    #[tokio::test]
    async fn apply_respects_cli_overrides_without_search() {
        let _guard = env_guard_async().await;
        clear_capability_cache();

        let temp = tempfile::tempdir().unwrap();
        let log_path = temp.path().join("apply.log");
        let script = format!(
            r#"#!/bin/bash
echo "$@" >> "{log}"
if [[ "$1" == "apply" ]]; then
  echo "applied"
fi
"#,
            log = log_path.display()
        );
        let binary = write_fake_codex(temp.path(), &script);
        let client = CodexClient::builder()
            .binary(&binary)
            .timeout(Duration::from_secs(5))
            .mirror_stdout(false)
            .quiet(true)
            .cd(temp.path().join("apply-cd"))
            .config_override("feature.toggle", "true")
            .search(true)
            .build();

        let artifacts = client.apply().await.unwrap();
        assert_eq!(artifacts.stdout.trim(), "applied");

        let logged = std_fs::read_to_string(&log_path).unwrap();
        assert!(logged.contains("--config"));
        assert!(logged.contains("feature.toggle=true"));
        assert!(logged.contains("apply-cd"));
        assert!(!logged.contains("--search"));
    }

    #[test]
    fn color_mode_strings_are_stable() {
        assert_eq!(ColorMode::Auto.as_str(), "auto");
        assert_eq!(ColorMode::Always.as_str(), "always");
        assert_eq!(ColorMode::Never.as_str(), "never");
    }

    #[tokio::test]
    async fn auth_helper_uses_app_scoped_home_without_mutating_env() {
        let _guard = env_guard_async().await;
        let temp = tempfile::tempdir().unwrap();
        let log_path = temp.path().join("auth.log");
        let app_home = temp.path().join("app-home");
        let caller_home = temp.path().join("caller-home");
        let previous_home = env::var("CODEX_HOME").ok();
        env::set_var("CODEX_HOME", &caller_home);
        env::set_var("AUTH_HELPER_LOG", &log_path);

        let script = r#"#!/usr/bin/env bash
set -e
echo "args:$*" >> "$AUTH_HELPER_LOG"
echo "CODEX_HOME=${CODEX_HOME:-missing}" >> "$AUTH_HELPER_LOG"
if [[ "$1" == "login" && "$2" == "status" ]]; then
  echo "Logged in using ChatGPT"
  exit 0
fi
echo "Not logged in" >&2
exit 1
"#;
        let binary = write_fake_codex(temp.path(), script);
        let helper = AuthSessionHelper::with_client(
            CodexClient::builder()
                .binary(&binary)
                .codex_home(&app_home)
                .build(),
        );

        let status = helper.status().await.unwrap();
        assert!(matches!(
            status,
            CodexAuthStatus::LoggedIn(CodexAuthMethod::ChatGpt)
        ));

        let logged = std_fs::read_to_string(&log_path).unwrap();
        assert!(logged.contains("args:login status"));
        assert!(logged.contains(&format!("CODEX_HOME={}", app_home.display())));

        assert_eq!(
            env::var("CODEX_HOME").unwrap(),
            caller_home.display().to_string()
        );

        env::remove_var("AUTH_HELPER_LOG");
        if let Some(previous) = previous_home {
            env::set_var("CODEX_HOME", previous);
        } else {
            env::remove_var("CODEX_HOME");
        }
    }

    #[tokio::test]
    async fn ensure_api_key_login_runs_when_logged_out() {
        let _guard = env_guard_async().await;
        let temp = tempfile::tempdir().unwrap();
        let log_path = temp.path().join("login.log");
        let state_path = temp.path().join("api-key-state");
        let script = format!(
            r#"#!/usr/bin/env bash
set -e
echo "$@" >> "{log}"
if [[ "$1" == "login" && "$2" == "status" ]]; then
  if [[ -f "{state}" ]]; then
    echo "Logged in using an API key - sk-already"
    exit 0
  fi
  echo "Not logged in" >&2
  exit 1
fi
if [[ "$1" == "login" && "$2" == "--api-key" ]]; then
  echo "Logged in using an API key - $3" > "{state}"
  echo "Logged in using an API key - $3"
  exit 0
fi
echo "unexpected args: $*" >&2
exit 2
"#,
            log = log_path.display(),
            state = state_path.display()
        );
        let binary = write_fake_codex(temp.path(), &script);
        let helper = AuthSessionHelper::with_client(
            CodexClient::builder()
                .binary(&binary)
                .codex_home(temp.path().join("app-home"))
                .build(),
        );

        let status = helper.ensure_api_key_login("sk-test-key").await.unwrap();
        match status {
            CodexAuthStatus::LoggedIn(CodexAuthMethod::ApiKey { masked_key }) => {
                assert_eq!(masked_key.as_deref(), Some("sk-test-key"));
            }
            other => panic!("unexpected status: {other:?}"),
        }

        let second = helper.ensure_api_key_login("sk-other").await.unwrap();
        assert!(matches!(
            second,
            CodexAuthStatus::LoggedIn(CodexAuthMethod::ApiKey { .. })
        ));

        let log = std_fs::read_to_string(&log_path).unwrap();
        assert!(log.contains("login status"));
        assert!(log.contains("login --api-key sk-test-key"));
        assert_eq!(
            log.lines()
                .filter(|line| line.contains("--api-key"))
                .count(),
            1
        );
    }

    #[tokio::test]
    async fn ensure_chatgpt_login_launches_when_needed() {
        let _guard = env_guard_async().await;
        let temp = tempfile::tempdir().unwrap();
        let log_path = temp.path().join("chatgpt.log");
        let state_path = temp.path().join("chatgpt-state");
        let script = format!(
            r#"#!/usr/bin/env bash
set -e
echo "$@" >> "{log}"
if [[ "$1" == "login" && "$2" == "status" ]]; then
  if [[ -f "{state}" ]]; then
    echo "Logged in using ChatGPT"
    exit 0
  fi
  echo "Not logged in" >&2
  exit 1
fi
if [[ "$1" == "login" && -z "$2" ]]; then
  echo "Logged in using ChatGPT" > "{state}"
  echo "Logged in using ChatGPT"
  exit 0
fi
echo "unknown args: $*" >&2
exit 2
"#,
            log = log_path.display(),
            state = state_path.display()
        );
        let binary = write_fake_codex(temp.path(), &script);
        let helper = AuthSessionHelper::with_client(
            CodexClient::builder()
                .binary(&binary)
                .codex_home(temp.path().join("app-home"))
                .build(),
        );

        let child = helper.ensure_chatgpt_login().await.unwrap();
        let child = child.expect("expected ChatGPT login child");
        let output = child.wait_with_output().await.unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("Logged in using ChatGPT"));

        let second = helper.ensure_chatgpt_login().await.unwrap();
        assert!(second.is_none());

        let log = std_fs::read_to_string(&log_path).unwrap();
        assert!(log.lines().any(|line| line == "login"));
        assert_eq!(log.lines().filter(|line| line == &"login").count(), 1);
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

    #[test]
    fn parse_login_accepts_unknown_on_success() {
        let message = "Authenticated";
        assert!(parse_login_success(message).is_none());
        let status = CodexAuthStatus::LoggedIn(CodexAuthMethod::Unknown {
            raw: message.to_string(),
        });
        assert!(matches!(
            status,
            CodexAuthStatus::LoggedIn(CodexAuthMethod::Unknown { .. })
        ));
    }
}

fn default_rust_log_value() -> Option<&'static str> {
    env::var_os(RUST_LOG_ENV)
        .is_none()
        .then_some(DEFAULT_RUST_LOG)
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

fn preferred_output_channel(output: &CommandOutput) -> String {
    let stderr = String::from_utf8(output.stderr.clone()).unwrap_or_default();
    let stdout = String::from_utf8(output.stdout.clone()).unwrap_or_default();
    if stderr.trim().is_empty() {
        stdout
    } else {
        stderr
    }
}

struct CommandOutput {
    status: ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}
