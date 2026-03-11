use std::{
    collections::BTreeMap,
    env,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

use claude_code::ClaudeHomeLayout;

use crate::{mcp::AgentWrapperMcpCommandContext, AgentWrapperError};

use super::{
    CLAUDE_BINARY_ENV, CLAUDE_HOME_ENV, DISABLE_AUTOUPDATER_ENV, HOME_ENV, PATH_ENV,
    XDG_CACHE_HOME_ENV, XDG_CONFIG_HOME_ENV, XDG_DATA_HOME_ENV,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ResolvedClaudeMcpCommand {
    pub(super) binary_path: PathBuf,
    pub(super) working_dir: Option<PathBuf>,
    pub(super) timeout: Option<Duration>,
    pub(super) env: BTreeMap<String, String>,
    pub(super) materialize_claude_home: Option<ClaudeHomeLayout>,
}

pub(super) fn resolve_claude_mcp_command(
    config: &super::super::ClaudeCodeBackendConfig,
    context: &AgentWrapperMcpCommandContext,
) -> Result<ResolvedClaudeMcpCommand, AgentWrapperError> {
    resolve_claude_mcp_command_with_env(
        config,
        context,
        env::var_os(CLAUDE_BINARY_ENV),
        env::var_os(CLAUDE_HOME_ENV).map(PathBuf::from),
    )
}

pub(super) fn resolve_claude_mcp_command_with_env(
    config: &super::super::ClaudeCodeBackendConfig,
    context: &AgentWrapperMcpCommandContext,
    claude_binary_env: Option<OsString>,
    claude_home_env: Option<PathBuf>,
) -> Result<ResolvedClaudeMcpCommand, AgentWrapperError> {
    let ambient_path_env = env::var_os(PATH_ENV);
    let effective_path_env = effective_path_env(config, context, ambient_path_env.as_ref());
    let binary_path = resolve_claude_binary_path(
        config.binary.as_ref(),
        claude_binary_env,
        effective_path_env.as_deref(),
        ambient_path_env,
    )?;
    let mut env = config.env.clone();
    env.entry(DISABLE_AUTOUPDATER_ENV.to_string())
        .or_insert_with(|| "1".to_string());

    let claude_home_layout =
        resolve_claude_home_layout(config.claude_home.as_ref(), claude_home_env);
    if let Some(layout) = claude_home_layout.as_ref() {
        inject_claude_home_env(&mut env, layout);
    }

    env.extend(context.env.clone());
    if let Some(path_env) = effective_path_env {
        env.entry(PATH_ENV.to_string()).or_insert(path_env);
    }
    let materialize_claude_home = claude_home_layout
        .as_ref()
        .filter(|layout| effective_env_matches_claude_home_layout(&env, layout))
        .cloned();

    Ok(ResolvedClaudeMcpCommand {
        binary_path,
        working_dir: context
            .working_dir
            .clone()
            .or_else(|| config.default_working_dir.clone()),
        timeout: context.timeout.or(config.default_timeout),
        env,
        materialize_claude_home,
    })
}

fn effective_path_env(
    config: &super::super::ClaudeCodeBackendConfig,
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

pub(super) fn resolve_claude_binary_path(
    config_binary: Option<&PathBuf>,
    claude_binary_env: Option<OsString>,
    effective_path_env: Option<&str>,
    ambient_path_env: Option<OsString>,
) -> Result<PathBuf, AgentWrapperError> {
    let binary_path = config_binary
        .cloned()
        .or_else(|| {
            claude_binary_env.and_then(|value| {
                if value.is_empty() {
                    None
                } else {
                    Some(PathBuf::from(value))
                }
            })
        })
        .unwrap_or_else(|| PathBuf::from("claude"));

    resolve_path_for_spawn(binary_path, effective_path_env, ambient_path_env)
        .ok_or_else(|| super::backend_error(super::PINNED_SPAWN_FAILURE))
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

fn resolve_claude_home_layout(
    config_claude_home: Option<&PathBuf>,
    claude_home_env: Option<PathBuf>,
) -> Option<ClaudeHomeLayout> {
    config_claude_home
        .cloned()
        .or_else(|| claude_home_env.filter(|path| !path.as_os_str().is_empty()))
        .map(ClaudeHomeLayout::new)
}

fn inject_claude_home_env(env: &mut BTreeMap<String, String>, layout: &ClaudeHomeLayout) {
    let root = layout.root().to_string_lossy().into_owned();
    env.entry(CLAUDE_HOME_ENV.to_string())
        .or_insert_with(|| root.clone());
    env.entry(HOME_ENV.to_string())
        .or_insert_with(|| root.clone());
    env.entry(XDG_CONFIG_HOME_ENV.to_string())
        .or_insert_with(|| layout.xdg_config_home().to_string_lossy().into_owned());
    env.entry(XDG_DATA_HOME_ENV.to_string())
        .or_insert_with(|| layout.xdg_data_home().to_string_lossy().into_owned());
    env.entry(XDG_CACHE_HOME_ENV.to_string())
        .or_insert_with(|| layout.xdg_cache_home().to_string_lossy().into_owned());

    #[cfg(windows)]
    {
        env.entry(super::USERPROFILE_ENV.to_string())
            .or_insert_with(|| root.clone());
        env.entry(super::APPDATA_ENV.to_string())
            .or_insert_with(|| layout.appdata_dir().to_string_lossy().into_owned());
        env.entry(super::LOCALAPPDATA_ENV.to_string())
            .or_insert_with(|| layout.localappdata_dir().to_string_lossy().into_owned());
    }
}

fn effective_env_matches_claude_home_layout(
    env: &BTreeMap<String, String>,
    layout: &ClaudeHomeLayout,
) -> bool {
    let root = layout.root().to_string_lossy();
    if env.get(CLAUDE_HOME_ENV).map(String::as_str) != Some(root.as_ref()) {
        return false;
    }
    if env.get(HOME_ENV).map(String::as_str) != Some(root.as_ref()) {
        return false;
    }
    if env.get(XDG_CONFIG_HOME_ENV).map(String::as_str)
        != Some(layout.xdg_config_home().to_string_lossy().as_ref())
    {
        return false;
    }
    if env.get(XDG_DATA_HOME_ENV).map(String::as_str)
        != Some(layout.xdg_data_home().to_string_lossy().as_ref())
    {
        return false;
    }
    if env.get(XDG_CACHE_HOME_ENV).map(String::as_str)
        != Some(layout.xdg_cache_home().to_string_lossy().as_ref())
    {
        return false;
    }

    #[cfg(windows)]
    {
        if env.get(super::USERPROFILE_ENV).map(String::as_str) != Some(root.as_ref()) {
            return false;
        }
        if env.get(super::APPDATA_ENV).map(String::as_str)
            != Some(layout.appdata_dir().to_string_lossy().as_ref())
        {
            return false;
        }
        if env.get(super::LOCALAPPDATA_ENV).map(String::as_str)
            != Some(layout.localappdata_dir().to_string_lossy().as_ref())
        {
            return false;
        }
    }

    true
}
