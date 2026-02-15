//! Small helper for examples that need a real Claude Code CLI binary.
//!
//! Conventions:
//! - Examples default to using the caller's existing config/auth state.
//! - Set `CLAUDE_EXAMPLE_ISOLATED_HOME=1` to run with an isolated home under `target/`.
//! - Set `CLAUDE_EXAMPLE_LIVE=1` to enable examples that may require network/auth.

#![allow(dead_code)]

use std::{
    env,
    error::Error,
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use claude_code::{ClaudeClient, ClaudeClientBuilder};

pub const ENV_BINARY: &str = "CLAUDE_BINARY";
pub const ENV_EXAMPLE_ISOLATED_HOME: &str = "CLAUDE_EXAMPLE_ISOLATED_HOME";
pub const ENV_EXAMPLE_LIVE: &str = "CLAUDE_EXAMPLE_LIVE";
pub const ENV_EXAMPLE_ALLOW_MUTATION: &str = "CLAUDE_EXAMPLE_ALLOW_MUTATION";

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/claude_code has repo root parent")
        .parent()
        .expect("repo root exists")
        .to_path_buf()
}

fn is_truthy(var: &str) -> bool {
    matches!(
        env::var(var).ok().as_deref(),
        Some("1") | Some("true") | Some("yes")
    )
}

pub fn live_enabled() -> bool {
    is_truthy(ENV_EXAMPLE_LIVE)
}

pub fn mutation_enabled() -> bool {
    is_truthy(ENV_EXAMPLE_ALLOW_MUTATION)
}

pub fn resolve_binary() -> PathBuf {
    // Prefer explicit env override.
    if let Some(binary) = env::var_os(ENV_BINARY) {
        return PathBuf::from(binary);
    }

    // Prefer a repo-local pinned binary when present (common in CI).
    let root = repo_root();
    let candidates = [
        root.join("claude-linux-x64"),
        root.join("claude-darwin-arm64"),
        root.join("claude-win32-x64.exe"),
    ];
    for c in candidates {
        if c.is_file() {
            return c;
        }
    }

    PathBuf::from("claude")
}

pub fn default_client() -> ClaudeClient {
    default_client_with_mirroring(false, false)
}

pub fn default_client_with_mirroring(mirror_stdout: bool, mirror_stderr: bool) -> ClaudeClient {
    default_builder_with_mirroring(mirror_stdout, mirror_stderr).build()
}

pub fn default_builder_with_mirroring(
    mirror_stdout: bool,
    mirror_stderr: bool,
) -> ClaudeClientBuilder {
    ClaudeClient::builder()
        .binary(resolve_binary())
        .mirror_stdout(mirror_stdout)
        .mirror_stderr(mirror_stderr)
}

pub fn isolated_home_root(example_name: &str) -> PathBuf {
    let target = repo_root().join("target");
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    target.join(format!(
        "claude-example-home-{}-{}-{}",
        example_name,
        std::process::id(),
        now
    ))
}

pub fn maybe_isolated_client(example_name: &str) -> Result<ClaudeClient, Box<dyn Error>> {
    maybe_isolated_client_with_mirroring(example_name, false, false)
}

pub fn maybe_isolated_client_with_mirroring(
    example_name: &str,
    mirror_stdout: bool,
    mirror_stderr: bool,
) -> Result<ClaudeClient, Box<dyn Error>> {
    Ok(maybe_isolated_builder_with_mirroring(example_name, mirror_stdout, mirror_stderr)?.build())
}

pub fn maybe_isolated_builder_with_mirroring(
    example_name: &str,
    mirror_stdout: bool,
    mirror_stderr: bool,
) -> Result<ClaudeClientBuilder, Box<dyn Error>> {
    if !is_truthy(ENV_EXAMPLE_ISOLATED_HOME) {
        return Ok(default_builder_with_mirroring(mirror_stdout, mirror_stderr));
    }

    let home = isolated_home_root(example_name);
    let xdg_config = home.join("xdg-config");
    let xdg_data = home.join("xdg-data");
    let xdg_cache = home.join("xdg-cache");

    fs::create_dir_all(&xdg_config)?;
    fs::create_dir_all(&xdg_data)?;
    fs::create_dir_all(&xdg_cache)?;

    Ok(ClaudeClient::builder()
        .binary(resolve_binary())
        .env("HOME", home.to_string_lossy())
        .env("XDG_CONFIG_HOME", xdg_config.to_string_lossy())
        .env("XDG_DATA_HOME", xdg_data.to_string_lossy())
        .env("XDG_CACHE_HOME", xdg_cache.to_string_lossy())
        .mirror_stdout(mirror_stdout)
        .mirror_stderr(mirror_stderr))
}

pub fn require_live(example_name: &str) -> Result<(), Box<dyn Error>> {
    if live_enabled() {
        return Ok(());
    }
    eprintln!(
        "skipped {example_name}: set {ENV_EXAMPLE_LIVE}=1 to run examples that may require network/auth"
    );
    Ok(())
}

pub fn require_mutation(example_name: &str) -> Result<(), Box<dyn Error>> {
    if mutation_enabled() {
        return Ok(());
    }
    eprintln!(
        "skipped {example_name}: set {ENV_EXAMPLE_ALLOW_MUTATION}=1 to allow examples that may mutate local state"
    );
    Ok(())
}
