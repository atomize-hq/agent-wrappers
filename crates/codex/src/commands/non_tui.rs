use std::ffi::OsString;

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
