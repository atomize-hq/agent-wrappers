use std::ffi::OsString;

use tokio::process::Command;

use crate::{
    builder::{apply_cli_overrides, resolve_cli_overrides},
    process::spawn_with_retry,
    ApplyDiffArtifacts, CliOverridesPatch, CodexClient, CodexError, NonTuiCommand,
    NonTuiCommandRequest,
};

impl CodexClient {
    /// Runs a packet-maintained non-TUI Codex command.
    ///
    /// This compatibility surface forwards command-specific arguments verbatim
    /// while restricting the command path to [`crate::NonTuiCommand`]. It is
    /// intended for upstream CLI drift that has not earned a specialized typed
    /// request yet; deferred completion generation is intentionally excluded.
    ///
    /// [`NonTuiCommand::AppServer`] and [`NonTuiCommand::ExecServer`] are
    /// long-running stdio servers, so this method rejects them rather than
    /// consuming their streams. Use [`Self::start_non_tui_server`] for those
    /// variants.
    pub async fn run_non_tui_command(
        &self,
        request: NonTuiCommandRequest,
    ) -> Result<ApplyDiffArtifacts, CodexError> {
        let (command, args, overrides) = prepare_non_tui_command(request)?;
        if command.requires_process_handle() {
            return Err(CodexError::NonTuiServerRequiresSpawn {
                command: command.path().join(" "),
            });
        }

        self.run_simple_command_with_overrides(args, overrides)
            .await
    }

    /// Spawns a packet-maintained stdio server and returns its process handle.
    ///
    /// This accepts only [`NonTuiCommand::AppServer`] and
    /// [`NonTuiCommand::ExecServer`]. The returned child has piped stdin,
    /// stdout, and stderr, allowing callers to exchange protocol messages and
    /// manage the server lifecycle themselves.
    pub fn start_non_tui_server(
        &self,
        request: NonTuiCommandRequest,
    ) -> Result<tokio::process::Child, CodexError> {
        let (command_kind, args, overrides) = prepare_non_tui_command(request)?;
        if !command_kind.requires_process_handle() {
            return Err(CodexError::NonTuiCommandIsNotServer {
                command: command_kind.path().join(" "),
            });
        }

        let resolved_overrides =
            resolve_cli_overrides(&self.cli_overrides, &overrides, self.model.as_deref());

        let mut command = Command::new(self.command_env.binary_path());
        command
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .current_dir(self.sandbox_working_dir(None)?);

        apply_cli_overrides(&mut command, &resolved_overrides, true);
        command.args(args);
        self.command_env.apply(&mut command)?;

        spawn_with_retry(&mut command, self.command_env.binary_path())
    }
}

fn prepare_non_tui_command(
    request: NonTuiCommandRequest,
) -> Result<(NonTuiCommand, Vec<OsString>, CliOverridesPatch), CodexError> {
    let NonTuiCommandRequest {
        command,
        arguments,
        dangerously_bypass_hook_trust,
        strict_config,
        overrides,
    } = request;

    if command.requires_flag_only_passthrough() {
        if let Some(argument) = arguments
            .iter()
            .find(|argument| !argument.to_string_lossy().starts_with('-'))
        {
            return Err(CodexError::InvalidNonTuiPassthrough {
                command: command.path().join(" "),
                token: argument.to_string_lossy().into_owned(),
            });
        }
    }

    let mut args = Vec::new();
    if dangerously_bypass_hook_trust {
        args.push(OsString::from("--dangerously-bypass-hook-trust"));
    }
    if strict_config {
        args.push(OsString::from("--strict-config"));
    }
    args.extend(command.path().iter().map(OsString::from));
    args.extend(arguments);

    Ok((command, args, overrides))
}
