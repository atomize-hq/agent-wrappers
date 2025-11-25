//! Resume a Codex session and apply the latest diff.
//!
//! This example streams events from `codex resume --json` (using `--last` or a provided
//! conversation ID) and then calls `codex diff`/`codex apply` to preview and apply the staged
//! changes. Pass `--sample` to replay bundled payloads from
//! `crates/codex/examples/fixtures/` when you do not have a Codex binary.
//!
//! Examples:
//! ```bash
//! cargo run -p codex --example resume_apply -- --sample
//! CODEX_CONVERSATION_ID=abc123 cargo run -p codex --example resume_apply
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

    let binary = resolve_binary();
    if use_sample || !binary_exists(&binary) {
        eprintln!(
            "Using sample resume/apply payloads from {} and {}; set CODEX_BINARY and drop --sample to hit the real binary.",
            fixtures::RESUME_FIXTURE_PATH,
            fixtures::APPLY_FIXTURE_PATH
        );
        replay_samples(!skip_apply);
        return Ok(());
    }

    stream_resume(&binary, resume_id.as_deref()).await?;
    if !skip_apply {
        run_diff_and_apply(&binary).await?;
    }

    Ok(())
}

async fn stream_resume(binary: &Path, resume_id: Option<&str>) -> Result<(), Box<dyn Error>> {
    println!("--- resume stream ---");

    let mut command = Command::new(binary);
    command
        .arg("resume")
        .args(["--json", "--skip-git-repo-check", "--timeout", "0"]);
    if let Some(id) = resume_id {
        command.args(["--id", id]);
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
        return Err(format!("codex resume exited with {status}").into());
    }

    Ok(())
}

async fn run_diff_and_apply(binary: &Path) -> Result<(), Box<dyn Error>> {
    println!("--- diff preview ---");
    let diff_output = Command::new(binary)
        .args(["diff", "--json", "--skip-git-repo-check"])
        .output()
        .await?;
    if !diff_output.stdout.is_empty() {
        println!("{}", String::from_utf8_lossy(&diff_output.stdout));
    }

    if !diff_output.status.success() {
        return Err(format!("codex diff exited with {}", diff_output.status).into());
    }

    println!("--- apply ---");
    let output = Command::new(binary)
        .args(["apply", "--json", "--skip-git-repo-check"])
        .output()
        .await?;

    println!("exit status: {}", output.status);
    if !output.stdout.is_empty() {
        println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    }
    if !output.stderr.is_empty() {
        println!("stderr: {}", String::from_utf8_lossy(&output.stderr));
    }

    Ok(())
}

fn replay_samples(include_apply: bool) {
    println!("--- resume stream (sample) ---");
    for line in fixtures::resume_events() {
        println!("{line}");
    }

    println!("--- diff preview (sample) ---");
    print!("{}", fixtures::sample_diff());

    if include_apply {
        println!("--- apply (sample) ---");
        println!("{}", fixtures::apply_result());
    }
}

fn resolve_binary() -> PathBuf {
    env::var_os("CODEX_BINARY")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("codex"))
}

fn binary_exists(path: &Path) -> bool {
    if path.is_absolute() || path.components().count() > 1 {
        std::fs::metadata(path).is_ok()
    } else {
        env::var_os("PATH")
            .and_then(|paths| {
                env::split_paths(&paths)
                    .map(|dir| dir.join(path))
                    .find(|candidate| std::fs::metadata(candidate).is_ok())
            })
            .is_some()
    }
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
