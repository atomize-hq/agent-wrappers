use std::future::Future;

use tokio::sync::OnceCell;

pub(super) fn parse_bool(
    value: &serde_json::Value,
    key: &str,
) -> Result<bool, crate::AgentWrapperError> {
    value
        .as_bool()
        .ok_or_else(|| crate::AgentWrapperError::InvalidRequest {
            message: format!("{key} must be a boolean"),
        })
}

fn is_session_not_found_signal(text: &str) -> bool {
    let text = text.to_ascii_lowercase();

    (text.contains("not found")
        && (text.contains("session") || text.contains("thread") || text.contains("conversation")))
        || text.contains("no session")
        || text.contains("unknown session")
        || text.contains("no thread")
        || text.contains("unknown thread")
        || text.contains("no conversation")
        || text.contains("unknown conversation")
}

pub(super) fn json_contains_not_found_signal(raw: &serde_json::Value) -> bool {
    const MAX_DEPTH: usize = 6;
    const MAX_STRING_LEAVES: usize = 64;

    fn visit(raw: &serde_json::Value, depth: usize, strings_seen: &mut usize) -> bool {
        if depth > MAX_DEPTH || *strings_seen >= MAX_STRING_LEAVES {
            return false;
        }

        match raw {
            serde_json::Value::String(s) => {
                *strings_seen += 1;
                is_session_not_found_signal(s)
            }
            serde_json::Value::Array(arr) => arr
                .iter()
                .any(|child| visit(child, depth + 1, strings_seen)),
            serde_json::Value::Object(obj) => obj
                .values()
                .any(|child| visit(child, depth + 1, strings_seen)),
            _ => false,
        }
    }

    let mut strings_seen = 0usize;
    visit(raw, 0, &mut strings_seen)
}

pub(super) fn generic_non_zero_exit_message(status: &std::process::ExitStatus) -> String {
    match status.code() {
        Some(code) => format!("claude_code exited non-zero: code={code} (output redacted)"),
        None => "claude_code exited non-zero (output redacted)".to_string(),
    }
}

fn help_supports_allow_flag(stdout: &str) -> bool {
    stdout.contains("--allow-dangerously-skip-permissions")
}

pub(super) async fn preflight_allow_flag_support<F, Fut>(
    cell: &OnceCell<bool>,
    help: F,
) -> Result<bool, String>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<claude_code::CommandOutput, claude_code::ClaudeCodeError>>,
{
    let supported = cell
        .get_or_try_init(move || async move {
            let output = match help().await {
                Ok(output) => output,
                Err(err) => return Err(format!("claude --help preflight failed: {err}")),
            };

            if !output.status.success() {
                let message = match output.status.code() {
                    Some(code) => {
                        format!("claude --help exited non-zero: code={code} (output redacted)")
                    }
                    None => "claude --help exited non-zero (output redacted)".to_string(),
                };
                return Err(message);
            }

            let stdout = String::from_utf8_lossy(&output.stdout);
            Ok(help_supports_allow_flag(&stdout))
        })
        .await?;

    Ok(*supported)
}
