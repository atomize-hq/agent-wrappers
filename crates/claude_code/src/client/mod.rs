use std::{collections::BTreeMap, path::PathBuf, time::Duration};

use tokio::process::Command;

use crate::{
    builder::ClaudeClientBuilder,
    commands::command::ClaudeCommandRequest,
    commands::doctor::ClaudeDoctorRequest,
    commands::mcp::{
        McpAddFromClaudeDesktopRequest, McpAddJsonRequest, McpAddRequest, McpGetRequest,
        McpRemoveRequest, McpServeRequest,
    },
    commands::plugin::{
        PluginDisableRequest, PluginEnableRequest, PluginInstallRequest, PluginListRequest,
        PluginManifestMarketplaceRequest, PluginManifestRequest, PluginMarketplaceAddRequest,
        PluginMarketplaceListRequest, PluginMarketplaceRemoveRequest, PluginMarketplaceRepoRequest,
        PluginMarketplaceRequest, PluginMarketplaceUpdateRequest, PluginRequest,
        PluginUninstallRequest, PluginUpdateRequest, PluginValidateRequest,
    },
    commands::print::{ClaudeOutputFormat, ClaudePrintRequest},
    commands::update::ClaudeUpdateRequest,
    parse_stream_json_lines, process, ClaudeCodeError, CommandOutput, StreamJsonLineOutcome,
};

mod setup_token;

pub use setup_token::ClaudeSetupTokenSession;

#[derive(Debug, Clone)]
pub struct ClaudeClient {
    pub(crate) binary: Option<PathBuf>,
    pub(crate) working_dir: Option<PathBuf>,
    pub(crate) env: BTreeMap<String, String>,
    pub(crate) timeout: Option<Duration>,
    pub(crate) mirror_stdout: bool,
    pub(crate) mirror_stderr: bool,
}

impl ClaudeClient {
    pub fn builder() -> ClaudeClientBuilder {
        ClaudeClientBuilder::default()
    }

    pub async fn help(&self) -> Result<CommandOutput, ClaudeCodeError> {
        self.run_command(ClaudeCommandRequest::root().arg("--help"))
            .await
    }

    pub async fn version(&self) -> Result<CommandOutput, ClaudeCodeError> {
        self.run_command(ClaudeCommandRequest::root().arg("--version"))
            .await
    }

    pub async fn run_command(
        &self,
        request: ClaudeCommandRequest,
    ) -> Result<CommandOutput, ClaudeCodeError> {
        let binary = self.resolve_binary();
        let mut cmd = Command::new(&binary);
        cmd.args(request.argv());

        if let Some(dir) = self.working_dir.as_ref() {
            cmd.current_dir(dir);
        }

        process::apply_env(&mut cmd, &self.env);

        let timeout = request.timeout.or(self.timeout);
        process::run_command(
            cmd,
            &binary,
            request.stdin.as_deref(),
            timeout,
            self.mirror_stdout,
            self.mirror_stderr,
        )
        .await
    }

    pub async fn print(
        &self,
        request: ClaudePrintRequest,
    ) -> Result<ClaudePrintResult, ClaudeCodeError> {
        let allow_missing_prompt = request.stdin.is_some()
            || request.continue_session
            || request.resume
            || request.resume_value.is_some()
            || request.from_pr;
        if request.prompt.is_none() && !allow_missing_prompt {
            return Err(ClaudeCodeError::InvalidRequest(
                "either prompt, stdin_bytes, or a continuation flag must be provided".to_string(),
            ));
        }

        let binary = self.resolve_binary();
        let mut cmd = Command::new(&binary);
        cmd.args(request.argv());

        if let Some(dir) = self.working_dir.as_ref() {
            cmd.current_dir(dir);
        }

        process::apply_env(&mut cmd, &self.env);

        let timeout = request.timeout.or(self.timeout);
        let output = process::run_command(
            cmd,
            &binary,
            request.stdin.as_deref(),
            timeout,
            self.mirror_stdout,
            self.mirror_stderr,
        )
        .await?;

        let parsed = match request.output_format {
            ClaudeOutputFormat::Json => {
                let v = serde_json::from_slice(&output.stdout)?;
                Some(ClaudeParsedOutput::Json(v))
            }
            ClaudeOutputFormat::StreamJson => {
                let s = String::from_utf8_lossy(&output.stdout);
                Some(ClaudeParsedOutput::StreamJson(parse_stream_json_lines(&s)))
            }
            ClaudeOutputFormat::Text => None,
        };

        Ok(ClaudePrintResult { output, parsed })
    }

    pub async fn mcp_list(&self) -> Result<CommandOutput, ClaudeCodeError> {
        self.run_command(ClaudeCommandRequest::new(["mcp", "list"]))
            .await
    }

    pub async fn mcp_reset_project_choices(&self) -> Result<CommandOutput, ClaudeCodeError> {
        self.run_command(ClaudeCommandRequest::new(["mcp", "reset-project-choices"]))
            .await
    }

    pub async fn mcp_get(&self, req: McpGetRequest) -> Result<CommandOutput, ClaudeCodeError> {
        self.run_command(req.into_command()).await
    }

    pub async fn mcp_add(&self, req: McpAddRequest) -> Result<CommandOutput, ClaudeCodeError> {
        self.run_command(req.into_command()).await
    }

    pub async fn mcp_remove(
        &self,
        req: McpRemoveRequest,
    ) -> Result<CommandOutput, ClaudeCodeError> {
        self.run_command(req.into_command()).await
    }

    pub async fn mcp_add_json(
        &self,
        req: McpAddJsonRequest,
    ) -> Result<CommandOutput, ClaudeCodeError> {
        self.run_command(req.into_command()).await
    }

    pub async fn mcp_serve(&self, req: McpServeRequest) -> Result<CommandOutput, ClaudeCodeError> {
        self.run_command(req.into_command()).await
    }

    pub async fn mcp_add_from_claude_desktop(
        &self,
        req: McpAddFromClaudeDesktopRequest,
    ) -> Result<CommandOutput, ClaudeCodeError> {
        self.run_command(req.into_command()).await
    }

    pub async fn doctor(&self) -> Result<CommandOutput, ClaudeCodeError> {
        self.doctor_with(ClaudeDoctorRequest::new()).await
    }

    pub async fn doctor_with(
        &self,
        req: ClaudeDoctorRequest,
    ) -> Result<CommandOutput, ClaudeCodeError> {
        self.run_command(req.into_command()).await
    }

    pub async fn plugin_list(
        &self,
        req: PluginListRequest,
    ) -> Result<CommandOutput, ClaudeCodeError> {
        self.run_command(req.into_command()).await
    }

    pub async fn plugin(&self, req: PluginRequest) -> Result<CommandOutput, ClaudeCodeError> {
        self.run_command(req.into_command()).await
    }

    pub async fn plugin_enable(
        &self,
        req: PluginEnableRequest,
    ) -> Result<CommandOutput, ClaudeCodeError> {
        self.run_command(req.into_command()).await
    }

    pub async fn plugin_disable(
        &self,
        req: PluginDisableRequest,
    ) -> Result<CommandOutput, ClaudeCodeError> {
        self.run_command(req.into_command()).await
    }

    pub async fn plugin_install(
        &self,
        req: PluginInstallRequest,
    ) -> Result<CommandOutput, ClaudeCodeError> {
        self.run_command(req.into_command()).await
    }

    pub async fn plugin_uninstall(
        &self,
        req: PluginUninstallRequest,
    ) -> Result<CommandOutput, ClaudeCodeError> {
        self.run_command(req.into_command()).await
    }

    pub async fn plugin_update(
        &self,
        req: PluginUpdateRequest,
    ) -> Result<CommandOutput, ClaudeCodeError> {
        self.run_command(req.into_command()).await
    }

    pub async fn plugin_validate(
        &self,
        req: PluginValidateRequest,
    ) -> Result<CommandOutput, ClaudeCodeError> {
        self.run_command(req.into_command()).await
    }

    pub async fn plugin_manifest(
        &self,
        req: PluginManifestRequest,
    ) -> Result<CommandOutput, ClaudeCodeError> {
        self.run_command(req.into_command()).await
    }

    pub async fn plugin_manifest_marketplace(
        &self,
        req: PluginManifestMarketplaceRequest,
    ) -> Result<CommandOutput, ClaudeCodeError> {
        self.run_command(req.into_command()).await
    }

    pub async fn plugin_marketplace_repo(
        &self,
        req: PluginMarketplaceRepoRequest,
    ) -> Result<CommandOutput, ClaudeCodeError> {
        self.run_command(req.into_command()).await
    }

    pub async fn plugin_marketplace(
        &self,
        req: PluginMarketplaceRequest,
    ) -> Result<CommandOutput, ClaudeCodeError> {
        self.run_command(req.into_command()).await
    }

    pub async fn plugin_marketplace_add(
        &self,
        req: PluginMarketplaceAddRequest,
    ) -> Result<CommandOutput, ClaudeCodeError> {
        self.run_command(req.into_command()).await
    }

    pub async fn plugin_marketplace_list(
        &self,
        req: PluginMarketplaceListRequest,
    ) -> Result<CommandOutput, ClaudeCodeError> {
        self.run_command(req.into_command()).await
    }

    pub async fn plugin_marketplace_remove(
        &self,
        req: PluginMarketplaceRemoveRequest,
    ) -> Result<CommandOutput, ClaudeCodeError> {
        self.run_command(req.into_command()).await
    }

    pub async fn plugin_marketplace_update(
        &self,
        req: PluginMarketplaceUpdateRequest,
    ) -> Result<CommandOutput, ClaudeCodeError> {
        self.run_command(req.into_command()).await
    }

    pub async fn update(&self) -> Result<CommandOutput, ClaudeCodeError> {
        self.update_with(ClaudeUpdateRequest::new()).await
    }

    pub async fn update_with(
        &self,
        req: ClaudeUpdateRequest,
    ) -> Result<CommandOutput, ClaudeCodeError> {
        self.run_command(req.into_command()).await
    }

    fn resolve_binary(&self) -> PathBuf {
        if let Some(b) = self.binary.as_ref() {
            return b.clone();
        }
        if let Ok(v) = std::env::var("CLAUDE_BINARY") {
            if !v.trim().is_empty() {
                return PathBuf::from(v);
            }
        }
        PathBuf::from("claude")
    }
}

#[derive(Debug, Clone)]
pub struct ClaudePrintResult {
    pub output: CommandOutput,
    pub parsed: Option<ClaudeParsedOutput>,
}

#[derive(Debug, Clone)]
pub enum ClaudeParsedOutput {
    Json(serde_json::Value),
    StreamJson(Vec<StreamJsonLineOutcome>),
}
