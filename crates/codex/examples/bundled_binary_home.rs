//! Run Codex using a bundled binary and an app-scoped `CODEX_HOME`.
//! This keeps user installations untouched while shipping Codex alongside
//! your application assets.
//!
//! Example layout (relative to the compiled binary):
//! ```text
//! target/release/your-app.exe
//! target/release/bin/codex(.exe)
//! target/release/data/codex/
//! ```
//!
//! Usage:
//! ```powershell
//! cargo run -p codex --example bundled_binary_home -- "Health check prompt"
//! ```

use codex::CodexClient;
use std::{env, error::Error, path::PathBuf};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let prompt = collect_prompt()?;
    let codex_binary = bundled_codex_path();
    let codex_home = bundled_codex_home();
    println!("Using binary: {}", codex_binary.display());
    println!("Using CODEX_HOME: {}", codex_home.display());

    let client = CodexClient::builder()
        .binary(&codex_binary)
        .codex_home(&codex_home)
        .build();

    let response = client.send_prompt(&prompt).await?;
    println!("{response}");
    Ok(())
}

fn bundled_codex_path() -> PathBuf {
    let root = app_root();
    let binary_name = if cfg!(windows) { "codex.exe" } else { "codex" };
    root.join("bin").join(binary_name)
}

fn bundled_codex_home() -> PathBuf {
    app_root().join("data").join("codex")
}

fn app_root() -> PathBuf {
    env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(PathBuf::from))
        .unwrap_or_else(|| env::current_dir().expect("failed to read current working dir"))
}

fn collect_prompt() -> Result<String, Box<dyn Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        return Err("Provide a prompt".into());
    }
    Ok(args.join(" "))
}
