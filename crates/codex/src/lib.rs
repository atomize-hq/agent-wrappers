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
//! - [`CodexClient::apply`] / [`CodexClient::diff`] to run `codex apply <TASK_ID>` and `codex cloud diff <TASK_ID>`, echo stdout/stderr according to the builder (`mirror_stdout` / `quiet`), and return captured output + exit status.
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
//! - `codex exec --json resume --last [-]` streams the same `thread/turn/item` events as `codex exec --json` but starts from an existing session (`thread.resumed`).
//! - Apply/diff require task IDs: `codex apply <TASK_ID>` applies a diff, and `codex cloud diff <TASK_ID>` prints a cloud task diff when supported by the binary.
//! - Convenience: [`CodexClient::apply`] / [`CodexClient::diff`] will append `<TASK_ID>` from `CODEX_TASK_ID` when set; otherwise they still spawn the command and return the non-zero exit status/output from the CLI.
//! - `crates/codex/examples/resume_apply.rs` shows a CLI-native resume/apply flow and ships `--sample` fixtures for offline inspection.
//!
//! ## Servers and capability detection
//! - Integrate the stdio servers via `codex mcp-server` / `codex app-server` (`crates/codex/examples/mcp_codex_flow.rs`, `mcp_codex_tool.rs`, `mcp_codex_reply.rs`, `app_server_turns.rs`, `app_server_thread_turn.rs`) to drive JSON-RPC flows, approvals, and shutdown.
//! - `probe_capabilities` and the `feature_detection` example focus on `--output-schema`, `--add-dir`, `codex login --mcp`, and `codex features list` availability; other subcommand drift (like cloud-only commands) is surfaced by the parity snapshot/reports in `cli_manifests/codex/`.
//!
//! More end-to-end flows and CLI mappings live in `crates/codex/README.md` and `crates/codex/EXAMPLES.md`.
//!
//! ## Capability/versioning surfaces (Workstream F)
//! - `probe_capabilities` captures `--version`, `features list`, and `--help` hints into a `CodexCapabilities` snapshot with `collected_at` timestamps and `BinaryFingerprint` metadata keyed by canonical binary path.
//! - Guard helpers (`guard_output_schema`, `guard_add_dir`, `guard_mcp_login`, `guard_features_list`) keep optional flags disabled when support is unknown and return operator-facing notes for unsupported features.
//! - Cache controls: `CapabilityCachePolicy::{PreferCache, Refresh, Bypass}` plus builder helpers steer cache reuse. Use `Refresh` for TTL/backoff windows or hot-swaps that reuse the same binary path; use `Bypass` when metadata is missing (FUSE/overlay filesystems) or when you need an isolated probe.
//! - TTL/backoff helper: `capability_cache_ttl_decision` inspects `collected_at` to suggest when to reuse, refresh, or bypass cached snapshots and stretches the recommended policy when metadata is missing.
//! - Overrides + persistence: `capability_snapshot`, `capability_overrides`, `write_capabilities_snapshot`, `read_capabilities_snapshot`, and `capability_snapshot_matches_binary` let hosts reuse snapshots across processes and fall back to probes when fingerprints diverge.

mod apply_diff;
mod auth;
mod builder;
mod bundled_binary;
mod cli;
mod commands;
mod events;
mod exec;
mod execpolicy;
mod home;
pub mod jsonl;
pub mod mcp;
mod process;
pub mod wrapper_coverage_manifest;

pub use apply_diff::{ApplyDiffArtifacts, CloudApplyRequest, CloudDiffRequest};
pub use auth::{AuthSessionHelper, CodexAuthMethod, CodexAuthStatus, CodexLogoutStatus};
pub use builder::{
    ApprovalPolicy, CliOverrides, CliOverridesPatch, CodexClientBuilder, ColorMode, ConfigOverride,
    FeatureToggles, FlagState, LocalProvider, ModelVerbosity, ReasoningEffort, ReasoningOverrides,
    ReasoningSummary, ReasoningSummaryFormat, SafetyOverride, SandboxMode,
};
pub use bundled_binary::{
    default_bundled_platform_label, resolve_bundled_binary, BundledBinary, BundledBinaryError,
    BundledBinarySpec,
};
pub use cli::{
    AppServerCodegenOutput, AppServerCodegenRequest, AppServerCodegenTarget, CloudExecRequest,
    CloudListOutput, CloudListRequest, CloudOverviewRequest, CloudStatusRequest, CodexFeature,
    CodexFeatureStage, ExecRequest, ExecReviewCommandRequest, FeaturesCommandRequest,
    FeaturesListFormat, FeaturesListOutput, FeaturesListRequest, ForkSessionRequest,
    HelpCommandRequest, HelpScope, McpAddRequest, McpAddTransport, McpGetRequest, McpListOutput,
    McpListRequest, McpLogoutRequest, McpOauthLoginRequest, McpOverviewRequest, McpRemoveRequest,
    ResponsesApiProxyHandle, ResponsesApiProxyInfo, ResponsesApiProxyRequest, ResumeSessionRequest,
    ReviewCommandRequest, SandboxCommandRequest, SandboxPlatform, SandboxRun, StdioToUdsRequest,
};
pub use events::{
    CommandExecutionDelta, CommandExecutionState, EventError, FileChangeDelta, FileChangeKind,
    FileChangeState, ItemDelta, ItemDeltaPayload, ItemEnvelope, ItemFailure, ItemPayload,
    ItemSnapshot, ItemStatus, McpToolCallDelta, McpToolCallState, TextContent, TextDelta,
    ThreadEvent, ThreadStarted, TodoItem, TodoListDelta, TodoListState, ToolCallStatus,
    TurnCompleted, TurnFailed, TurnStarted, WebSearchDelta, WebSearchState, WebSearchStatus,
};
pub use exec::{
    DynExecCompletion, DynThreadEventStream, ExecCompletion, ExecStream, ExecStreamError,
    ExecStreamRequest, ResumeRequest, ResumeSelector,
};
pub use execpolicy::{
    ExecPolicyCheckRequest, ExecPolicyCheckResult, ExecPolicyDecision, ExecPolicyEvaluation,
    ExecPolicyMatch, ExecPolicyNoMatch, ExecPolicyRuleMatch,
};
pub use home::{AuthSeedError, AuthSeedOptions, AuthSeedOutcome, CodexHomeLayout};
pub use jsonl::{
    thread_event_jsonl_file, thread_event_jsonl_reader, JsonlThreadEventParser,
    ThreadEventJsonlFileReader, ThreadEventJsonlReader, ThreadEventJsonlRecord,
};

use std::{
    env,
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
    process::ExitStatus,
    time::{Duration, SystemTime},
};

use builder::{apply_cli_overrides, resolve_cli_overrides};
use home::CommandEnvironment;
use process::{command_output_text, spawn_with_retry, tee_stream, CommandOutput, ConsoleTarget};
use tempfile::TempDir;
use thiserror::Error;
use tokio::{process::Command, time};
use tracing::warn;

#[cfg(test)]
use tokio::sync::mpsc;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(120);
const CODEX_BINARY_ENV: &str = "CODEX_BINARY";
const CODEX_HOME_ENV: &str = "CODEX_HOME";
const RUST_LOG_ENV: &str = "RUST_LOG";
const DEFAULT_RUST_LOG: &str = "error";

#[cfg(test)]
use builder::{
    cli_override_args, reasoning_config_for, DEFAULT_REASONING_CONFIG_GPT5,
    DEFAULT_REASONING_CONFIG_GPT5_1, DEFAULT_REASONING_CONFIG_GPT5_CODEX,
};

fn normalize_non_empty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then_some(trimmed.to_string())
}

#[cfg(test)]
fn bundled_binary_filename(platform: &str) -> &'static str {
    bundled_binary::bundled_binary_filename(platform)
}

mod capabilities;
mod version;
pub use capabilities::*;
pub use version::update_advisory_from_capabilities;

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

    /// Runs `codex features` and returns captured output.
    pub async fn features(
        &self,
        request: FeaturesCommandRequest,
    ) -> Result<ApplyDiffArtifacts, CodexError> {
        self.run_simple_command_with_overrides(vec![OsString::from("features")], request.overrides)
            .await
    }

    /// Runs `codex <scope> help [COMMAND]...` and returns captured output.
    pub async fn help(
        &self,
        request: HelpCommandRequest,
    ) -> Result<ApplyDiffArtifacts, CodexError> {
        let mut args: Vec<OsString> = request
            .scope
            .argv_prefix()
            .iter()
            .map(|value| OsString::from(*value))
            .collect();
        args.extend(request.command.into_iter().map(OsString::from));
        self.run_simple_command_with_overrides(args, request.overrides)
            .await
    }

    /// Runs `codex review [OPTIONS] [PROMPT]` and returns captured output.
    pub async fn review(
        &self,
        request: ReviewCommandRequest,
    ) -> Result<ApplyDiffArtifacts, CodexError> {
        if matches!(request.prompt.as_deref(), Some(prompt) if prompt.trim().is_empty()) {
            return Err(CodexError::EmptyPrompt);
        }

        let mut args = vec![OsString::from("review")];
        if let Some(base) = request.base {
            if !base.trim().is_empty() {
                args.push(OsString::from("--base"));
                args.push(OsString::from(base));
            }
        }
        if let Some(commit) = request.commit {
            if !commit.trim().is_empty() {
                args.push(OsString::from("--commit"));
                args.push(OsString::from(commit));
            }
        }
        if let Some(title) = request.title {
            if !title.trim().is_empty() {
                args.push(OsString::from("--title"));
                args.push(OsString::from(title));
            }
        }
        if request.uncommitted {
            args.push(OsString::from("--uncommitted"));
        }
        if let Some(prompt) = request.prompt {
            if !prompt.trim().is_empty() {
                args.push(OsString::from(prompt));
            }
        }

        self.run_simple_command_with_overrides(args, request.overrides)
            .await
    }

    /// Runs `codex exec review [OPTIONS] [PROMPT]` and returns captured output.
    pub async fn exec_review(
        &self,
        request: ExecReviewCommandRequest,
    ) -> Result<ApplyDiffArtifacts, CodexError> {
        if matches!(request.prompt.as_deref(), Some(prompt) if prompt.trim().is_empty()) {
            return Err(CodexError::EmptyPrompt);
        }

        let mut args = vec![OsString::from("exec"), OsString::from("review")];
        if let Some(base) = request.base {
            if !base.trim().is_empty() {
                args.push(OsString::from("--base"));
                args.push(OsString::from(base));
            }
        }
        if let Some(commit) = request.commit {
            if !commit.trim().is_empty() {
                args.push(OsString::from("--commit"));
                args.push(OsString::from(commit));
            }
        }
        if request.json {
            args.push(OsString::from("--json"));
        }
        if request.skip_git_repo_check {
            args.push(OsString::from("--skip-git-repo-check"));
        }
        if let Some(title) = request.title {
            if !title.trim().is_empty() {
                args.push(OsString::from("--title"));
                args.push(OsString::from(title));
            }
        }
        if request.uncommitted {
            args.push(OsString::from("--uncommitted"));
        }
        if let Some(prompt) = request.prompt {
            if !prompt.trim().is_empty() {
                args.push(OsString::from(prompt));
            }
        }

        self.run_simple_command_with_overrides(args, request.overrides)
            .await
    }

    /// Runs `codex fork [OPTIONS] [SESSION_ID] [PROMPT]` and returns captured output.
    pub async fn fork_session(
        &self,
        request: ForkSessionRequest,
    ) -> Result<ApplyDiffArtifacts, CodexError> {
        if matches!(request.prompt.as_deref(), Some(prompt) if prompt.trim().is_empty()) {
            return Err(CodexError::EmptyPrompt);
        }

        let mut args = vec![OsString::from("fork")];
        if request.all {
            args.push(OsString::from("--all"));
        }
        if request.last {
            args.push(OsString::from("--last"));
        }
        if let Some(session_id) = request.session_id {
            if !session_id.trim().is_empty() {
                args.push(OsString::from(session_id));
            }
        }
        if let Some(prompt) = request.prompt {
            if !prompt.trim().is_empty() {
                args.push(OsString::from(prompt));
            }
        }

        self.run_simple_command_with_overrides(args, request.overrides)
            .await
    }

    /// Runs `codex cloud --help` and returns captured output.
    pub async fn cloud_overview(
        &self,
        request: CloudOverviewRequest,
    ) -> Result<ApplyDiffArtifacts, CodexError> {
        self.run_simple_command_with_overrides(
            vec![OsString::from("cloud"), OsString::from("--help")],
            request.overrides,
        )
        .await
    }

    /// Lists Codex Cloud tasks via `codex cloud list`.
    pub async fn cloud_list(
        &self,
        request: CloudListRequest,
    ) -> Result<CloudListOutput, CodexError> {
        let CloudListRequest {
            json,
            env_id,
            limit,
            cursor,
            overrides,
        } = request;

        let mut args = vec![OsString::from("cloud"), OsString::from("list")];
        if let Some(env_id) = env_id {
            args.push(OsString::from("--env"));
            args.push(OsString::from(env_id));
        }
        if let Some(limit) = limit {
            args.push(OsString::from("--limit"));
            args.push(OsString::from(limit.to_string()));
        }
        if let Some(cursor) = cursor {
            args.push(OsString::from("--cursor"));
            args.push(OsString::from(cursor));
        }
        if json {
            args.push(OsString::from("--json"));
        }

        let artifacts = self
            .run_simple_command_with_overrides(args, overrides)
            .await?;
        let parsed = if json {
            Some(serde_json::from_str(&artifacts.stdout).map_err(|source| {
                CodexError::JsonParse {
                    context: "cloud list",
                    stdout: artifacts.stdout.clone(),
                    source,
                }
            })?)
        } else {
            None
        };

        Ok(CloudListOutput {
            status: artifacts.status,
            stdout: artifacts.stdout,
            stderr: artifacts.stderr,
            json: parsed,
        })
    }

    /// Shows the status of a Codex Cloud task via `codex cloud status <TASK_ID>`.
    pub async fn cloud_status(
        &self,
        request: CloudStatusRequest,
    ) -> Result<ApplyDiffArtifacts, CodexError> {
        let task_id = request.task_id.trim();
        if task_id.is_empty() {
            return Err(CodexError::EmptyTaskId);
        }

        self.run_simple_command_with_overrides(
            vec![
                OsString::from("cloud"),
                OsString::from("status"),
                OsString::from(task_id),
            ],
            request.overrides,
        )
        .await
    }

    /// Shows the unified diff for a Codex Cloud task via `codex cloud diff [--attempt N] <TASK_ID>`.
    pub async fn cloud_diff(
        &self,
        request: CloudDiffRequest,
    ) -> Result<ApplyDiffArtifacts, CodexError> {
        let task_id = request.task_id.trim();
        if task_id.is_empty() {
            return Err(CodexError::EmptyTaskId);
        }

        let mut args = vec![OsString::from("cloud"), OsString::from("diff")];
        if let Some(attempt) = request.attempt {
            args.push(OsString::from("--attempt"));
            args.push(OsString::from(attempt.to_string()));
        }
        args.push(OsString::from(task_id));
        self.run_simple_command_with_overrides(args, request.overrides)
            .await
    }

    /// Applies the diff for a Codex Cloud task locally via `codex cloud apply [--attempt N] <TASK_ID>`.
    pub async fn cloud_apply(
        &self,
        request: CloudApplyRequest,
    ) -> Result<ApplyDiffArtifacts, CodexError> {
        let task_id = request.task_id.trim();
        if task_id.is_empty() {
            return Err(CodexError::EmptyTaskId);
        }

        let mut args = vec![OsString::from("cloud"), OsString::from("apply")];
        if let Some(attempt) = request.attempt {
            args.push(OsString::from("--attempt"));
            args.push(OsString::from(attempt.to_string()));
        }
        args.push(OsString::from(task_id));
        self.run_simple_command_with_overrides(args, request.overrides)
            .await
    }

    /// Submits a new Codex Cloud task via `codex cloud exec`.
    pub async fn cloud_exec(
        &self,
        request: CloudExecRequest,
    ) -> Result<ApplyDiffArtifacts, CodexError> {
        let env_id = request.env_id.trim();
        if env_id.is_empty() {
            return Err(CodexError::EmptyEnvId);
        }

        let mut args = vec![OsString::from("cloud"), OsString::from("exec")];
        args.push(OsString::from("--env"));
        args.push(OsString::from(env_id));
        if let Some(attempts) = request.attempts {
            args.push(OsString::from("--attempts"));
            args.push(OsString::from(attempts.to_string()));
        }
        if let Some(branch) = request.branch {
            args.push(OsString::from("--branch"));
            args.push(OsString::from(branch));
        }
        if let Some(query) = request.query {
            args.push(OsString::from(query));
        }

        self.run_simple_command_with_overrides(args, request.overrides)
            .await
    }

    /// Runs `codex mcp --help` and returns captured output.
    pub async fn mcp_overview(
        &self,
        request: McpOverviewRequest,
    ) -> Result<ApplyDiffArtifacts, CodexError> {
        self.run_simple_command_with_overrides(
            vec![OsString::from("mcp"), OsString::from("--help")],
            request.overrides,
        )
        .await
    }

    /// Lists configured MCP servers via `codex mcp list`.
    pub async fn mcp_list(&self, request: McpListRequest) -> Result<McpListOutput, CodexError> {
        let McpListRequest { json, overrides } = request;
        let mut args = vec![OsString::from("mcp"), OsString::from("list")];
        if json {
            args.push(OsString::from("--json"));
        }

        let artifacts = self
            .run_simple_command_with_overrides(args, overrides)
            .await?;
        let parsed = if json {
            Some(serde_json::from_str(&artifacts.stdout).map_err(|source| {
                CodexError::JsonParse {
                    context: "mcp list",
                    stdout: artifacts.stdout.clone(),
                    source,
                }
            })?)
        } else {
            None
        };

        Ok(McpListOutput {
            status: artifacts.status,
            stdout: artifacts.stdout,
            stderr: artifacts.stderr,
            json: parsed,
        })
    }

    /// Gets a configured MCP server entry via `codex mcp get <NAME>`.
    pub async fn mcp_get(&self, request: McpGetRequest) -> Result<McpListOutput, CodexError> {
        let name = request.name.trim();
        if name.is_empty() {
            return Err(CodexError::EmptyMcpServerName);
        }

        let mut args = vec![OsString::from("mcp"), OsString::from("get")];
        if request.json {
            args.push(OsString::from("--json"));
        }
        args.push(OsString::from(name));

        let artifacts = self
            .run_simple_command_with_overrides(args, request.overrides)
            .await?;
        let parsed = if request.json {
            Some(serde_json::from_str(&artifacts.stdout).map_err(|source| {
                CodexError::JsonParse {
                    context: "mcp get",
                    stdout: artifacts.stdout.clone(),
                    source,
                }
            })?)
        } else {
            None
        };

        Ok(McpListOutput {
            status: artifacts.status,
            stdout: artifacts.stdout,
            stderr: artifacts.stderr,
            json: parsed,
        })
    }

    /// Adds an MCP server configuration entry via `codex mcp add`.
    pub async fn mcp_add(&self, request: McpAddRequest) -> Result<ApplyDiffArtifacts, CodexError> {
        let name = request.name.trim();
        if name.is_empty() {
            return Err(CodexError::EmptyMcpServerName);
        }

        let mut args = vec![
            OsString::from("mcp"),
            OsString::from("add"),
            OsString::from(name),
        ];
        match request.transport {
            McpAddTransport::StreamableHttp {
                url,
                bearer_token_env_var,
            } => {
                let url = url.trim();
                if url.is_empty() {
                    return Err(CodexError::EmptyMcpUrl);
                }
                args.push(OsString::from("--url"));
                args.push(OsString::from(url));
                if let Some(env_var) = bearer_token_env_var {
                    if !env_var.trim().is_empty() {
                        args.push(OsString::from("--bearer-token-env-var"));
                        args.push(OsString::from(env_var));
                    }
                }
            }
            McpAddTransport::Stdio { env, command } => {
                if command.is_empty() {
                    return Err(CodexError::EmptyMcpCommand);
                }
                for (key, value) in env {
                    let key = key.trim();
                    if key.is_empty() {
                        continue;
                    }
                    args.push(OsString::from("--env"));
                    args.push(OsString::from(format!("{key}={value}")));
                }
                args.push(OsString::from("--"));
                args.extend(command);
            }
        }

        self.run_simple_command_with_overrides(args, request.overrides)
            .await
    }

    /// Removes an MCP server configuration entry via `codex mcp remove <NAME>`.
    pub async fn mcp_remove(
        &self,
        request: McpRemoveRequest,
    ) -> Result<ApplyDiffArtifacts, CodexError> {
        let name = request.name.trim();
        if name.is_empty() {
            return Err(CodexError::EmptyMcpServerName);
        }

        self.run_simple_command_with_overrides(
            vec![
                OsString::from("mcp"),
                OsString::from("remove"),
                OsString::from(name),
            ],
            request.overrides,
        )
        .await
    }

    /// Deauthenticates from an MCP server via `codex mcp logout <NAME>`.
    pub async fn mcp_logout(
        &self,
        request: McpLogoutRequest,
    ) -> Result<ApplyDiffArtifacts, CodexError> {
        let name = request.name.trim();
        if name.is_empty() {
            return Err(CodexError::EmptyMcpServerName);
        }

        self.run_simple_command_with_overrides(
            vec![
                OsString::from("mcp"),
                OsString::from("logout"),
                OsString::from(name),
            ],
            request.overrides,
        )
        .await
    }

    /// Spawns `codex mcp login <NAME> [--scopes ...]`.
    pub fn spawn_mcp_oauth_login_process(
        &self,
        request: McpOauthLoginRequest,
    ) -> Result<tokio::process::Child, CodexError> {
        let name = request.name.trim();
        if name.is_empty() {
            return Err(CodexError::EmptyMcpServerName);
        }

        let resolved_overrides = resolve_cli_overrides(
            &self.cli_overrides,
            &request.overrides,
            self.model.as_deref(),
        );

        let mut command = Command::new(self.command_env.binary_path());
        command
            .arg("mcp")
            .arg("login")
            .arg(name)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);

        if !request.scopes.is_empty() {
            command.arg("--scopes").arg(request.scopes.join(","));
        }

        apply_cli_overrides(&mut command, &resolved_overrides, true);
        self.command_env.apply(&mut command)?;

        spawn_with_retry(&mut command, self.command_env.binary_path())
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
                    version = Some(version::parse_version_output(&text));
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
                if let Some(parsed) = version::parse_features_from_json(&text) {
                    version::merge_feature_flags(&mut features, parsed);
                    parsed_features = version::detected_feature_flags(&features);
                } else if !text.is_empty() {
                    let parsed = version::parse_features_from_text(&text);
                    version::merge_feature_flags(&mut features, parsed);
                    parsed_features = version::detected_feature_flags(&features);
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
                    let parsed = version::parse_features_from_text(&text);
                    version::merge_feature_flags(&mut features, parsed);
                }
                Err(error) => warn!(
                    ?error,
                    binary = ?cache_key.binary_path,
                    "codex features list probe failed"
                ),
            }
        }

        if version::should_run_help_fallback(&features) {
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
                    let parsed = version::parse_help_output(&text);
                    version::merge_feature_flags(&mut features, parsed);
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

    async fn run_simple_command_with_overrides(
        &self,
        args: Vec<OsString>,
        overrides: CliOverridesPatch,
    ) -> Result<ApplyDiffArtifacts, CodexError> {
        let dir_ctx = self.directory_context()?;
        let resolved_overrides =
            resolve_cli_overrides(&self.cli_overrides, &overrides, self.model.as_deref());

        let mut command = Command::new(self.command_env.binary_path());
        command
            .args(args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .current_dir(dir_ctx.path());

        apply_cli_overrides(&mut command, &resolved_overrides, true);

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

        let timeout = self.timeout;
        let wait_task = async move {
            let _dir_ctx = dir_ctx;
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

        let (status, stdout_bytes, stderr_bytes) = if timeout.is_zero() {
            wait_task.await?
        } else {
            match time::timeout(timeout, wait_task).await {
                Ok(result) => result?,
                Err(_) => {
                    return Err(CodexError::Timeout { timeout });
                }
            }
        };

        if !status.success() {
            return Err(CodexError::NonZeroExit {
                status,
                stderr: String::from_utf8(stderr_bytes)?,
            });
        }

        Ok(ApplyDiffArtifacts {
            status,
            stdout: String::from_utf8(stdout_bytes)?,
            stderr: String::from_utf8(stderr_bytes)?,
        })
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
    #[error("failed to parse {context} JSON output: {source}")]
    JsonParse {
        context: &'static str,
        stdout: String,
        #[source]
        source: serde_json::Error,
    },
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
    #[error("task id must not be empty")]
    EmptyTaskId,
    #[error("environment id must not be empty")]
    EmptyEnvId,
    #[error("MCP server name must not be empty")]
    EmptyMcpServerName,
    #[error("MCP server command must not be empty")]
    EmptyMcpCommand,
    #[error("MCP server URL must not be empty")]
    EmptyMcpUrl,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::parse_login_success;
    use crate::builder::ResolvedCliOverrides;
    use futures_util::{pin_mut, StreamExt};
    use semver::Version;
    use serde_json::json;
    use std::collections::HashMap;
    use std::fs as std_fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use std::sync::OnceLock;
    use std::time::{Duration, SystemTime};
    use tokio::{
        fs,
        io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    };

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
        let forward_handle =
            tokio::spawn(crate::jsonl::forward_json_events(reader, tx, false, None));

        for line in &lines {
            writer.write_all(line.as_bytes()).await.unwrap();
            writer.write_all(b"\n").await.unwrap();
        }
        writer.shutdown().await.unwrap();

        let stream = crate::jsonl::EventChannelStream::new(rx, None);
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
        let forward_handle =
            tokio::spawn(crate::jsonl::forward_json_events(reader, tx, false, None));

        writer
            .write_all(br#"{"type":"thread.started","thread_id":"thread-err"}"#)
            .await
            .unwrap();
        writer.write_all(b"\nthis is not json\n").await.unwrap();
        writer.shutdown().await.unwrap();

        let stream = crate::jsonl::EventChannelStream::new(rx, None);
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
        let log_sink = crate::jsonl::JsonLogSink::new(log_path.clone())
            .await
            .unwrap();
        let forward_handle = tokio::spawn(crate::jsonl::forward_json_events(
            reader,
            tx,
            false,
            Some(log_sink),
        ));

        let stream = crate::jsonl::EventChannelStream::new(rx, None);
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
        let log_sink = crate::jsonl::JsonLogSink::new(log_path.clone())
            .await
            .unwrap();
        let forward_handle = tokio::spawn(crate::jsonl::forward_json_events(
            reader,
            tx,
            false,
            Some(log_sink),
        ));

        for line in &lines {
            writer.write_all(line.as_bytes()).await.unwrap();
            writer.write_all(b"\n").await.unwrap();
        }
        writer.shutdown().await.unwrap();

        let stream = crate::jsonl::EventChannelStream::new(rx, None);
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
        let stream = crate::jsonl::EventChannelStream::new(rx, Some(Duration::from_millis(5)));
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
    async fn supports_help_review_fork_resume_and_features_commands() {
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

        let features = client
            .features(FeaturesCommandRequest::new())
            .await
            .unwrap();
        assert_eq!(
            features.stdout.lines().collect::<Vec<_>>(),
            vec!["features"]
        );

        let help = client
            .help(HelpCommandRequest::new(HelpScope::Root).command(["exec", "review"]))
            .await
            .unwrap();
        assert_eq!(
            help.stdout.lines().collect::<Vec<_>>(),
            vec!["help", "exec", "review"]
        );

        let review = client
            .review(
                ReviewCommandRequest::new()
                    .base("main")
                    .commit("abc123")
                    .title("hello")
                    .uncommitted(true)
                    .prompt("please review"),
            )
            .await
            .unwrap();
        assert_eq!(
            review.stdout.lines().collect::<Vec<_>>(),
            vec![
                "review",
                "--base",
                "main",
                "--commit",
                "abc123",
                "--title",
                "hello",
                "--uncommitted",
                "please review"
            ]
        );

        let exec_review = client
            .exec_review(
                ExecReviewCommandRequest::new()
                    .base("main")
                    .commit("abc123")
                    .title("hello")
                    .uncommitted(true)
                    .json(true)
                    .prompt("please review"),
            )
            .await
            .unwrap();
        assert_eq!(
            exec_review.stdout.lines().collect::<Vec<_>>(),
            vec![
                "exec",
                "review",
                "--base",
                "main",
                "--commit",
                "abc123",
                "--json",
                "--skip-git-repo-check",
                "--title",
                "hello",
                "--uncommitted",
                "please review"
            ]
        );

        let resume = client
            .resume_session(
                ResumeSessionRequest::new()
                    .all(true)
                    .last(true)
                    .session_id("sess-1")
                    .prompt("resume prompt"),
            )
            .await
            .unwrap();
        assert_eq!(
            resume.stdout.lines().collect::<Vec<_>>(),
            vec!["resume", "--all", "--last", "sess-1", "resume prompt"]
        );

        let fork = client
            .fork_session(
                ForkSessionRequest::new()
                    .all(true)
                    .last(true)
                    .session_id("sess-1")
                    .prompt("fork prompt"),
            )
            .await
            .unwrap();
        assert_eq!(
            fork.stdout.lines().collect::<Vec<_>>(),
            vec!["fork", "--all", "--last", "sess-1", "fork prompt"]
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn cloud_list_parses_json_and_maps_args() {
        let dir = tempfile::tempdir().unwrap();
        let script_path = write_fake_codex(
            dir.path(),
            r#"#!/usr/bin/env bash
printf "%s\n" "$@" 1>&2
cat <<'JSON'
{"tasks":[],"cursor":null}
JSON
"#,
        );

        let client = CodexClient::builder()
            .binary(&script_path)
            .mirror_stdout(false)
            .quiet(true)
            .build();

        let output = client
            .cloud_list(
                CloudListRequest::new()
                    .json(true)
                    .env_id("env-1")
                    .limit(3)
                    .cursor("cur-1"),
            )
            .await
            .unwrap();

        assert_eq!(output.json, Some(json!({"tasks": [], "cursor": null})));
        assert_eq!(
            output.stderr.lines().collect::<Vec<_>>(),
            vec!["cloud", "list", "--env", "env-1", "--limit", "3", "--cursor", "cur-1", "--json"]
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn cloud_exec_maps_args_and_rejects_empty_env_id() {
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

        let output = client
            .cloud_exec(
                CloudExecRequest::new("env-1")
                    .attempts(2)
                    .branch("main")
                    .query("hello"),
            )
            .await
            .unwrap();
        assert_eq!(
            output.stdout.lines().collect::<Vec<_>>(),
            vec![
                "cloud",
                "exec",
                "--env",
                "env-1",
                "--attempts",
                "2",
                "--branch",
                "main",
                "hello"
            ]
        );

        let err = client
            .cloud_exec(CloudExecRequest::new("  "))
            .await
            .unwrap_err();
        assert!(matches!(err, CodexError::EmptyEnvId));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn mcp_list_get_and_add_map_args_and_parse_json() {
        let dir = tempfile::tempdir().unwrap();
        let script_path = write_fake_codex(
            dir.path(),
            r#"#!/usr/bin/env bash
printf "%s\n" "$@" 1>&2
cat <<'JSON'
{"servers":[{"name":"files"}]}
JSON
"#,
        );

        let client = CodexClient::builder()
            .binary(&script_path)
            .mirror_stdout(false)
            .quiet(true)
            .build();

        let list = client
            .mcp_list(McpListRequest::new().json(true))
            .await
            .unwrap();
        assert_eq!(list.json, Some(json!({"servers": [{"name": "files"}]})));
        assert_eq!(
            list.stderr.lines().collect::<Vec<_>>(),
            vec!["mcp", "list", "--json"]
        );

        let get = client
            .mcp_get(McpGetRequest::new("files").json(true))
            .await
            .unwrap();
        assert_eq!(get.json, Some(json!({"servers": [{"name": "files"}]})));
        assert_eq!(
            get.stderr.lines().collect::<Vec<_>>(),
            vec!["mcp", "get", "--json", "files"]
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn mcp_add_maps_transports_and_validates_required_fields() {
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

        let stdio = client
            .mcp_add(
                McpAddRequest::stdio("files", vec![OsString::from("node"), OsString::from("srv")])
                    .env("TOKEN", "abc"),
            )
            .await
            .unwrap();
        assert_eq!(
            stdio.stdout.lines().collect::<Vec<_>>(),
            vec![
                "mcp",
                "add",
                "files",
                "--env",
                "TOKEN=abc",
                "--",
                "node",
                "srv"
            ]
        );

        let http = client
            .mcp_add(
                McpAddRequest::streamable_http("http", "https://example.test")
                    .bearer_token_env_var("TOKEN_ENV"),
            )
            .await
            .unwrap();
        assert_eq!(
            http.stdout.lines().collect::<Vec<_>>(),
            vec![
                "mcp",
                "add",
                "http",
                "--url",
                "https://example.test",
                "--bearer-token-env-var",
                "TOKEN_ENV"
            ]
        );

        let err = client
            .mcp_add(McpAddRequest::stdio("files", Vec::new()))
            .await
            .unwrap_err();
        assert!(matches!(err, CodexError::EmptyMcpCommand));

        let err = client
            .mcp_add(McpAddRequest::streamable_http("bad", "  "))
            .await
            .unwrap_err();
        assert!(matches!(err, CodexError::EmptyMcpUrl));
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
            version: Some(version::parse_version_output(raw_version)),
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
            version: Some(version::parse_version_output("codex 9.9.9-nightly")),
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
elif [[ "$cmd" == "cloud" && "${2:-}" == "diff" ]]; then
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
        let parsed = version::parse_version_output("codex v3.4.5-nightly (commit abc1234)");
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
        let mut context = crate::jsonl::StreamContext::default();
        // thread.started establishes thread context
        let thread_line = r#"{"type":"thread.started","thread_id":"thread-1"}"#;
        let thread_event = crate::jsonl::normalize_thread_event(thread_line, &mut context).unwrap();
        match thread_event {
            ThreadEvent::ThreadStarted(t) => assert_eq!(t.thread_id, "thread-1"),
            other => panic!("unexpected event: {other:?}"),
        }
        // turn.started without thread_id should inherit
        let turn_line = r#"{"type":"turn.started","turn_id":"turn-1"}"#;
        let turn_event = crate::jsonl::normalize_thread_event(turn_line, &mut context).unwrap();
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
        let item_event = crate::jsonl::normalize_thread_event(item_line, &mut context).unwrap();
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
        let mut context = crate::jsonl::StreamContext::default();
        let line = r#"{"type":"turn.started"}"#;
        let err = crate::jsonl::normalize_thread_event(line, &mut context).unwrap_err();
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
            version: Some(version::parse_version_output("codex 0.0.1")),
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
                    version: Some(version::parse_version_output("codex 9.9.9")),
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
        let parsed_json = version::parse_features_from_json(json).unwrap();
        assert!(parsed_json.supports_output_schema);
        assert!(parsed_json.supports_add_dir);
        assert!(parsed_json.supports_mcp_login);

        let text = "Features: output-schema add-dir login --mcp";
        let parsed_text = version::parse_features_from_text(text);
        assert!(parsed_text.supports_output_schema);
        assert!(parsed_text.supports_add_dir);
        assert!(parsed_text.supports_mcp_login);
    }

    #[test]
    fn parses_feature_list_json_and_text_tables() {
        let json = r#"{"features":[{"name":"json-stream","stage":"stable","enabled":true,"notes":"keep"},{"name":"cloud-exec","stage":"experimental","enabled":false}]}"#;
        let (json_features, json_format) = version::parse_feature_list_output(json, true).unwrap();
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
        let (text_features, text_format) = version::parse_feature_list_output(text, false).unwrap();
        assert_eq!(text_format, FeaturesListFormat::Text);
        assert_eq!(text_features.len(), 2);
        assert_eq!(
            text_features[1].stage,
            Some(CodexFeatureStage::Experimental)
        );
        assert!(!text_features[1].enabled);

        let (fallback_features, fallback_format) =
            version::parse_feature_list_output(text, true).unwrap();
        assert_eq!(fallback_format, FeaturesListFormat::Text);
        assert_eq!(fallback_features.len(), 2);
    }

    #[test]
    fn parses_help_output_flags() {
        let help =
            "Usage: codex --output-schema ... add-dir ... login --mcp. See `codex features list`.";
        let parsed = version::parse_help_output(help);
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
            version: Some(version::parse_version_output("codex 9.9.9-custom")),
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
        let version_override =
            version::parse_version_output("codex 9.9.9-nightly (commit beefcafe)");

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
if [[ "$1" == "exec" ]]; then
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
