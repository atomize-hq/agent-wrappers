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
mod client_core;
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
    ffi::OsString,
    path::PathBuf,
    process::ExitStatus,
    time::{Duration, SystemTime},
};

use builder::{apply_cli_overrides, resolve_cli_overrides};
use home::CommandEnvironment;
use process::{command_output_text, spawn_with_retry, tee_stream, ConsoleTarget};
use thiserror::Error;
use tokio::process::Command;
use tracing::warn;

#[cfg(test)]
use std::path::Path;

#[cfg(test)]
use tokio::time;

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
mod tests;
