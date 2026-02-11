//! Demonstrates `codex debug ...` surfaces via the wrapper.
//!
//! Usage:
//! - `cargo run -p codex --example debug_cmd -- help`
//! - `cargo run -p codex --example debug_cmd -- help app-server`
//! - `cargo run -p codex --example debug_cmd -- app-server`
//! - `cargo run -p codex --example debug_cmd -- app-server help`
//! - `cargo run -p codex --example debug_cmd -- app-server send-message-v2 "hello"`
//!
//! Environment:
//! - `CODEX_BINARY` (optional): path to the `codex` CLI binary (defaults to `codex` in PATH).
//! - `CODEX_HOME` (optional): Codex home directory for config/auth.

use codex::{
    DebugAppServerHelpRequest, DebugAppServerRequest, DebugAppServerSendMessageV2Request,
    DebugHelpRequest,
};

#[path = "support/real_cli.rs"]
mod real_cli;

fn usage() {
    eprintln!("usage: debug_cmd <help|app-server ...>");
    eprintln!("examples:");
    eprintln!("  debug_cmd help");
    eprintln!("  debug_cmd help app-server");
    eprintln!("  debug_cmd app-server");
    eprintln!("  debug_cmd app-server help");
    eprintln!("  debug_cmd app-server send-message-v2 \"hello\"");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        usage();
        return Ok(());
    }

    let client = real_cli::default_client();

    match args[0].to_ascii_lowercase().as_str() {
        "help" => {
            let command = args.drain(1..).collect::<Vec<_>>();
            let output = client
                .debug_help(DebugHelpRequest::new().command(command))
                .await?;
            print!("{}", output.stdout);
            if !output.stderr.is_empty() {
                eprintln!("{}", output.stderr);
            }
        }
        "app-server" => {
            if args.len() == 1 {
                let output = client
                    .debug_app_server(DebugAppServerRequest::new())
                    .await?;
                print!("{}", output.stdout);
                if !output.stderr.is_empty() {
                    eprintln!("{}", output.stderr);
                }
                return Ok(());
            }

            match args[1].to_ascii_lowercase().as_str() {
                "help" => {
                    let command = args.drain(2..).collect::<Vec<_>>();
                    let output = client
                        .debug_app_server_help(DebugAppServerHelpRequest::new().command(command))
                        .await?;
                    print!("{}", output.stdout);
                    if !output.stderr.is_empty() {
                        eprintln!("{}", output.stderr);
                    }
                }
                "send-message-v2" => {
                    if args.len() < 3 {
                        usage();
                        return Ok(());
                    }
                    let message = args.drain(2..).collect::<Vec<_>>().join(" ");
                    let output = client
                        .debug_app_server_send_message_v2(DebugAppServerSendMessageV2Request::new(
                            message,
                        ))
                        .await?;
                    print!("{}", output.stdout);
                    if !output.stderr.is_empty() {
                        eprintln!("{}", output.stderr);
                    }
                }
                _ => usage(),
            }
        }
        _ => usage(),
    }

    Ok(())
}
