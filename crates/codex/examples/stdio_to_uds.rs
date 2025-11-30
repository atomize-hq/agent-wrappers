#[cfg(unix)]
use std::{
    io::{Read, Write},
    os::unix::net::UnixListener,
    thread,
};

use codex::{CodexClient, StdioToUdsRequest};
#[cfg(unix)]
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};

#[cfg(unix)]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Maps to: codex stdio-to-uds <SOCKET_PATH>
    // Bridges stdin/stdout to a Unix domain socket; this spins up a local echo server for the demo.
    let socket_dir = tempfile::tempdir()?;
    let socket_path = socket_dir.path().join("echo.sock");

    let listener = UnixListener::bind(&socket_path)?;
    let server = thread::spawn(move || {
        if let Ok((mut stream, _addr)) = listener.accept() {
            let mut buf = [0u8; 1024];
            if let Ok(read) = stream.read(&mut buf) {
                let _ = stream.write_all(&buf[..read]);
            }
        }
    });

    let mut bridge = CodexClient::builder()
        .mirror_stdout(false)
        .quiet(true)
        .build()
        .stdio_to_uds(StdioToUdsRequest::new(&socket_path))?;

    let mut stdin = bridge.stdin.take().expect("stdio bridge stdin missing");
    let stdout = bridge.stdout.take().expect("stdio bridge stdout missing");

    stdin.write_all(b"ping from codex stdio-to-uds\n").await?;
    stdin.shutdown().await?;

    let mut echoed = String::new();
    BufReader::new(stdout).read_to_string(&mut echoed).await?;
    println!("echoed from UDS:\n{echoed}");

    let _ = bridge.wait().await?;
    let _ = server.join();
    Ok(())
}

#[cfg(not(unix))]
fn main() {
    eprintln!("stdio-to-uds example is only available on Unix platforms");
}
