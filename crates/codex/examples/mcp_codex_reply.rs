//! Continue an existing Codex MCP session via the `codex-reply` tool.
//!
//! Requirements:
//! - `CODEX_BINARY` (optional) to point at the Codex CLI.
//! - `CODEX_HOME` (optional) for app-scoped state.
//! - `CODEX_CONVERSATION_ID` must be set (or pass one as the first argument).
//! - Use `--sample` to see mocked notifications without spawning Codex.
//!
//! Example:
//! ```bash
//! CODEX_CONVERSATION_ID=abc123 \
//!   cargo run -p codex --example mcp_codex_reply -- "Continue the prior run"
//! ```

use std::{env, error::Error, path::Path, path::PathBuf};

use serde_json::{json, Value};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::Command,
    time::{self, Duration},
};

const SAMPLE_NOTIFICATIONS: &[&str] = &[
    r#"{"jsonrpc":"2.0","method":"codex/event","params":{"type":"approval_required","kind":"apply","message":"Apply staged diff?","thread_id":"demo-thread","turn_id":"turn-2"}}"#,
    r#"{"jsonrpc":"2.0","method":"codex/event","params":{"type":"task_complete","message":"Conversation resumed","turn_id":"turn-2","thread_id":"demo-thread"}}"#,
];

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut args: Vec<String> = env::args().skip(1).collect();
    let use_sample = take_flag(&mut args, "--sample");
    let conversation_id_arg = if !args.is_empty() {
        Some(args.remove(0))
    } else {
        None
    };
    let conversation_id = conversation_id_arg.or_else(|| env::var("CODEX_CONVERSATION_ID").ok());
    let prompt = if args.is_empty() {
        "Resume the last Codex turn".to_string()
    } else {
        args.join(" ")
    };

    if conversation_id.is_none() {
        eprintln!("Set CODEX_CONVERSATION_ID or pass a conversation id as the first argument.");
        print_sample_flow();
        return Ok(());
    }

    if use_sample {
        print_sample_flow();
        return Ok(());
    }

    let binary = resolve_binary();
    if !binary_exists(&binary) {
        eprintln!(
            "codex binary not found at {}. Set CODEX_BINARY or use --sample.",
            binary.display()
        );
        print_sample_flow();
        return Ok(());
    }

    demo_codex_reply(&binary, conversation_id.as_deref().unwrap(), &prompt).await?;
    Ok(())
}

async fn demo_codex_reply(
    binary: &Path,
    conversation_id: &str,
    prompt: &str,
) -> Result<(), Box<dyn Error>> {
    println!(
        "Starting `codex mcp-server` then calling codex-reply for conversation {conversation_id}"
    );

    let mut command = Command::new(binary);
    command
        .arg("mcp-server")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit())
        .kill_on_drop(true);

    let mut child = command.spawn()?;
    let mut stdin = child.stdin.take().ok_or("stdin unavailable")?;
    let mut stdout = BufReader::new(child.stdout.take().ok_or("stdout unavailable")?).lines();

    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/codex-reply",
        "params": {
            "conversationId": conversation_id,
            "prompt": prompt,
            "sandbox": true,
            "approval": "auto"
        }
    });

    stdin.write_all(request.to_string().as_bytes()).await?;
    stdin.write_all(b"\n").await?;
    stdin.flush().await?;

    let mut seen = 0;
    while seen < 4 {
        let next = time::timeout(Duration::from_secs(5), stdout.next_line()).await;
        match next {
            Ok(Ok(Some(line))) => {
                seen += 1;
                println!("[notification] {line}");
            }
            Ok(Ok(None)) => break,
            Ok(Err(error)) => {
                eprintln!("Failed to read MCP output: {error}");
                break;
            }
            Err(_) => {
                eprintln!("Timed out waiting for MCP notification");
                break;
            }
        }
    }

    let _ = child.kill().await;
    Ok(())
}

fn print_sample_flow() {
    println!("Sample codex-reply notifications:");
    for line in SAMPLE_NOTIFICATIONS {
        match serde_json::from_str::<Value>(line) {
            Ok(value) => println!("{}", serde_json::to_string_pretty(&value).unwrap()),
            Err(_) => println!("{line}"),
        }
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
