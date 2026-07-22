use std::ffi::OsString;
use std::process::ExitStatus;

use crate::{ApplyDiffArtifacts, CodexClient, CodexError, NonTuiCommandRequest};

impl CodexClient {
    /// Runs a packet-maintained non-TUI Codex command.
    ///
    /// This compatibility surface forwards command-specific arguments verbatim
    /// while restricting the command path to [`crate::NonTuiCommand`]. It is
    /// intended for upstream CLI drift that has not earned a specialized typed
    /// request yet; deferred completion generation is intentionally excluded.
    pub async fn run_non_tui_command(
        &self,
        request: NonTuiCommandRequest,
    ) -> Result<ApplyDiffArtifacts, CodexError> {
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
                return Err(CodexError::NonZeroExit {
                    status: invalid_usage_status(),
                    stderr: format!(
                        "non-TUI command `{}` only accepts flag-form passthrough arguments; got bare token `{}`",
                        command.path().join(" "),
                        argument.to_string_lossy()
                    ),
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

        self.run_simple_command_with_overrides(args, overrides)
            .await
    }
}

#[cfg(unix)]
fn invalid_usage_status() -> ExitStatus {
    use std::os::unix::process::ExitStatusExt;

    ExitStatus::from_raw(2 << 8)
}

#[cfg(windows)]
fn invalid_usage_status() -> ExitStatus {
    use std::os::windows::process::ExitStatusExt;

    ExitStatus::from_raw(2)
}
