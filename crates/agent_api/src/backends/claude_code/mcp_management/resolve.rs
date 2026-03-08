use std::{collections::BTreeMap, env, path::PathBuf, time::Duration};

use claude_code::ClaudeHomeLayout;

use crate::mcp::AgentWrapperMcpCommandContext;

use super::{
    CLAUDE_BINARY_ENV, CLAUDE_HOME_ENV, DISABLE_AUTOUPDATER_ENV, HOME_ENV, XDG_CACHE_HOME_ENV,
    XDG_CONFIG_HOME_ENV, XDG_DATA_HOME_ENV,
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
) -> ResolvedClaudeMcpCommand {
    resolve_claude_mcp_command_with_env(config, context, env::var(CLAUDE_BINARY_ENV).ok())
}

pub(super) fn resolve_claude_mcp_command_with_env(
    config: &super::super::ClaudeCodeBackendConfig,
    context: &AgentWrapperMcpCommandContext,
    claude_binary_env: Option<String>,
) -> ResolvedClaudeMcpCommand {
    let binary_path = resolve_claude_binary_path(config.binary.as_ref(), claude_binary_env);
    let mut env = config.env.clone();
    env.entry(DISABLE_AUTOUPDATER_ENV.to_string())
        .or_insert_with(|| "1".to_string());

    let materialize_claude_home = config
        .claude_home
        .as_ref()
        .map(|path| ClaudeHomeLayout::new(path.clone()));
    if let Some(layout) = materialize_claude_home.as_ref() {
        inject_claude_home_env(&mut env, layout);
    }

    env.extend(context.env.clone());

    ResolvedClaudeMcpCommand {
        binary_path,
        working_dir: context
            .working_dir
            .clone()
            .or_else(|| config.default_working_dir.clone()),
        timeout: context.timeout.or(config.default_timeout),
        env,
        materialize_claude_home,
    }
}

pub(super) fn resolve_claude_binary_path(
    config_binary: Option<&PathBuf>,
    claude_binary_env: Option<String>,
) -> PathBuf {
    if let Some(binary) = config_binary {
        return binary.clone();
    }
    if let Some(binary) = claude_binary_env {
        if !binary.trim().is_empty() {
            return PathBuf::from(binary);
        }
    }
    PathBuf::from("claude")
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
