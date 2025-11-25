//! Prefer a bundled Codex binary while still honoring `CODEX_BINARY`.
//!
//! Env hints:
//! - `CODEX_BINARY` — highest precedence; point at a specific binary on disk.
//! - `CODEX_BUNDLED_PATH` — packaged binary shipped with your app (fallback).
//! - Default fallback: `<crate>/bin/codex` (or `codex.exe` on Windows).
//!
//! Example:
//! ```bash
//! CODEX_BUNDLED_PATH=/opt/myapp/codex \
//!   cargo run -p codex --example bundled_binary -- "Quick health check"
//! ```

use std::{env, error::Error, path::PathBuf, time::Duration};

use codex::CodexClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let prompt = collect_prompt();
    let binary = select_binary();

    println!("Selected Codex binary: {}", binary.display());
    println!(
        "Order: CODEX_BINARY > CODEX_BUNDLED_PATH > {}",
        default_bundled_hint().display()
    );

    let client = CodexClient::builder()
        .binary(&binary)
        .timeout(Duration::from_secs(45))
        .build();

    match client.send_prompt(&prompt).await {
        Ok(response) => println!("Codex replied:\n{response}"),
        Err(error) => {
            eprintln!("Codex invocation failed: {error}");
            eprintln!(
                "Set CODEX_BINARY or CODEX_BUNDLED_PATH to the actual packaged binary before rerunning."
            );
        }
    }

    Ok(())
}

fn collect_prompt() -> String {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        "Say hello from the bundled binary".to_string()
    } else {
        args.join(" ")
    }
}

fn select_binary() -> PathBuf {
    if let Some(explicit) = env::var_os("CODEX_BINARY") {
        return PathBuf::from(explicit);
    }
    if let Some(bundled) = env::var_os("CODEX_BUNDLED_PATH") {
        return PathBuf::from(bundled);
    }
    default_bundled_hint()
}

fn default_bundled_hint() -> PathBuf {
    let binary_name = if cfg!(windows) { "codex.exe" } else { "codex" };
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("bin")
        .join(binary_name)
}
