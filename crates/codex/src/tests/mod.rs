use super::*;
use crate::auth::parse_login_success;
use crate::builder::ResolvedCliOverrides;
use crate::defaults::{
    default_binary_path, default_rust_log_value, CODEX_BINARY_ENV, CODEX_HOME_ENV,
    DEFAULT_RUST_LOG, DEFAULT_TIMEOUT, RUST_LOG_ENV,
};
use futures_util::{pin_mut, StreamExt};
use semver::Version;
use serde_json::json;
use std::collections::HashMap;
use std::env;
use std::fs as std_fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::sync::OnceLock;
use std::time::{Duration, SystemTime};
use tokio::{
    fs,
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
};

fn env_mutex() -> &'static tokio::sync::Mutex<()> {
    static ENV_MUTEX: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
    ENV_MUTEX.get_or_init(|| tokio::sync::Mutex::new(()))
}

fn env_guard() -> tokio::sync::MutexGuard<'static, ()> {
    env_mutex().blocking_lock()
}

async fn env_guard_async() -> tokio::sync::MutexGuard<'static, ()> {
    env_mutex().lock().await
}

fn write_executable(dir: &Path, name: &str, script: &str) -> PathBuf {
    let path = dir.join(name);
    std_fs::write(&path, script).unwrap();
    let mut perms = std_fs::metadata(&path).unwrap().permissions();
    #[cfg(unix)]
    {
        perms.set_mode(0o755);
    }
    std_fs::set_permissions(&path, perms).unwrap();
    path
}

fn write_fake_codex(dir: &Path, script: &str) -> PathBuf {
    write_executable(dir, "codex", script)
}

fn write_fake_bundled_codex(dir: &Path, platform: &str, script: &str) -> PathBuf {
    write_executable(dir, bundled_binary_filename(platform), script)
}

mod auth_session;
mod builder_env_home;
mod bundled_binary;
mod capabilities;
mod cli_commands;
mod cli_overrides;
mod jsonl_stream;
mod sandbox_execpolicy;
