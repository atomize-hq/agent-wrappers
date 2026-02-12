use std::process::{Command, Output};

pub(super) fn command_failed_message(cmd: &Command, output: &Output) -> String {
    let mut s = String::new();
    s.push_str(&format!(
        "{} (exit {})",
        command_string(cmd),
        output.status.code().unwrap_or(-1)
    ));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stdout.trim().is_empty() {
        s.push_str(&format!("\nstdout:\n{stdout}"));
    }
    if !stderr.trim().is_empty() {
        s.push_str(&format!("\nstderr:\n{stderr}"));
    }
    s
}

pub(super) fn command_string(cmd: &Command) -> String {
    let program = cmd.get_program().to_string_lossy();
    let args = cmd
        .get_args()
        .map(|a| shell_escape(a.to_string_lossy().as_ref()))
        .collect::<Vec<_>>()
        .join(" ");
    if args.is_empty() {
        program.to_string()
    } else {
        format!("{program} {args}")
    }
}

fn shell_escape(s: &str) -> String {
    if s.bytes().all(
        |b| matches!(b, b'0'..=b'9' | b'a'..=b'z' | b'A'..=b'Z' | b'_' | b'-' | b'.' | b'/' | b':'),
    ) {
        s.to_string()
    } else {
        format!("{s:?}")
    }
}

pub(super) fn normalize_text(stdout: &[u8], stderr: &[u8]) -> String {
    let stdout = String::from_utf8_lossy(stdout);
    if !stdout.trim().is_empty() {
        stdout.replace("\r\n", "\n")
    } else {
        String::from_utf8_lossy(stderr).replace("\r\n", "\n")
    }
}
