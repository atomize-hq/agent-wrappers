//! Demonstrates the `codex cloud` task management wrapper APIs.
//!
//! Requirements:
//! - A Codex CLI binary on PATH (or set `CODEX_BINARY` via the wrapper builder in your app).
//! - Codex Cloud access and any required auth/config for your org.
//!
//! Usage:
//! - List tasks (JSON):
//!   `cargo run -p codex --example cloud_tasks -- list --env <ENV_ID> --json --limit 10`
//! - Show status:
//!   `cargo run -p codex --example cloud_tasks -- status <TASK_ID>`
//! - Diff/apply (optional attempt):
//!   `cargo run -p codex --example cloud_tasks -- diff <TASK_ID> [--attempt N]`
//!   `cargo run -p codex --example cloud_tasks -- apply <TASK_ID> [--attempt N]`
//! - Execute a task:
//!   `cargo run -p codex --example cloud_tasks -- exec --env <ENV_ID> [--attempts N] [--branch BRANCH] -- <QUERY>`

use std::{env, error::Error};

use codex::{
    CloudApplyRequest, CloudDiffRequest, CloudExecRequest, CloudListRequest, CloudStatusRequest,
    CodexClient,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        eprintln!("missing subcommand (list/status/diff/apply/exec)");
        return Ok(());
    }

    let subcommand = args.remove(0);
    let client = CodexClient::builder()
        .mirror_stdout(false)
        .quiet(true)
        .build();

    match subcommand.as_str() {
        "list" => {
            let mut env_id: Option<String> = None;
            let mut limit: Option<u32> = None;
            let mut cursor: Option<String> = None;
            let mut json = false;

            while let Some(flag) = args.first().cloned() {
                match flag.as_str() {
                    "--env" => {
                        args.remove(0);
                        env_id = args.get(0).cloned();
                        if env_id.is_some() {
                            args.remove(0);
                        }
                    }
                    "--limit" => {
                        args.remove(0);
                        let value = args.get(0).cloned();
                        if let Some(value) = value {
                            args.remove(0);
                            limit = value.parse::<u32>().ok();
                        }
                    }
                    "--cursor" => {
                        args.remove(0);
                        cursor = args.get(0).cloned();
                        if cursor.is_some() {
                            args.remove(0);
                        }
                    }
                    "--json" => {
                        args.remove(0);
                        json = true;
                    }
                    _ => break,
                }
            }

            let mut request = CloudListRequest::new().json(json);
            if let Some(env_id) = env_id {
                request = request.env_id(env_id);
            }
            if let Some(limit) = limit {
                request = request.limit(limit);
            }
            if let Some(cursor) = cursor {
                request = request.cursor(cursor);
            }

            let output = client.cloud_list(request).await?;
            if let Some(value) = output.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string())
                );
            } else {
                print!("{}", output.stdout);
            }
        }
        "status" => {
            let task_id = args
                .get(0)
                .ok_or("usage: cloud_tasks status <TASK_ID>")?
                .to_string();
            let output = client.cloud_status(CloudStatusRequest::new(task_id)).await?;
            print!("{}", output.stdout);
        }
        "diff" => {
            let task_id = args
                .get(0)
                .ok_or("usage: cloud_tasks diff <TASK_ID> [--attempt N]")?
                .to_string();
            let attempt = args
                .windows(2)
                .find(|w| w[0] == "--attempt")
                .and_then(|w| w[1].parse::<u32>().ok());
            let mut request = CloudDiffRequest::new(task_id);
            if let Some(attempt) = attempt {
                request = request.attempt(attempt);
            }
            let output = client.cloud_diff(request).await?;
            print!("{}", output.stdout);
        }
        "apply" => {
            let task_id = args
                .get(0)
                .ok_or("usage: cloud_tasks apply <TASK_ID> [--attempt N]")?
                .to_string();
            let attempt = args
                .windows(2)
                .find(|w| w[0] == "--attempt")
                .and_then(|w| w[1].parse::<u32>().ok());
            let mut request = CloudApplyRequest::new(task_id);
            if let Some(attempt) = attempt {
                request = request.attempt(attempt);
            }
            let output = client.cloud_apply(request).await?;
            print!("{}", output.stdout);
        }
        "exec" => {
            let mut env_id: Option<String> = None;
            let mut attempts: Option<u32> = None;
            let mut branch: Option<String> = None;
            let mut query: Vec<String> = Vec::new();

            while !args.is_empty() {
                match args[0].as_str() {
                    "--env" => {
                        args.remove(0);
                        env_id = args.get(0).cloned();
                        if env_id.is_some() {
                            args.remove(0);
                        }
                    }
                    "--attempts" => {
                        args.remove(0);
                        let value = args.get(0).cloned();
                        if let Some(value) = value {
                            args.remove(0);
                            attempts = value.parse::<u32>().ok();
                        }
                    }
                    "--branch" => {
                        args.remove(0);
                        branch = args.get(0).cloned();
                        if branch.is_some() {
                            args.remove(0);
                        }
                    }
                    "--" => {
                        args.remove(0);
                        query.extend(args.drain(..));
                        break;
                    }
                    other => {
                        query.push(other.to_string());
                        args.remove(0);
                    }
                }
            }

            let env_id = env_id.ok_or("missing --env <ENV_ID>")?;
            let mut request = CloudExecRequest::new(env_id);
            if let Some(attempts) = attempts {
                request = request.attempts(attempts);
            }
            if let Some(branch) = branch {
                request = request.branch(branch);
            }
            if !query.is_empty() {
                request = request.query(query.join(" "));
            }

            let output = client.cloud_exec(request).await?;
            print!("{}", output.stdout);
        }
        other => {
            eprintln!("unknown subcommand: {other} (expected list/status/diff/apply/exec)");
        }
    }

    Ok(())
}
