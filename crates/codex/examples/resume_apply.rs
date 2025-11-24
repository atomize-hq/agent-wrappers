//! Resume a Codex session and apply the latest diff.
//!
//! The current CLI (0.61.0) streams resume events via `codex exec --json resume --last`
//! but does not expose JSON diff/apply helpers. This example streams the resume events
//! and will attempt a live `codex apply <task-id>` when `--apply-task <id>` is provided;
//! otherwise it replays the bundled diff/apply fixtures. Pass `--sample` to replay all
//! fixtures when you do not have a Codex binary.
//!
//! Examples:
//! ```bash
//! cargo run -p codex --example resume_apply -- --sample
//! CODEX_CONVERSATION_ID=abc123 cargo run -p codex --example resume_apply
//! CODEX_CONVERSATION_ID=abc123 cargo run -p codex --example resume_apply -- --apply-task task-123
//! cargo run -p codex --example resume_apply -- --resume-id abc123 --no-apply
//! ```

use std::{
    env,
    error::Error,
    path::{Path, PathBuf},
    process::Stdio,
};

#[path = "support/fixtures.rs"]
mod fixtures;

use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut args: Vec<String> = env::args().skip(1).collect();
    let use_sample = take_flag(&mut args, "--sample");
    let skip_apply = take_flag(&mut args, "--no-apply");
    let resume_id =
        take_value(&mut args, "--resume-id").or_else(|| env::var("CODEX_CONVERSATION_ID").ok());
    let apply_task = take_value(&mut args, "--apply-task");

    let binary = resolve_binary();
    if use_sample || !binary_exists(&binary) {
        eprintln!(
            "Using sample resume/apply payloads from {}, {}, and {}; set CODEX_BINARY and drop --sample to hit the real binary.",
            fixtures::RESUME_FIXTURE_PATH,
            fixtures::DIFF_FIXTURE_PATH,
            fixtures::APPLY_FIXTURE_PATH
        );
        replay_samples(!skip_apply);
        return Ok(());
    }

    stream_resume(&binary, resume_id.as_deref()).await?;

    if skip_apply {
        return Ok(());
    }

    if let Some(task_id) = apply_task.as_deref() {
        if let Err(err) = run_apply(&binary, task_id).await {
            eprintln!(
                "codex apply failed; showing sample diff/apply payloads from {} and {}: {err}",
                fixtures::DIFF_FIXTURE_PATH,
                fixtures::APPLY_FIXTURE_PATH
            );
            print_diff_apply_samples(true);
        }
    } else {
        eprintln!(
            "codex apply now expects a task id; showing sample diff/apply payloads from {} and {}.",
            fixtures::DIFF_FIXTURE_PATH,
            fixtures::APPLY_FIXTURE_PATH
        );
        print_diff_apply_samples(true);
    }

    Ok(())
}

async fn stream_resume(binary: &Path, resume_id: Option<&str>) -> Result<(), Box<dyn Error>> {
    println!("--- resume stream ---");

    let mut command = Command::new(binary);
    command
        .args([
            "exec",
            "--json",
            "--skip-git-repo-check",
            "--sandbox",
            "read-only",
            "--color",
            "never",
        ])
        .arg("resume");

    if let Some(id) = resume_id {
        command.arg(id);
    } else {
        command.arg("--last");
    }

    command
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .kill_on_drop(true);

    let mut child = command.spawn()?;
    let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
    while let Some(line) = lines.next_line().await? {
        println!("{line}");
    }

    let status = child.wait().await?;
    if !status.success() {
        return Err(format!("codex exec resume exited with {status}").into());
    }

    Ok(())
}

async fn run_apply(binary: &Path, task_id: &str) -> Result<(), Box<dyn Error>> {
    println!("--- apply (task {task_id}) ---");
    let output = Command::new(binary).args(["apply", task_id]).output().await?;

    println!("exit status: {}", output.status);
    if !output.stdout.is_empty() {
        println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    }
    if !output.stderr.is_empty() {
        println!("stderr: {}", String::from_utf8_lossy(&output.stderr));
    }

    if !output.status.success() {
        return Err(format!("codex apply exited with {}", output.status).into());
    }

    Ok(())
}

fn print_diff_apply_samples(include_apply: bool) {
    println!("--- diff preview (sample) ---");
    print!("{}", fixtures::sample_diff());

    if include_apply {
        println!("--- apply (sample) ---");
        println!("{}", fixtures::apply_result());
    }
}

fn replay_samples(include_apply: bool) {
    println!("--- resume stream (sample) ---");
    for line in fixtures::resume_events() {
        println!("{line}");
    }

    print_diff_apply_samples(include_apply);
}

fn resolve_binary() -> PathBuf {
    env::var_os("CODEX_BINARY")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("codex"))
}

fn binary_exists(path: &Path) -> bool {
    std::fs::metadata(path).is_ok()
}

fn take_flag(args: &mut Vec<String>, flag: &str) -> bool {
    let before = args.len();
    args.retain(|value| value != flag);
    before != args.len()
}

fn take_value(args: &mut Vec<String>, key: &str) -> Option<String> {
    let mut value = None;
    let mut i = 0;
    while i < args.len() {
        if args[i] == key {
            if i + 1 < args.len() {
                value = Some(args.remove(i + 1));
            }
            args.remove(i);
            break;
        }
        i += 1;
    }
    value
}
