use std::time::Duration;

use codex::{CodexClient, ResponsesApiProxyRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Maps to: echo "<API_KEY>" | codex responses-api-proxy [--port <PORT>] [--server-info <FILE>] [--http-shutdown] [--upstream-url <URL>]
    // Reads an API key from stdin, starts the proxy, and optionally parses the server-info JSON.
    let api_key = std::env::var("OPENAI_API_KEY")
        .or_else(|_| std::env::var("CODEX_API_KEY"))
        .unwrap_or_else(|_| "sk-placeholder".to_string());

    let server_info_dir = tempfile::tempdir()?;
    let server_info_path = server_info_dir.path().join("responses-api-proxy.json");

    let client = CodexClient::builder()
        .mirror_stdout(false)
        .quiet(true)
        .build();

    let mut proxy = client
        .start_responses_api_proxy(
            ResponsesApiProxyRequest::new(api_key)
                .http_shutdown(true)
                .server_info(&server_info_path),
        )
        .await?;

    if let Some(info) = proxy.read_server_info().await? {
        println!(
            "responses-api-proxy listening on 127.0.0.1:{} (pid {})",
            info.port, info.pid
        );
    } else {
        println!("responses-api-proxy started (no server-info file was written)");
    }

    if let Some(pid) = proxy.child.id() {
        println!("proxy pid: {pid}");
    }

    tokio::time::sleep(Duration::from_millis(500)).await;
    let _ = proxy.child.start_kill();
    let _ = proxy.child.wait().await;
    Ok(())
}
