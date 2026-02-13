use std::{collections::BTreeMap, path::PathBuf, time::Duration};

use tokio::process::Command;

use crate::{
    builder::ClaudeClientBuilder,
    commands::command::ClaudeCommandRequest,
    commands::mcp::{McpAddJsonRequest, McpAddRequest, McpGetRequest, McpRemoveRequest},
    commands::print::{ClaudeOutputFormat, ClaudePrintRequest},
    parse_stream_json_lines, process, ClaudeCodeError, CommandOutput, StreamJsonLineOutcome,
};

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
        if request.prompt.is_none() && request.stdin.is_none() {
            return Err(ClaudeCodeError::InvalidRequest(
                "either prompt or stdin_bytes must be provided".to_string(),
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

    pub async fn mcp_remove(&self, req: McpRemoveRequest) -> Result<CommandOutput, ClaudeCodeError> {
        self.run_command(req.into_command()).await
    }

    pub async fn mcp_add_json(
        &self,
        req: McpAddJsonRequest,
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
