use std::{
    collections::BTreeMap,
    env,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

use crate::{mcp::AgentWrapperMcpCommandContext, AgentWrapperError};

use super::{backend_error, CODEX_BINARY_ENV, CODEX_HOME_ENV, PATH_ENV, PINNED_SPAWN_FAILURE};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ResolvedCodexMcpCommand {
    pub(super) binary_path: PathBuf,
    pub(super) working_dir: Option<PathBuf>,
    pub(super) timeout: Option<Duration>,
    pub(super) env: BTreeMap<String, String>,
    pub(super) materialize_codex_home: Option<PathBuf>,
}

pub(super) fn resolve_codex_mcp_command(
    config: &super::super::CodexBackendConfig,
    context: &AgentWrapperMcpCommandContext,
) -> Result<ResolvedCodexMcpCommand, AgentWrapperError> {
    let ambient_path_env = env::var_os(PATH_ENV);
    let effective_path_env = effective_path_env(config, context, ambient_path_env.as_ref());
    let binary_path = resolve_codex_binary_path(
        config.binary.as_ref(),
        env::var_os(CODEX_BINARY_ENV),
        effective_path_env.as_deref(),
        ambient_path_env,
    )?;
    let mut env = config.env.clone();
    env.insert(
        CODEX_BINARY_ENV.to_string(),
        binary_path.to_string_lossy().into_owned(),
    );

    if let Some(codex_home) = config.codex_home.as_ref() {
        env.insert(
            CODEX_HOME_ENV.to_string(),
            codex_home.to_string_lossy().into_owned(),
        );
    }

    env.extend(context.env.clone());
    if let Some(path_env) = effective_path_env {
        env.entry(PATH_ENV.to_string()).or_insert(path_env);
    }

    let materialize_codex_home = config.codex_home.clone().filter(|codex_home| {
        env.get(CODEX_HOME_ENV)
            .is_some_and(|value| value == &codex_home.to_string_lossy())
    });

    Ok(ResolvedCodexMcpCommand {
        binary_path,
        working_dir: context
            .working_dir
            .clone()
            .or_else(|| config.default_working_dir.clone()),
        timeout: context.timeout.or(config.default_timeout),
        env,
        materialize_codex_home,
    })
}

fn default_codex_binary_path() -> PathBuf {
    env::var_os(CODEX_BINARY_ENV)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("codex"))
}

fn effective_path_env(
    config: &super::super::CodexBackendConfig,
    context: &AgentWrapperMcpCommandContext,
    ambient_path_env: Option<&OsString>,
) -> Option<String> {
    context
        .env
        .get(PATH_ENV)
        .cloned()
        .or_else(|| config.env.get(PATH_ENV).cloned())
        .or_else(|| ambient_path_env.map(|value| value.to_string_lossy().into_owned()))
}

pub(super) fn resolve_codex_binary_path(
    config_binary: Option<&PathBuf>,
    ambient_codex_binary: Option<OsString>,
    effective_path_env: Option<&str>,
    ambient_path_env: Option<OsString>,
) -> Result<PathBuf, AgentWrapperError> {
    let binary_path = config_binary
        .cloned()
        .or_else(|| {
            ambient_codex_binary.and_then(|value| {
                if value.is_empty() {
                    None
                } else {
                    Some(PathBuf::from(value))
                }
            })
        })
        .unwrap_or_else(default_codex_binary_path);

    resolve_path_for_spawn(binary_path, effective_path_env, ambient_path_env)
        .ok_or_else(|| backend_error(PINNED_SPAWN_FAILURE))
}

fn resolve_path_for_spawn(
    binary_path: PathBuf,
    effective_path_env: Option<&str>,
    ambient_path_env: Option<OsString>,
) -> Option<PathBuf> {
    if is_path_qualified(&binary_path) {
        return Some(binary_path);
    }

    if let Some(path_env) = effective_path_env {
        return find_binary_on_path(&binary_path, Some(OsString::from(path_env)));
    }

    find_binary_on_path(&binary_path, ambient_path_env)
}

fn is_path_qualified(path: &Path) -> bool {
    path.is_absolute()
        || path
            .parent()
            .is_some_and(|parent| !parent.as_os_str().is_empty())
}

fn find_binary_on_path(binary_name: &Path, path_env: Option<OsString>) -> Option<PathBuf> {
    let path_env = path_env?;
    env::split_paths(&path_env)
        .find_map(|directory| candidate_binary_path(&directory, binary_name))
        .map(|candidate| fs::canonicalize(&candidate).unwrap_or(candidate))
}

fn candidate_binary_path(directory: &Path, binary_name: &Path) -> Option<PathBuf> {
    let candidate = directory.join(binary_name);
    if candidate.is_file() {
        return Some(candidate);
    }

    #[cfg(windows)]
    {
        if candidate.extension().is_some() {
            return None;
        }

        let pathext =
            env::var_os("PATHEXT").unwrap_or_else(|| OsString::from(".COM;.EXE;.BAT;.CMD"));
        for extension in pathext.to_string_lossy().split(';') {
            let extension = extension.trim();
            if extension.is_empty() {
                continue;
            }

            let suffixed = candidate.with_extension(extension.trim_start_matches('.'));
            if suffixed.is_file() {
                return Some(suffixed);
            }
        }
    }

    None
}
