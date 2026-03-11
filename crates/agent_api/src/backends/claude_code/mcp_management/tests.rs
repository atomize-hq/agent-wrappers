use std::{
    collections::BTreeMap,
    env,
    ffi::OsString,
    path::PathBuf,
    process::ExitStatus,
    sync::{Mutex, OnceLock},
    time::Duration,
};

use claude_code::ClaudeHomeLayout;
use tokio::io::{duplex, AsyncWriteExt, DuplexStream};

use super::*;
use crate::mcp::{AgentWrapperMcpAddTransport, AgentWrapperMcpCommandContext};

#[cfg(unix)]
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    time::{SystemTime, UNIX_EPOCH},
};

fn success_exit_status() -> ExitStatus {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        ExitStatus::from_raw(0)
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::ExitStatusExt;
        ExitStatus::from_raw(0)
    }
}

fn exit_status_with_code(code: i32) -> ExitStatus {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        ExitStatus::from_raw(code << 8)
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::ExitStatusExt;
        ExitStatus::from_raw(code as u32)
    }
}

fn sample_config() -> super::super::ClaudeCodeBackendConfig {
    super::super::ClaudeCodeBackendConfig {
        binary: Some(PathBuf::from("/tmp/fake-claude")),
        claude_home: Some(PathBuf::from("/tmp/claude-home")),
        default_timeout: Some(Duration::from_secs(30)),
        default_working_dir: Some(PathBuf::from("default/workdir")),
        env: BTreeMap::from([
            ("CONFIG_ONLY".to_string(), "config-only".to_string()),
            ("OVERRIDE_ME".to_string(), "config".to_string()),
        ]),
        ..Default::default()
    }
}

fn sample_config_without_home() -> super::super::ClaudeCodeBackendConfig {
    super::super::ClaudeCodeBackendConfig {
        binary: Some(PathBuf::from("/tmp/fake-claude")),
        claude_home: None,
        default_timeout: Some(Duration::from_secs(30)),
        default_working_dir: Some(PathBuf::from("default/workdir")),
        env: BTreeMap::from([
            ("CONFIG_ONLY".to_string(), "config-only".to_string()),
            ("OVERRIDE_ME".to_string(), "config".to_string()),
        ]),
        ..Default::default()
    }
}

fn sample_context() -> AgentWrapperMcpCommandContext {
    AgentWrapperMcpCommandContext {
        working_dir: Some(PathBuf::from("request/workdir")),
        timeout: Some(Duration::from_secs(5)),
        env: BTreeMap::from([
            ("OVERRIDE_ME".to_string(), "request".to_string()),
            ("REQUEST_ONLY".to_string(), "request-only".to_string()),
        ]),
    }
}

fn test_env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

struct EnvGuard {
    key: &'static str,
    previous: Option<OsString>,
}

impl EnvGuard {
    fn set(key: &'static str, value: impl Into<OsString>) -> Self {
        let previous = env::var_os(key);
        env::set_var(key, value.into());
        Self { key, previous }
    }

    fn unset(key: &'static str) -> Self {
        let previous = env::var_os(key);
        env::remove_var(key);
        Self { key, previous }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        if let Some(value) = self.previous.take() {
            env::set_var(self.key, value);
        } else {
            env::remove_var(self.key);
        }
    }
}

fn assert_backend_spawn_failure(err: AgentWrapperError) {
    match err {
        AgentWrapperError::Backend { message } => {
            assert_eq!(message, PINNED_SPAWN_FAILURE);
        }
        other => panic!("expected Backend error, got: {other:?}"),
    }
}

async fn write_all_and_close(mut writer: DuplexStream, bytes: Vec<u8>) {
    writer.write_all(&bytes).await.expect("write succeeds");
    writer.shutdown().await.expect("shutdown succeeds");
}

#[cfg(unix)]
fn temp_test_dir(label: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "agent-api-claude-mcp-{label}-{}-{unique}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).expect("temp dir should be created");
    dir
}

#[cfg(unix)]
fn write_fake_claude(dir: &std::path::Path, script: &str) -> PathBuf {
    let path = dir.join("claude");
    fs::write(&path, script).expect("script should be written");
    let mut permissions = fs::metadata(&path)
        .expect("script metadata should exist")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&path, permissions).expect("script should be executable");
    path
}

#[test]
fn claude_mcp_list_argv_is_pinned() {
    assert_eq!(
        claude_mcp_list_argv(),
        vec![OsString::from("mcp"), OsString::from("list")]
    );
}

#[test]
fn claude_mcp_get_argv_is_pinned() {
    assert_eq!(
        claude_mcp_get_argv("demo"),
        vec![
            OsString::from("mcp"),
            OsString::from("get"),
            OsString::from("demo"),
        ]
    );
}

#[test]
fn claude_mcp_remove_argv_is_pinned() {
    assert_eq!(
        claude_mcp_remove_argv("demo"),
        vec![
            OsString::from("mcp"),
            OsString::from("remove"),
            OsString::from("demo"),
        ]
    );
}

#[test]
fn claude_mcp_remove_argv_preserves_name_as_single_item() {
    assert_eq!(
        claude_mcp_remove_argv("demo server"),
        vec![
            OsString::from("mcp"),
            OsString::from("remove"),
            OsString::from("demo server"),
        ]
    );
}

#[test]
fn claude_mcp_add_argv_maps_stdio_transport_with_sorted_env_and_no_separator() {
    let transport = AgentWrapperMcpAddTransport::Stdio {
        command: vec!["node".to_string()],
        args: vec!["server.js".to_string(), "--flag".to_string()],
        env: BTreeMap::from([
            ("BETA".to_string(), "two".to_string()),
            ("ALPHA".to_string(), "one".to_string()),
        ]),
    };

    assert_eq!(
        claude_mcp_add_argv("demo", &transport).expect("stdio transport should map"),
        vec![
            OsString::from("mcp"),
            OsString::from("add"),
            OsString::from("--transport"),
            OsString::from("stdio"),
            OsString::from("--env"),
            OsString::from("ALPHA=one"),
            OsString::from("--env"),
            OsString::from("BETA=two"),
            OsString::from("demo"),
            OsString::from("node"),
            OsString::from("server.js"),
            OsString::from("--flag"),
        ]
    );
}

#[test]
fn claude_mcp_add_argv_maps_url_transport_without_bearer_env() {
    let transport = AgentWrapperMcpAddTransport::Url {
        url: "https://example.test/mcp".to_string(),
        bearer_token_env_var: None,
    };

    assert_eq!(
        claude_mcp_add_argv("demo", &transport).expect("url transport should map"),
        vec![
            OsString::from("mcp"),
            OsString::from("add"),
            OsString::from("--transport"),
            OsString::from("http"),
            OsString::from("demo"),
            OsString::from("https://example.test/mcp"),
        ]
    );
}

#[test]
fn claude_mcp_add_argv_rejects_url_transport_with_bearer_env_var() {
    let transport = AgentWrapperMcpAddTransport::Url {
        url: "https://example.test/mcp".to_string(),
        bearer_token_env_var: Some("TOKEN_ENV".to_string()),
    };

    let err = claude_mcp_add_argv("demo", &transport)
        .expect_err("url bearer token env var should be rejected");

    match err {
        AgentWrapperError::InvalidRequest { message } => {
            assert_eq!(message, PINNED_URL_BEARER_TOKEN_ENV_VAR_UNSUPPORTED);
        }
        other => panic!("expected InvalidRequest, got: {other:?}"),
    }
}

#[test]
fn resolve_claude_binary_path_prefers_config_over_env() {
    let resolved = resolve_claude_binary_path(
        Some(&PathBuf::from("/tmp/from-config")),
        Some(OsString::from("/tmp/from-env")),
        None,
        None,
    )
    .expect("config path should resolve");

    assert_eq!(resolved, PathBuf::from("/tmp/from-config"));
}

#[test]
fn resolve_claude_binary_path_uses_env_when_config_absent() {
    let resolved =
        resolve_claude_binary_path(None, Some(OsString::from("/tmp/from-env")), None, None)
            .expect("env path should resolve");

    assert_eq!(resolved, PathBuf::from("/tmp/from-env"));
}

#[test]
fn resolve_claude_binary_path_rejects_blank_env_without_a_resolvable_path() {
    let err = resolve_claude_binary_path(None, Some(OsString::from("")), None, None)
        .expect_err("blank env should fail resolution");

    assert_backend_spawn_failure(err);
}

#[cfg(unix)]
#[test]
fn resolve_claude_binary_path_uses_effective_path_env_for_unqualified_binary() {
    let temp_dir = temp_test_dir("binary-path");
    let script_path = write_fake_claude(&temp_dir, "#!/usr/bin/env bash\nexit 0\n");

    let resolved = resolve_claude_binary_path(
        None,
        Some(OsString::from("claude")),
        Some(temp_dir.to_string_lossy().as_ref()),
        None,
    )
    .expect("effective PATH should resolve claude");

    assert_eq!(
        resolved,
        fs::canonicalize(&script_path).expect("canonicalize fake claude")
    );

    fs::remove_dir_all(temp_dir).expect("temp dir should be removed");
}

#[cfg(unix)]
#[test]
fn resolve_claude_binary_path_prefers_request_path_over_config_and_ambient_path() {
    let _env_lock = test_env_lock().lock().expect("lock test env");
    let request_dir = temp_test_dir("request-path");
    let config_dir = temp_test_dir("config-path");
    let ambient_dir = temp_test_dir("ambient-path");

    let request_binary = write_fake_claude(&request_dir, "#!/usr/bin/env bash\nexit 0\n");
    let _config_binary = write_fake_claude(&config_dir, "#!/usr/bin/env bash\nexit 0\n");
    let _ambient_binary = write_fake_claude(&ambient_dir, "#!/usr/bin/env bash\nexit 0\n");
    let _ambient_path = EnvGuard::set(PATH_ENV, ambient_dir.as_os_str().to_os_string());

    let resolved = resolve_claude_binary_path(
        None,
        Some(OsString::from("claude")),
        Some(
            env::join_paths([request_dir.as_path(), config_dir.as_path()])
                .expect("join request path")
                .to_string_lossy()
                .as_ref(),
        ),
        env::var_os(PATH_ENV),
    )
    .expect("request PATH should resolve claude");

    assert_eq!(
        resolved,
        fs::canonicalize(&request_binary).expect("canonicalize request binary")
    );

    fs::remove_dir_all(request_dir).expect("request dir should be removed");
    fs::remove_dir_all(config_dir).expect("config dir should be removed");
    fs::remove_dir_all(ambient_dir).expect("ambient dir should be removed");
}

#[cfg(unix)]
#[test]
fn resolve_claude_binary_path_prefers_config_path_over_ambient_path() {
    let _env_lock = test_env_lock().lock().expect("lock test env");
    let config_dir = temp_test_dir("config-precedence");
    let ambient_dir = temp_test_dir("ambient-precedence");

    let config_binary = write_fake_claude(&config_dir, "#!/usr/bin/env bash\nexit 0\n");
    let _ambient_binary = write_fake_claude(&ambient_dir, "#!/usr/bin/env bash\nexit 0\n");
    let _ambient_path = EnvGuard::set(PATH_ENV, ambient_dir.as_os_str().to_os_string());

    let resolved = resolve_claude_binary_path(
        None,
        Some(OsString::from("claude")),
        Some(config_dir.to_string_lossy().as_ref()),
        env::var_os(PATH_ENV),
    )
    .expect("config PATH should resolve claude");

    assert_eq!(
        resolved,
        fs::canonicalize(&config_binary).expect("canonicalize config binary")
    );

    fs::remove_dir_all(config_dir).expect("config dir should be removed");
    fs::remove_dir_all(ambient_dir).expect("ambient dir should be removed");
}

#[cfg(unix)]
#[test]
fn resolve_claude_binary_path_uses_ambient_path_when_effective_path_is_absent() {
    let _env_lock = test_env_lock().lock().expect("lock test env");
    let ambient_dir = temp_test_dir("ambient-only");
    let ambient_binary = write_fake_claude(&ambient_dir, "#!/usr/bin/env bash\nexit 0\n");
    let _ambient_path = EnvGuard::set(PATH_ENV, ambient_dir.as_os_str().to_os_string());
    let _claude_binary = EnvGuard::unset(CLAUDE_BINARY_ENV);

    let resolved = resolve_claude_binary_path(None, None, None, env::var_os(PATH_ENV))
        .expect("ambient PATH should resolve claude");

    assert_eq!(
        resolved,
        fs::canonicalize(&ambient_binary).expect("canonicalize ambient binary")
    );

    fs::remove_dir_all(ambient_dir).expect("ambient dir should be removed");
}

#[test]
fn resolve_claude_mcp_command_applies_precedence_and_home_injection() {
    let resolved = resolve_claude_mcp_command_with_env(
        &sample_config(),
        &sample_context(),
        Some(OsString::from("/tmp/from-env")),
        Some(PathBuf::from("/tmp/from-ambient-home")),
    )
    .expect("command should resolve");
    let layout = ClaudeHomeLayout::new("/tmp/claude-home");

    assert_eq!(resolved.binary_path, PathBuf::from("/tmp/fake-claude"));
    assert_eq!(resolved.working_dir, Some(PathBuf::from("request/workdir")));
    assert_eq!(resolved.timeout, Some(Duration::from_secs(5)));
    assert_eq!(
        resolved.env.get("CONFIG_ONLY").map(String::as_str),
        Some("config-only")
    );
    assert_eq!(
        resolved.env.get("OVERRIDE_ME").map(String::as_str),
        Some("request")
    );
    assert_eq!(
        resolved.env.get("REQUEST_ONLY").map(String::as_str),
        Some("request-only")
    );
    assert_eq!(
        resolved
            .env
            .get(DISABLE_AUTOUPDATER_ENV)
            .map(String::as_str),
        Some("1")
    );
    assert_eq!(
        resolved.env.get(CLAUDE_HOME_ENV).map(String::as_str),
        Some("/tmp/claude-home")
    );
    assert_eq!(
        resolved.env.get(HOME_ENV).map(String::as_str),
        Some("/tmp/claude-home")
    );
    assert_eq!(
        resolved.env.get(XDG_CONFIG_HOME_ENV).map(String::as_str),
        Some(layout.xdg_config_home().to_string_lossy().as_ref())
    );
    assert_eq!(
        resolved.env.get(XDG_DATA_HOME_ENV).map(String::as_str),
        Some(layout.xdg_data_home().to_string_lossy().as_ref())
    );
    assert_eq!(
        resolved.env.get(XDG_CACHE_HOME_ENV).map(String::as_str),
        Some(layout.xdg_cache_home().to_string_lossy().as_ref())
    );
    assert_eq!(resolved.materialize_claude_home, Some(layout));
}

#[test]
fn resolve_claude_mcp_command_uses_backend_defaults_when_request_values_absent() {
    let resolved = resolve_claude_mcp_command_with_env(
        &sample_config(),
        &AgentWrapperMcpCommandContext::default(),
        None,
        None,
    )
    .expect("command should resolve");

    assert_eq!(resolved.working_dir, Some(PathBuf::from("default/workdir")));
    assert_eq!(resolved.timeout, Some(Duration::from_secs(30)));
}

#[cfg(unix)]
#[test]
fn resolve_claude_mcp_command_prefers_request_path_over_config_and_ambient_path() {
    let _env_lock = test_env_lock().lock().expect("lock test env");
    let request_dir = temp_test_dir("request-command-path");
    let config_dir = temp_test_dir("config-command-path");
    let ambient_dir = temp_test_dir("ambient-command-path");
    let request_binary = write_fake_claude(&request_dir, "#!/usr/bin/env bash\nexit 0\n");
    let _config_binary = write_fake_claude(&config_dir, "#!/usr/bin/env bash\nexit 0\n");
    let _ambient_binary = write_fake_claude(&ambient_dir, "#!/usr/bin/env bash\nexit 0\n");
    let _ambient_path = EnvGuard::set(PATH_ENV, ambient_dir.as_os_str().to_os_string());
    let _claude_binary = EnvGuard::unset(CLAUDE_BINARY_ENV);

    let mut config = sample_config_without_home();
    config.binary = None;
    config.env.insert(
        PATH_ENV.to_string(),
        config_dir.to_string_lossy().into_owned(),
    );

    let mut context = AgentWrapperMcpCommandContext::default();
    context.env.insert(
        PATH_ENV.to_string(),
        request_dir.to_string_lossy().into_owned(),
    );

    let resolved =
        resolve_claude_mcp_command_with_env(&config, &context, None, None).expect("resolve");

    assert_eq!(
        resolved.binary_path,
        fs::canonicalize(&request_binary).expect("canonicalize request binary")
    );

    fs::remove_dir_all(request_dir).expect("request dir should be removed");
    fs::remove_dir_all(config_dir).expect("config dir should be removed");
    fs::remove_dir_all(ambient_dir).expect("ambient dir should be removed");
}

#[cfg(unix)]
#[test]
fn resolve_claude_mcp_command_prefers_config_path_over_ambient_path() {
    let _env_lock = test_env_lock().lock().expect("lock test env");
    let config_dir = temp_test_dir("config-command-only-path");
    let ambient_dir = temp_test_dir("ambient-command-only-path");
    let config_binary = write_fake_claude(&config_dir, "#!/usr/bin/env bash\nexit 0\n");
    let _ambient_binary = write_fake_claude(&ambient_dir, "#!/usr/bin/env bash\nexit 0\n");
    let _ambient_path = EnvGuard::set(PATH_ENV, ambient_dir.as_os_str().to_os_string());
    let _claude_binary = EnvGuard::unset(CLAUDE_BINARY_ENV);

    let mut config = sample_config_without_home();
    config.binary = None;
    config.env.insert(
        PATH_ENV.to_string(),
        config_dir.to_string_lossy().into_owned(),
    );

    let resolved = resolve_claude_mcp_command_with_env(
        &config,
        &AgentWrapperMcpCommandContext::default(),
        None,
        None,
    )
    .expect("command should resolve");

    assert_eq!(
        resolved.binary_path,
        fs::canonicalize(&config_binary).expect("canonicalize config binary")
    );

    fs::remove_dir_all(config_dir).expect("config dir should be removed");
    fs::remove_dir_all(ambient_dir).expect("ambient dir should be removed");
}

#[test]
fn disable_autoupdater_default_does_not_override_explicit_values() {
    let mut config = sample_config();
    config
        .env
        .insert(DISABLE_AUTOUPDATER_ENV.to_string(), "0".to_string());
    let resolved = resolve_claude_mcp_command_with_env(
        &config,
        &AgentWrapperMcpCommandContext::default(),
        None,
        None,
    )
    .expect("command should resolve");
    assert_eq!(
        resolved
            .env
            .get(DISABLE_AUTOUPDATER_ENV)
            .map(String::as_str),
        Some("0")
    );

    let mut context = AgentWrapperMcpCommandContext::default();
    context
        .env
        .insert(DISABLE_AUTOUPDATER_ENV.to_string(), "2".to_string());
    let resolved =
        resolve_claude_mcp_command_with_env(&config, &context, None, None).expect("resolve");
    assert_eq!(
        resolved
            .env
            .get(DISABLE_AUTOUPDATER_ENV)
            .map(String::as_str),
        Some("2")
    );
}

#[test]
fn request_env_overrides_injected_home_keys() {
    let mut context = AgentWrapperMcpCommandContext::default();
    context
        .env
        .insert(HOME_ENV.to_string(), "/tmp/request-home".to_string());
    context.env.insert(
        XDG_CONFIG_HOME_ENV.to_string(),
        "/tmp/request-xdg-config".to_string(),
    );

    let resolved = resolve_claude_mcp_command_with_env(&sample_config(), &context, None, None)
        .expect("resolve");

    assert_eq!(
        resolved.env.get(HOME_ENV).map(String::as_str),
        Some("/tmp/request-home")
    );
    assert_eq!(
        resolved.env.get(XDG_CONFIG_HOME_ENV).map(String::as_str),
        Some("/tmp/request-xdg-config")
    );
    assert_eq!(
        resolved.env.get(CLAUDE_HOME_ENV).map(String::as_str),
        Some("/tmp/claude-home")
    );
    assert_eq!(resolved.materialize_claude_home, None);
}

#[test]
fn request_env_override_of_claude_home_disables_materialization() {
    let mut context = AgentWrapperMcpCommandContext::default();
    context.env.insert(
        CLAUDE_HOME_ENV.to_string(),
        "/tmp/request-claude-home".to_string(),
    );

    let resolved = resolve_claude_mcp_command_with_env(&sample_config(), &context, None, None)
        .expect("resolve");

    assert_eq!(
        resolved.env.get(CLAUDE_HOME_ENV).map(String::as_str),
        Some("/tmp/request-claude-home")
    );
    assert_eq!(resolved.materialize_claude_home, None);
}

#[test]
fn request_env_with_same_injected_home_values_keeps_materialization() {
    let layout = ClaudeHomeLayout::new("/tmp/claude-home");
    let mut context = AgentWrapperMcpCommandContext::default();
    context
        .env
        .insert(CLAUDE_HOME_ENV.to_string(), "/tmp/claude-home".to_string());
    context
        .env
        .insert(HOME_ENV.to_string(), "/tmp/claude-home".to_string());
    context.env.insert(
        XDG_CONFIG_HOME_ENV.to_string(),
        layout.xdg_config_home().to_string_lossy().into_owned(),
    );
    context.env.insert(
        XDG_DATA_HOME_ENV.to_string(),
        layout.xdg_data_home().to_string_lossy().into_owned(),
    );
    context.env.insert(
        XDG_CACHE_HOME_ENV.to_string(),
        layout.xdg_cache_home().to_string_lossy().into_owned(),
    );

    let resolved = resolve_claude_mcp_command_with_env(&sample_config(), &context, None, None)
        .expect("resolve");

    assert_eq!(resolved.materialize_claude_home, Some(layout));
}

#[test]
fn ambient_claude_home_is_used_when_config_home_is_absent() {
    let ambient_home = PathBuf::from("/tmp/ambient-claude-home");
    let layout = ClaudeHomeLayout::new(ambient_home.clone());
    let resolved = resolve_claude_mcp_command_with_env(
        &sample_config_without_home(),
        &AgentWrapperMcpCommandContext::default(),
        None,
        Some(ambient_home.clone()),
    )
    .expect("command should resolve");

    assert_eq!(
        resolved.env.get(CLAUDE_HOME_ENV).map(String::as_str),
        Some(ambient_home.to_string_lossy().as_ref())
    );
    assert_eq!(
        resolved.env.get(HOME_ENV).map(String::as_str),
        Some(ambient_home.to_string_lossy().as_ref())
    );
    assert_eq!(
        resolved.env.get(XDG_CONFIG_HOME_ENV).map(String::as_str),
        Some(layout.xdg_config_home().to_string_lossy().as_ref())
    );
    assert_eq!(resolved.materialize_claude_home, Some(layout));
}

#[test]
fn blank_ambient_claude_home_is_ignored_when_config_home_is_absent() {
    let resolved = resolve_claude_mcp_command_with_env(
        &sample_config_without_home(),
        &AgentWrapperMcpCommandContext::default(),
        None,
        Some(PathBuf::new()),
    )
    .expect("command should resolve");

    assert_eq!(resolved.env.get(CLAUDE_HOME_ENV), None);
    assert_eq!(resolved.env.get(HOME_ENV), None);
    assert_eq!(resolved.materialize_claude_home, None);
}

#[test]
fn configured_claude_home_beats_ambient_claude_home() {
    let layout = ClaudeHomeLayout::new("/tmp/claude-home");
    let resolved = resolve_claude_mcp_command_with_env(
        &sample_config(),
        &AgentWrapperMcpCommandContext::default(),
        None,
        Some(PathBuf::from("/tmp/ambient-claude-home")),
    )
    .expect("command should resolve");

    assert_eq!(
        resolved.env.get(CLAUDE_HOME_ENV).map(String::as_str),
        Some("/tmp/claude-home")
    );
    assert_eq!(resolved.materialize_claude_home, Some(layout));
}

#[test]
fn config_env_override_of_home_disables_materialization() {
    let mut config = sample_config();
    config
        .env
        .insert(HOME_ENV.to_string(), "/tmp/config-home".to_string());

    let resolved = resolve_claude_mcp_command_with_env(
        &config,
        &AgentWrapperMcpCommandContext::default(),
        None,
        None,
    )
    .expect("command should resolve");

    assert_eq!(
        resolved.env.get(CLAUDE_HOME_ENV).map(String::as_str),
        Some("/tmp/claude-home")
    );
    assert_eq!(
        resolved.env.get(HOME_ENV).map(String::as_str),
        Some("/tmp/config-home")
    );
    assert_eq!(resolved.materialize_claude_home, None);
}

#[test]
fn request_env_override_of_ambient_claude_home_disables_materialization() {
    let ambient_home = PathBuf::from("/tmp/ambient-claude-home");
    let mut context = AgentWrapperMcpCommandContext::default();
    context.env.insert(
        CLAUDE_HOME_ENV.to_string(),
        "/tmp/request-claude-home".to_string(),
    );
    context.env.insert(
        XDG_CONFIG_HOME_ENV.to_string(),
        "/tmp/request-xdg-config".to_string(),
    );

    let resolved = resolve_claude_mcp_command_with_env(
        &sample_config_without_home(),
        &context,
        None,
        Some(ambient_home.clone()),
    )
    .expect("command should resolve");

    assert_eq!(
        resolved.env.get(CLAUDE_HOME_ENV).map(String::as_str),
        Some("/tmp/request-claude-home")
    );
    assert_eq!(
        resolved.env.get(XDG_CONFIG_HOME_ENV).map(String::as_str),
        Some("/tmp/request-xdg-config")
    );
    assert_eq!(resolved.materialize_claude_home, None);
}

#[test]
fn no_claude_home_is_materialized_without_config_or_ambient_home() {
    let resolved = resolve_claude_mcp_command_with_env(
        &sample_config_without_home(),
        &AgentWrapperMcpCommandContext::default(),
        None,
        None,
    )
    .expect("command should resolve");

    assert_eq!(resolved.env.get(CLAUDE_HOME_ENV), None);
    assert_eq!(resolved.env.get(HOME_ENV), None);
    assert_eq!(resolved.materialize_claude_home, None);
}

#[tokio::test]
async fn capture_bounded_preserves_small_streams() {
    let (writer, reader) = duplex(64);
    let writer_task = tokio::spawn(write_all_and_close(writer, b"hello".to_vec()));

    let (captured, saw_more) = capture_bounded(reader, 8).await.expect("capture succeeds");
    writer_task.await.expect("writer completes");

    assert_eq!(captured, b"hello");
    assert!(!saw_more);
}

#[tokio::test]
async fn capture_bounded_retains_only_bound_and_marks_overflow() {
    let (writer, reader) = duplex(64);
    let writer_task = tokio::spawn(write_all_and_close(
        writer,
        b"abcdefghijklmnopqrstuvwxyz".to_vec(),
    ));

    let (captured, saw_more) = capture_bounded(reader, 8).await.expect("capture succeeds");
    writer_task.await.expect("writer completes");

    assert_eq!(captured, b"abcdefgh");
    assert!(saw_more);
}

#[tokio::test]
async fn capture_bounded_with_zero_bound_drains_input_and_reports_overflow() {
    let (writer, reader) = duplex(64);
    let writer_task = tokio::spawn(write_all_and_close(writer, b"abcdef".to_vec()));

    let (captured, saw_more) = capture_bounded(reader, 0).await.expect("capture succeeds");
    writer_task.await.expect("writer completes");

    assert!(captured.is_empty());
    assert!(saw_more);
}

#[test]
fn classify_manifest_runtime_conflict_detects_unknown_mcp_command() {
    assert!(classify_manifest_runtime_conflict_text(
        &claude_mcp_list_argv(),
        "error: unrecognized subcommand 'mcp'"
    ));
}

#[test]
fn classify_manifest_runtime_conflict_detects_unknown_get_subcommand() {
    assert!(classify_manifest_runtime_conflict_text(
        &claude_mcp_get_argv("demo"),
        "error: no such subcommand 'get'"
    ));
}

#[test]
fn classify_manifest_runtime_conflict_detects_unknown_add_subcommand() {
    let transport = AgentWrapperMcpAddTransport::Stdio {
        command: vec!["node".to_string()],
        args: vec!["server.js".to_string()],
        env: BTreeMap::new(),
    };
    let argv = claude_mcp_add_argv("demo", &transport).expect("stdio transport should map");

    assert!(classify_manifest_runtime_conflict_text(
        &argv,
        "error: unknown subcommand 'add'"
    ));
}

#[test]
fn classify_manifest_runtime_conflict_detects_add_transport_flag_drift_without_echoed_add() {
    let transport = AgentWrapperMcpAddTransport::Stdio {
        command: vec!["node".to_string()],
        args: vec!["server.js".to_string()],
        env: BTreeMap::new(),
    };
    let argv = claude_mcp_add_argv("demo", &transport).expect("stdio transport should map");

    assert!(classify_manifest_runtime_conflict_text(
        &argv,
        "error: unexpected argument '--transport' found"
    ));
}

#[test]
fn classify_manifest_runtime_conflict_detects_add_env_flag_usage_drift() {
    let transport = AgentWrapperMcpAddTransport::Stdio {
        command: vec!["node".to_string()],
        args: vec!["server.js".to_string()],
        env: BTreeMap::from([("ALPHA_ENV".to_string(), "1".to_string())]),
    };
    let argv = claude_mcp_add_argv("demo", &transport).expect("stdio transport should map");

    assert!(classify_manifest_runtime_conflict_text(
        &argv,
        "error: unexpected argument '--env' found\n\nusage: claude mcp add [options]"
    ));
}

#[test]
fn classify_manifest_runtime_conflict_detects_unknown_list_subcommand() {
    assert!(classify_manifest_runtime_conflict_text(
        &claude_mcp_list_argv(),
        "error: unknown subcommand 'list'"
    ));
}

#[test]
fn classify_manifest_runtime_conflict_detects_unknown_remove_subcommand() {
    assert!(classify_manifest_runtime_conflict_text(
        &claude_mcp_remove_argv("demo"),
        "error: unrecognized subcommand 'remove'"
    ));
}

#[test]
fn classify_manifest_runtime_conflict_ignores_domain_failures() {
    assert!(!classify_manifest_runtime_conflict_text(
        &claude_mcp_get_argv("demo"),
        "server demo not found"
    ));
    assert!(!classify_manifest_runtime_conflict_text(
        &claude_mcp_get_argv("demo"),
        "unknown server demo"
    ));
    assert!(!classify_manifest_runtime_conflict_text(
        &claude_mcp_get_argv("demo"),
        "permission denied while contacting remote service"
    ));
    assert!(!classify_manifest_runtime_conflict_text(
        &claude_mcp_get_argv("demo"),
        "network error: failed to connect"
    ));

    let transport = AgentWrapperMcpAddTransport::Stdio {
        command: vec!["node".to_string()],
        args: vec!["server.js".to_string()],
        env: BTreeMap::new(),
    };
    let argv = claude_mcp_add_argv("demo", &transport).expect("stdio transport should map");
    assert!(!classify_manifest_runtime_conflict_text(
        &argv,
        "error: unexpected argument '--foo' found"
    ));
}

#[test]
fn finalize_claude_mcp_output_returns_backend_error_for_drift() {
    let err = finalize_claude_mcp_output(
        &claude_mcp_get_argv("demo"),
        CapturedClaudeMcpCommandOutput {
            status: exit_status_with_code(2),
            stdout_bytes: b"raw stdout should not leak".to_vec(),
            stdout_saw_more: false,
            stderr_bytes: b"error: no such subcommand 'get'".to_vec(),
            stderr_saw_more: false,
        },
    )
    .expect_err("drift should fail closed");

    match err {
        AgentWrapperError::Backend { message } => {
            assert_eq!(message, PINNED_MCP_RUNTIME_CONFLICT);
        }
        other => panic!("expected Backend error, got {other:?}"),
    }
}

#[test]
fn finalize_claude_mcp_output_returns_backend_error_for_add_flag_drift() {
    let transport = AgentWrapperMcpAddTransport::Stdio {
        command: vec!["node".to_string()],
        args: vec!["server.js".to_string()],
        env: BTreeMap::new(),
    };
    let argv = claude_mcp_add_argv("demo", &transport).expect("stdio transport should map");

    let err = finalize_claude_mcp_output(
        &argv,
        CapturedClaudeMcpCommandOutput {
            status: exit_status_with_code(2),
            stdout_bytes: b"raw stdout should not leak".to_vec(),
            stdout_saw_more: false,
            stderr_bytes: b"error: unexpected argument '--transport' found".to_vec(),
            stderr_saw_more: false,
        },
    )
    .expect_err("add flag drift should fail closed");

    match err {
        AgentWrapperError::Backend { message } => {
            assert_eq!(message, PINNED_MCP_RUNTIME_CONFLICT);
        }
        other => panic!("expected Backend error, got {other:?}"),
    }
}

#[test]
fn finalize_claude_mcp_output_keeps_normal_non_zero_exits_as_ok() {
    let output = finalize_claude_mcp_output(
        &claude_mcp_get_argv("demo"),
        CapturedClaudeMcpCommandOutput {
            status: exit_status_with_code(3),
            stdout_bytes: b"listed output".to_vec(),
            stdout_saw_more: false,
            stderr_bytes: b"server demo not found".to_vec(),
            stderr_saw_more: false,
        },
    )
    .expect("normal failures should remain Ok(output)");

    assert_eq!(output.status, exit_status_with_code(3));
    assert_eq!(output.stdout, "listed output");
    assert_eq!(output.stderr, "server demo not found");
    assert!(!output.stdout_truncated);
    assert!(!output.stderr_truncated);
}

#[test]
fn finalize_claude_mcp_output_detects_drift_in_stdout_too() {
    let err = finalize_claude_mcp_output(
        &claude_mcp_list_argv(),
        CapturedClaudeMcpCommandOutput {
            status: exit_status_with_code(4),
            stdout_bytes: b"error: unknown subcommand 'list'".to_vec(),
            stdout_saw_more: false,
            stderr_bytes: Vec::new(),
            stderr_saw_more: false,
        },
    )
    .expect_err("stdout drift should fail closed");

    match err {
        AgentWrapperError::Backend { message } => {
            assert_eq!(message, PINNED_MCP_RUNTIME_CONFLICT);
        }
        other => panic!("expected Backend error, got {other:?}"),
    }
}

#[test]
fn success_exit_skips_drift_classification() {
    let output = finalize_claude_mcp_output(
        &claude_mcp_get_argv("demo"),
        CapturedClaudeMcpCommandOutput {
            status: success_exit_status(),
            stdout_bytes: b"error: no such subcommand 'get'".to_vec(),
            stdout_saw_more: false,
            stderr_bytes: Vec::new(),
            stderr_saw_more: false,
        },
    )
    .expect("successful exits should remain Ok(output)");

    assert_eq!(output.status, success_exit_status());
    assert_eq!(output.stdout, "error: no such subcommand 'get'");
    assert!(output.stderr.is_empty());
}
