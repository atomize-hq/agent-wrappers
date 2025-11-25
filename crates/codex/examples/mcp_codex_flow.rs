//! Minimal MCP example: start `codex mcp-server`, stream `codex/event` updates,
//! optionally cancel, and send a follow-up `codex/codex-reply`.
//! Usage:
//! `cargo run -p codex --example mcp_codex_flow -- "<prompt>" ["<follow up prompt>"]`
//! Gate with `crates/codex/examples/feature_detection.rs` if your binary might not expose MCP endpoints.
//! Environment:
//! - `CODEX_BINARY` (optional): path to the `codex` binary (defaults to `codex` in PATH).
//! - `CODEX_HOME` (optional): CODEX_HOME to pass through.
//! - `CANCEL_AFTER_MS` (optional): delay before sending `$ /cancelRequest`.

use std::{collections::BTreeMap, env, path::PathBuf, time::Duration};

use codex::mcp::{
    ClientInfo, CodexCallParams, CodexEvent, CodexMcpServer, CodexReplyParams, EventStream,
    McpError, StdioServerConfig,
};
use tokio::time;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let prompt = args
        .next()
        .expect("usage: mcp_codex_flow <prompt> [follow-up prompt]");
    let follow_up = args.next();

    let config = config_from_env();
    let client = ClientInfo {
        name: "codex-mcp-example".into(),
        version: env!("CARGO_PKG_VERSION").into(),
    };
    let server = CodexMcpServer::start(config, client)
        .await
        .map_err(boxed_err)?;

    let cancel_after_ms = env::var("CANCEL_AFTER_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok());

    let mut handle = server
        .codex(CodexCallParams {
            prompt: prompt.clone(),
            model: None,
            cwd: None,
            sandbox: None,
            approval_policy: None,
            config: BTreeMap::new(),
        })
        .await
        .map_err(boxed_err)?;

    if let Some(delay) = cancel_after_ms {
        time::sleep(Duration::from_millis(delay)).await;
        let _ = server.cancel(handle.request_id);
    }

    let conversation_id = stream_codex_events("codex/codex", &mut handle.events).await;
    let first_response = match handle.response.await {
        Ok(resp) => resp,
        Err(err) => return Err(boxed_err(err)),
    };
    match first_response {
        Ok(resp) => {
            println!("codex response: {}", resp.output);
            println!("conversation: {}", resp.conversation_id);
        }
        Err(McpError::Cancelled) => eprintln!("codex call cancelled"),
        Err(other) => return Err(boxed_err(other)),
    }

    if let (Some(follow_up_prompt), Some(conversation_id)) = (follow_up, conversation_id) {
        let mut follow_up = server
            .codex_reply(CodexReplyParams {
                conversation_id: conversation_id.clone(),
                prompt: follow_up_prompt,
                model: None,
                cwd: None,
                sandbox: None,
                approval_policy: None,
                config: BTreeMap::new(),
            })
            .await
            .map_err(boxed_err)?;

        let _ = stream_codex_events("codex/codex-reply", &mut follow_up.events).await;
        let follow_up_response = match follow_up.response.await {
            Ok(resp) => resp,
            Err(err) => return Err(boxed_err(err)),
        };
        match follow_up_response {
            Ok(resp) => println!("codex-reply {} => {}", resp.conversation_id, resp.output),
            Err(err) => eprintln!("codex-reply failed: {err}"),
        }
    }

    let _ = server.shutdown().await;
    Ok(())
}

fn boxed_err<E: std::error::Error + 'static>(err: E) -> Box<dyn std::error::Error> {
    Box::new(err)
}

fn config_from_env() -> StdioServerConfig {
    let binary = env::var_os("CODEX_BINARY")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("codex"));
    let code_home = env::var_os("CODEX_HOME").map(PathBuf::from);

    StdioServerConfig {
        binary,
        code_home,
        current_dir: None,
        env: Vec::new(),
        mirror_stdio: true,
        startup_timeout: Duration::from_secs(10),
    }
}

async fn stream_codex_events(label: &str, events: &mut EventStream<CodexEvent>) -> Option<String> {
    let mut conversation_id = None;
    while let Some(event) = events.recv().await {
        match &event {
            CodexEvent::TaskComplete {
                conversation_id: conv,
                result,
            } => {
                println!("[{label}] task_complete {conv}: {result}");
                conversation_id = Some(conv.clone());
                break;
            }
            CodexEvent::ApprovalRequired(req) => {
                println!("[{label}] approval {:?}: {:?}", req.approval_id, req.kind);
            }
            CodexEvent::Cancelled {
                conversation_id: conv,
                reason,
            } => {
                println!("[{label}] cancelled {:?}: {:?}", conv, reason);
                conversation_id = conv.clone();
                break;
            }
            CodexEvent::Error { message, data } => {
                println!("[{label}] error {message} {data:?}");
            }
            CodexEvent::Raw { method, params } => {
                println!("[{label}] raw {method}: {params}");
            }
        }
    }

    conversation_id
}
