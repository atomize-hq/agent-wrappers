use std::{
    path::Path,
    process::{Command, Stdio},
    time::Duration,
};

use wait_timeout::ChildExt;

use super::super::{util, Error};

pub(super) fn run_claude_help_strict(
    claude_binary: &Path,
    path: &[String],
    timeout: Duration,
) -> Result<String, Error> {
    let help =
        run_claude_help_with_timeout(claude_binary, path, timeout).map_err(Error::CommandFailed)?;
    if !help.status.success() {
        return Err(Error::CommandFailed(help.failure_debug));
    }
    Ok(help.text)
}

pub(super) fn run_claude_help_tolerant(
    claude_binary: &Path,
    path: &[String],
    timeout: Duration,
) -> Result<String, String> {
    let help = run_claude_help_with_timeout(claude_binary, path, timeout)?;

    if help.status.success() {
        return Ok(help.text);
    }

    // Some CLIs exit non-zero for `--help` while still printing a full usage block.
    // Prefer keeping snapshot generation moving, but record a stable omission note.
    if looks_like_help(&help.text) {
        return Ok(help.text);
    }

    Err(help.failure_note)
}

struct HelpRun {
    status: std::process::ExitStatus,
    text: String,
    failure_note: String,
    failure_debug: String,
}

fn run_claude_help_with_timeout(
    claude_binary: &Path,
    path: &[String],
    timeout: Duration,
) -> Result<HelpRun, String> {
    let mut cmd = Command::new(claude_binary);
    cmd.args(path);
    cmd.arg("--help");
    cmd.env("NO_COLOR", "1");
    cmd.env("CLICOLOR", "0");
    cmd.env("TERM", "dumb");
    cmd.env("DISABLE_AUTOUPDATER", "1");
    cmd.env("CI", "1");
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| e.to_string())?;

    // Avoid indefinite hangs on any single `--help` invocation.
    match child.wait_timeout(timeout).map_err(|e| e.to_string())? {
        Some(_) => {}
        None => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!(
                "timeout after {}ms: {}",
                timeout.as_millis(),
                util::command_string(&cmd)
            ));
        }
    }

    let output = child.wait_with_output().map_err(|e| e.to_string())?;
    let text = util::normalize_text(&output.stdout, &output.stderr);
    let cmd_string = util::command_string(&cmd);
    let exit_code = output.status.code().unwrap_or(-1);
    let failure_note = format!("help probe failed: {cmd_string} (exit {exit_code})");
    let failure_debug = util::command_failed_message(&cmd, &output);

    Ok(HelpRun {
        status: output.status,
        text,
        failure_note,
        failure_debug,
    })
}

fn looks_like_help(s: &str) -> bool {
    // Heuristic: accept common help markers even when exit status is non-zero.
    let lower = s.to_ascii_lowercase();
    lower.contains("usage:")
        || lower.contains("commands:")
        || lower.contains("subcommands:")
        || lower.contains("options:")
        || lower.contains("flags:")
}
