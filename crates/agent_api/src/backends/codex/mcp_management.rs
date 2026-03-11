use std::{
    collections::BTreeMap,
    env,
    ffi::OsString,
    fs, io,
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};

use codex::CodexHomeLayout;
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    process::{Child, Command},
    task::JoinHandle,
};

use crate::{
    bounds::{enforce_mcp_output_bound, MCP_STDERR_BOUND_BYTES, MCP_STDOUT_BOUND_BYTES},
    mcp::{
        AgentWrapperMcpAddTransport, AgentWrapperMcpCommandContext, AgentWrapperMcpCommandOutput,
    },
    AgentWrapperError,
};

const CODEX_BINARY_ENV: &str = "CODEX_BINARY";
const CODEX_HOME_ENV: &str = "CODEX_HOME";
const PATH_ENV: &str = "PATH";

const PINNED_SPAWN_FAILURE: &str = "codex backend error: spawn (details redacted when unsafe)";
const PINNED_WAIT_FAILURE: &str = "codex backend error: wait (details redacted when unsafe)";
const PINNED_CAPTURE_FAILURE: &str = "codex backend error: capture (details redacted when unsafe)";
const PINNED_PREPARE_CODEX_HOME_FAILURE: &str =
    "codex backend error: prepare CODEX_HOME (details redacted when unsafe)";
const PINNED_MCP_RUNTIME_CONFLICT: &str =
    "codex backend error: installed codex does not support pinned mcp management command shape (details redacted)";

#[derive(Clone, Debug, PartialEq, Eq)]
struct ResolvedCodexMcpCommand {
    binary_path: PathBuf,
    working_dir: Option<PathBuf>,
    timeout: Option<Duration>,
    env: BTreeMap<String, String>,
    materialize_codex_home: Option<PathBuf>,
}

struct CapturedCodexMcpCommandOutput {
    status: std::process::ExitStatus,
    stdout_bytes: Vec<u8>,
    stdout_saw_more: bool,
    stderr_bytes: Vec<u8>,
    stderr_saw_more: bool,
}

pub(super) fn codex_mcp_list_argv() -> Vec<OsString> {
    vec![
        OsString::from("mcp"),
        OsString::from("list"),
        OsString::from("--json"),
    ]
}

pub(super) fn codex_mcp_get_argv(name: &str) -> Vec<OsString> {
    vec![
        OsString::from("mcp"),
        OsString::from("get"),
        OsString::from("--json"),
        OsString::from(name),
    ]
}

pub(super) fn codex_mcp_remove_argv(name: &str) -> Vec<OsString> {
    vec![
        OsString::from("mcp"),
        OsString::from("remove"),
        OsString::from(name),
    ]
}

pub(super) fn codex_mcp_add_argv(
    name: &str,
    transport: &AgentWrapperMcpAddTransport,
) -> Vec<OsString> {
    let mut argv = vec![
        OsString::from("mcp"),
        OsString::from("add"),
        OsString::from(name),
    ];

    match transport {
        AgentWrapperMcpAddTransport::Stdio { command, args, env } => {
            for (key, value) in env {
                argv.push(OsString::from("--env"));
                argv.push(OsString::from(format!("{key}={value}")));
            }
            argv.push(OsString::from("--"));
            argv.extend(command.iter().cloned().map(OsString::from));
            argv.extend(args.iter().cloned().map(OsString::from));
        }
        AgentWrapperMcpAddTransport::Url {
            url,
            bearer_token_env_var,
        } => {
            argv.push(OsString::from("--url"));
            argv.push(OsString::from(url));
            if let Some(env_var) = bearer_token_env_var {
                argv.push(OsString::from("--bearer-token-env-var"));
                argv.push(OsString::from(env_var));
            }
        }
    }

    argv
}

pub(super) async fn run_codex_mcp(
    config: super::CodexBackendConfig,
    argv: Vec<OsString>,
    context: AgentWrapperMcpCommandContext,
) -> Result<AgentWrapperMcpCommandOutput, AgentWrapperError> {
    let resolved = resolve_codex_mcp_command(&config, &context);

    if let Some(codex_home) = resolved.materialize_codex_home.as_ref() {
        CodexHomeLayout::new(codex_home.clone())
            .materialize(true)
            .map_err(|_| backend_error(PINNED_PREPARE_CODEX_HOME_FAILURE))?;
    }

    let mut command = Command::new(&resolved.binary_path);
    command
        .args(&argv)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .env_clear()
        .envs(&resolved.env);

    if let Some(working_dir) = resolved.working_dir.as_ref() {
        command.current_dir(working_dir);
    }

    let mut child = command
        .spawn()
        .map_err(|_| backend_error(PINNED_SPAWN_FAILURE))?;

    let Some(stdout) = child.stdout.take() else {
        cleanup_child(&mut child).await;
        return Err(backend_error(PINNED_CAPTURE_FAILURE));
    };
    let Some(stderr) = child.stderr.take() else {
        cleanup_child(&mut child).await;
        return Err(backend_error(PINNED_CAPTURE_FAILURE));
    };

    let stdout_task = tokio::spawn(capture_bounded(stdout, MCP_STDOUT_BOUND_BYTES));
    let stderr_task = tokio::spawn(capture_bounded(stderr, MCP_STDERR_BOUND_BYTES));

    let status = match wait_for_exit(&mut child, resolved.timeout).await {
        Ok(status) => status,
        Err(err) => {
            stdout_task.abort();
            stderr_task.abort();
            return Err(err);
        }
    };

    let (stdout_bytes, stdout_saw_more) = join_capture_task(stdout_task).await?;
    let (stderr_bytes, stderr_saw_more) = join_capture_task(stderr_task).await?;

    finalize_codex_mcp_output(
        &argv,
        CapturedCodexMcpCommandOutput {
            status,
            stdout_bytes,
            stdout_saw_more,
            stderr_bytes,
            stderr_saw_more,
        },
    )
}

fn finalize_codex_mcp_output(
    argv: &[OsString],
    captured: CapturedCodexMcpCommandOutput,
) -> Result<AgentWrapperMcpCommandOutput, AgentWrapperError> {
    if !captured.status.success()
        && is_manifest_runtime_conflict(argv, &captured.stdout_bytes, &captured.stderr_bytes)
    {
        return Err(backend_error(PINNED_MCP_RUNTIME_CONFLICT));
    }

    let (stdout, stdout_truncated) = enforce_mcp_output_bound(
        &captured.stdout_bytes,
        captured.stdout_saw_more,
        MCP_STDOUT_BOUND_BYTES,
    );
    let (stderr, stderr_truncated) = enforce_mcp_output_bound(
        &captured.stderr_bytes,
        captured.stderr_saw_more,
        MCP_STDERR_BOUND_BYTES,
    );

    Ok(AgentWrapperMcpCommandOutput {
        status: captured.status,
        stdout,
        stderr,
        stdout_truncated,
        stderr_truncated,
    })
}

fn resolve_codex_mcp_command(
    config: &super::CodexBackendConfig,
    context: &AgentWrapperMcpCommandContext,
) -> ResolvedCodexMcpCommand {
    let binary_path = resolve_codex_binary_path(
        config.binary.as_ref(),
        env::var_os(CODEX_BINARY_ENV),
        context
            .env
            .get(PATH_ENV)
            .map(String::as_str)
            .or_else(|| config.env.get(PATH_ENV).map(String::as_str)),
        env::var_os(PATH_ENV),
    );
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

    let materialize_codex_home = config.codex_home.clone().filter(|codex_home| {
        env.get(CODEX_HOME_ENV)
            .is_some_and(|value| value == &codex_home.to_string_lossy())
    });

    ResolvedCodexMcpCommand {
        binary_path,
        working_dir: context
            .working_dir
            .clone()
            .or_else(|| config.default_working_dir.clone()),
        timeout: context.timeout.or(config.default_timeout),
        env,
        materialize_codex_home,
    }
}

fn default_codex_binary_path() -> PathBuf {
    env::var_os(CODEX_BINARY_ENV)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("codex"))
}

fn resolve_codex_binary_path(
    config_binary: Option<&PathBuf>,
    ambient_codex_binary: Option<OsString>,
    effective_path_env: Option<&str>,
    ambient_path_env: Option<OsString>,
) -> PathBuf {
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
}

fn resolve_path_for_spawn(
    binary_path: PathBuf,
    effective_path_env: Option<&str>,
    ambient_path_env: Option<OsString>,
) -> PathBuf {
    if is_path_qualified(&binary_path) {
        return binary_path;
    }

    if let Some(path_env) = effective_path_env {
        return find_binary_on_path(&binary_path, Some(OsString::from(path_env)))
            .unwrap_or(binary_path);
    }

    find_binary_on_path(&binary_path, ambient_path_env).unwrap_or(binary_path)
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

async fn wait_for_exit(
    child: &mut Child,
    timeout: Option<Duration>,
) -> Result<std::process::ExitStatus, AgentWrapperError> {
    match timeout {
        Some(timeout) => match tokio::time::timeout(timeout, child.wait()).await {
            Ok(Ok(status)) => Ok(status),
            Ok(Err(_)) => Err(backend_error(PINNED_WAIT_FAILURE)),
            Err(_) => {
                cleanup_child(child).await;
                Err(backend_error(super::PINNED_TIMEOUT))
            }
        },
        None => child
            .wait()
            .await
            .map_err(|_| backend_error(PINNED_WAIT_FAILURE)),
    }
}

async fn cleanup_child(child: &mut Child) {
    let _ = child.kill().await;
    let _ = child.wait().await;
}

async fn join_capture_task(
    task: JoinHandle<io::Result<(Vec<u8>, bool)>>,
) -> Result<(Vec<u8>, bool), AgentWrapperError> {
    task.await
        .map_err(|_| backend_error(PINNED_CAPTURE_FAILURE))?
        .map_err(|_| backend_error(PINNED_CAPTURE_FAILURE))
}

async fn capture_bounded<R>(mut reader: R, bound_bytes: usize) -> io::Result<(Vec<u8>, bool)>
where
    R: AsyncRead + Unpin,
{
    let retain_bound = bound_bytes.saturating_add(1);
    let mut retained = Vec::with_capacity(retain_bound.min(4096));
    let mut saw_more = false;
    let mut chunk = [0u8; 4096];

    loop {
        let read = reader.read(&mut chunk).await?;
        if read == 0 {
            break;
        }

        if retained.len() < retain_bound {
            let remaining = retain_bound - retained.len();
            let to_copy = remaining.min(read);
            retained.extend_from_slice(&chunk[..to_copy]);
            if to_copy < read {
                saw_more = true;
            }
        } else {
            saw_more = true;
        }
    }

    if retained.len() > bound_bytes {
        retained.truncate(bound_bytes);
        saw_more = true;
    }

    Ok((retained, saw_more))
}

fn is_manifest_runtime_conflict(argv: &[OsString], stdout: &[u8], stderr: &[u8]) -> bool {
    let stderr = String::from_utf8_lossy(stderr);
    let stdout = String::from_utf8_lossy(stdout);
    classify_manifest_runtime_conflict_text(argv, &format!("{stderr}\n{stdout}"))
}

fn classify_manifest_runtime_conflict_text(argv: &[OsString], text: &str) -> bool {
    let text = text.to_ascii_lowercase();

    let unknown_signal = [
        "unknown",
        "unrecognized",
        "unexpected",
        "invalid",
        "no such",
        "not recognized",
    ]
    .iter()
    .any(|signal| text.contains(signal));

    if !unknown_signal {
        return false;
    }

    let syntax_context = ["command", "subcommand", "argument", "option", "flag"]
        .iter()
        .any(|signal| text.contains(signal));
    if !syntax_context {
        return false;
    }

    let subcommand_conflict = manifest_conflict_tokens(argv)
        .into_iter()
        .any(|token| text.contains(token));
    let json_flag_conflict = text.contains("--json")
        && (text.contains("flag") || text.contains("option") || text.contains("argument"));
    let add_usage_conflict = text.contains("mcp add")
        && text.contains("usage:")
        && (text.contains("unexpected argument")
            || text.contains("unexpected arguments")
            || text.contains("missing required argument")
            || text.contains("requires")
            || text.contains("found argument")
            || text.contains("unexpected value"));

    subcommand_conflict || json_flag_conflict || add_usage_conflict
}

fn manifest_conflict_tokens(argv: &[OsString]) -> Vec<&'static str> {
    let mut tokens = vec!["mcp"];
    match argv.get(1).and_then(|arg| arg.to_str()) {
        Some("list") => tokens.push("list"),
        Some("get") => tokens.push("get"),
        Some("add") => tokens.push("add"),
        Some("remove") => tokens.push("remove"),
        _ => {}
    }
    tokens
}

fn backend_error(message: &'static str) -> AgentWrapperError {
    AgentWrapperError::Backend {
        message: message.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        collections::BTreeMap,
        ffi::OsString,
        process::ExitStatus,
        sync::{Mutex, OnceLock},
    };
    #[cfg(unix)]
    use std::{
        fs,
        os::unix::fs::PermissionsExt,
        time::{SystemTime, UNIX_EPOCH},
    };

    use tokio::io::{AsyncWriteExt, DuplexStream};

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

    fn sample_config() -> super::super::CodexBackendConfig {
        super::super::CodexBackendConfig {
            binary: Some(PathBuf::from("/tmp/fake-codex")),
            codex_home: Some(PathBuf::from("/tmp/codex-home")),
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
            "agent-api-codex-mcp-{label}-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).expect("temp dir should be created");
        dir
    }

    #[cfg(unix)]
    fn write_fake_codex(dir: &std::path::Path, script: &str) -> PathBuf {
        let path = dir.join("codex");
        fs::write(&path, script).expect("script should be written");
        let mut permissions = fs::metadata(&path)
            .expect("script metadata should exist")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&path, permissions).expect("script should be executable");
        path
    }

    #[test]
    fn codex_mcp_list_argv_is_pinned() {
        assert_eq!(
            codex_mcp_list_argv(),
            vec![
                OsString::from("mcp"),
                OsString::from("list"),
                OsString::from("--json"),
            ]
        );
    }

    #[test]
    fn codex_mcp_get_argv_is_pinned() {
        assert_eq!(
            codex_mcp_get_argv("demo"),
            vec![
                OsString::from("mcp"),
                OsString::from("get"),
                OsString::from("--json"),
                OsString::from("demo"),
            ]
        );
    }

    #[test]
    fn codex_mcp_remove_argv_is_pinned() {
        assert_eq!(
            codex_mcp_remove_argv("demo"),
            vec![
                OsString::from("mcp"),
                OsString::from("remove"),
                OsString::from("demo"),
            ]
        );
    }

    #[test]
    fn codex_mcp_add_argv_maps_stdio_transport_with_sorted_env_and_separator() {
        let transport = AgentWrapperMcpAddTransport::Stdio {
            command: vec!["node".to_string()],
            args: vec!["server.js".to_string(), "--flag".to_string()],
            env: BTreeMap::from([
                ("BETA".to_string(), "two".to_string()),
                ("ALPHA".to_string(), "one".to_string()),
            ]),
        };

        assert_eq!(
            codex_mcp_add_argv("demo", &transport),
            vec![
                OsString::from("mcp"),
                OsString::from("add"),
                OsString::from("demo"),
                OsString::from("--env"),
                OsString::from("ALPHA=one"),
                OsString::from("--env"),
                OsString::from("BETA=two"),
                OsString::from("--"),
                OsString::from("node"),
                OsString::from("server.js"),
                OsString::from("--flag"),
            ]
        );
    }

    #[test]
    fn codex_mcp_add_argv_maps_url_transport() {
        let transport = AgentWrapperMcpAddTransport::Url {
            url: "https://example.test/mcp".to_string(),
            bearer_token_env_var: Some("TOKEN_ENV".to_string()),
        };

        assert_eq!(
            codex_mcp_add_argv("demo", &transport),
            vec![
                OsString::from("mcp"),
                OsString::from("add"),
                OsString::from("demo"),
                OsString::from("--url"),
                OsString::from("https://example.test/mcp"),
                OsString::from("--bearer-token-env-var"),
                OsString::from("TOKEN_ENV"),
            ]
        );
    }

    #[test]
    fn codex_mcp_add_argv_maps_url_transport_without_bearer_env() {
        let transport = AgentWrapperMcpAddTransport::Url {
            url: "https://example.test/mcp".to_string(),
            bearer_token_env_var: None,
        };

        assert_eq!(
            codex_mcp_add_argv("demo", &transport),
            vec![
                OsString::from("mcp"),
                OsString::from("add"),
                OsString::from("demo"),
                OsString::from("--url"),
                OsString::from("https://example.test/mcp"),
            ]
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn run_codex_mcp_uses_context_env_without_leaking_stdio_transport_env() {
        let temp_dir = temp_test_dir("write-env");
        let script_path = write_fake_codex(
            &temp_dir,
            r#"#!/usr/bin/env bash
printf "%s\n" "$@"
printf "CLI_ONLY=%s\n" "${CLI_ONLY-unset}" 1>&2
printf "SERVER_ONLY=%s\n" "${SERVER_ONLY-unset}" 1>&2
"#,
        );

        let transport = AgentWrapperMcpAddTransport::Stdio {
            command: vec!["node".to_string()],
            args: vec!["server.js".to_string()],
            env: BTreeMap::from([("SERVER_ONLY".to_string(), "server-value".to_string())]),
        };
        let argv = codex_mcp_add_argv("demo", &transport);
        let context = AgentWrapperMcpCommandContext {
            env: BTreeMap::from([("CLI_ONLY".to_string(), "cli-value".to_string())]),
            ..Default::default()
        };

        let result = run_codex_mcp(
            super::super::CodexBackendConfig {
                binary: Some(script_path),
                ..Default::default()
            },
            argv,
            context,
        )
        .await
        .expect("runner should succeed");

        assert_eq!(
            result.stdout.lines().collect::<Vec<_>>(),
            vec![
                "mcp",
                "add",
                "demo",
                "--env",
                "SERVER_ONLY=server-value",
                "--",
                "node",
                "server.js",
            ]
        );
        assert_eq!(
            result.stderr.lines().collect::<Vec<_>>(),
            vec!["CLI_ONLY=cli-value", "SERVER_ONLY=unset"]
        );

        fs::remove_dir_all(temp_dir).expect("temp dir should be removed");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn run_codex_mcp_clears_ambient_env_before_spawn() {
        let _env_lock = test_env_lock().lock().expect("lock test env");
        let temp_dir = temp_test_dir("ambient-env");
        let script_path = write_fake_codex(
            &temp_dir,
            r#"#!/usr/bin/env bash
printf "AMBIENT_ONLY=%s\n" "${AMBIENT_ONLY-unset}" 1>&2
printf "CODEX_HOME=%s\n" "${CODEX_HOME-unset}" 1>&2
"#,
        );
        let _ambient_only = EnvGuard::set("AMBIENT_ONLY", "ambient-value");
        let _ambient_home = EnvGuard::set("CODEX_HOME", "/tmp/ambient-codex-home");

        let result = run_codex_mcp(
            super::super::CodexBackendConfig {
                binary: Some(script_path),
                codex_home: Some(PathBuf::from("/tmp/resolved-codex-home")),
                ..Default::default()
            },
            codex_mcp_list_argv(),
            AgentWrapperMcpCommandContext::default(),
        )
        .await
        .expect("runner should succeed");

        assert_eq!(
            result.stderr.lines().collect::<Vec<_>>(),
            vec!["AMBIENT_ONLY=unset", "CODEX_HOME=/tmp/resolved-codex-home"]
        );

        fs::remove_dir_all(temp_dir).expect("temp dir should be removed");
    }

    #[test]
    fn resolve_codex_mcp_command_applies_precedence_and_materializes_injected_home() {
        let resolved = resolve_codex_mcp_command(&sample_config(), &sample_context());

        assert_eq!(resolved.binary_path, PathBuf::from("/tmp/fake-codex"));
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
            resolved.env.get(CODEX_BINARY_ENV).map(String::as_str),
            Some("/tmp/fake-codex")
        );
        assert_eq!(
            resolved.env.get(CODEX_HOME_ENV).map(String::as_str),
            Some("/tmp/codex-home")
        );
        assert_eq!(
            resolved.materialize_codex_home,
            Some(PathBuf::from("/tmp/codex-home"))
        );
    }

    #[test]
    fn resolve_codex_mcp_command_skips_materialize_when_request_overrides_codex_home() {
        let mut context = sample_context();
        context
            .env
            .insert(CODEX_HOME_ENV.to_string(), "/tmp/request-home".to_string());

        let resolved = resolve_codex_mcp_command(&sample_config(), &context);

        assert_eq!(
            resolved.env.get(CODEX_HOME_ENV).map(String::as_str),
            Some("/tmp/request-home")
        );
        assert_eq!(resolved.materialize_codex_home, None);
    }

    #[test]
    fn resolve_codex_mcp_command_uses_backend_defaults_when_request_values_absent() {
        let resolved =
            resolve_codex_mcp_command(&sample_config(), &AgentWrapperMcpCommandContext::default());

        assert_eq!(resolved.working_dir, Some(PathBuf::from("default/workdir")));
        assert_eq!(resolved.timeout, Some(Duration::from_secs(30)));
    }

    #[cfg(unix)]
    #[test]
    fn resolve_codex_binary_path_uses_effective_path_env_for_unqualified_binary() {
        let temp_dir = temp_test_dir("binary-path");
        let script_path = write_fake_codex(&temp_dir, "#!/usr/bin/env bash\nexit 0\n");

        let resolved = resolve_codex_binary_path(
            None,
            Some(OsString::from("codex")),
            Some(temp_dir.to_string_lossy().as_ref()),
            None,
        );

        assert_eq!(
            resolved,
            fs::canonicalize(&script_path).expect("canonicalize fake codex")
        );

        fs::remove_dir_all(temp_dir).expect("temp dir should be removed");
    }

    #[tokio::test]
    async fn capture_bounded_retains_only_bound_and_marks_overflow() {
        let (writer, reader) = tokio::io::duplex(64);
        let payload = b"abcdefghijklmnopqrstuvwxyz".to_vec();
        let writer_task = tokio::spawn(write_all_and_close(writer, payload));

        let (captured, saw_more) = capture_bounded(reader, 8).await.expect("capture succeeds");
        writer_task.await.expect("writer completes");

        assert_eq!(captured, b"abcdefgh");
        assert!(saw_more);
    }

    #[tokio::test]
    async fn capture_bounded_preserves_small_streams() {
        let (writer, reader) = tokio::io::duplex(64);
        let payload = b"hello".to_vec();
        let writer_task = tokio::spawn(write_all_and_close(writer, payload));

        let (captured, saw_more) = capture_bounded(reader, 8).await.expect("capture succeeds");
        writer_task.await.expect("writer completes");

        assert_eq!(captured, b"hello");
        assert!(!saw_more);
    }

    #[test]
    fn enforce_mcp_output_bound_stays_utf8_safe_after_lossy_decode() {
        let bytes = vec![0xf0, 0x9f, 0x92, 0x61, 0x62, 0x63];
        let (bounded, truncated) = enforce_mcp_output_bound(&bytes, true, 8);

        assert!(truncated);
        assert!(bounded.len() <= 8);
        assert!(std::str::from_utf8(bounded.as_bytes()).is_ok());
    }

    #[test]
    fn classify_manifest_runtime_conflict_detects_unknown_mcp_subcommand() {
        assert!(classify_manifest_runtime_conflict_text(
            &codex_mcp_list_argv(),
            "error: unrecognized subcommand 'mcp'"
        ));
    }

    #[test]
    fn classify_manifest_runtime_conflict_detects_unknown_list_subcommand() {
        assert!(classify_manifest_runtime_conflict_text(
            &codex_mcp_list_argv(),
            "error: unknown subcommand 'list'"
        ));
    }

    #[test]
    fn classify_manifest_runtime_conflict_detects_unknown_get_subcommand() {
        assert!(classify_manifest_runtime_conflict_text(
            &codex_mcp_get_argv("demo"),
            "error: no such subcommand 'get'"
        ));
    }

    #[test]
    fn classify_manifest_runtime_conflict_detects_unknown_remove_subcommand() {
        assert!(classify_manifest_runtime_conflict_text(
            &codex_mcp_remove_argv("demo"),
            "error: unrecognized subcommand 'remove'"
        ));
    }

    #[test]
    fn classify_manifest_runtime_conflict_detects_unknown_add_subcommand() {
        let transport = AgentWrapperMcpAddTransport::Stdio {
            command: vec!["node".to_string()],
            args: vec!["server.js".to_string()],
            env: BTreeMap::new(),
        };

        assert!(classify_manifest_runtime_conflict_text(
            &codex_mcp_add_argv("demo", &transport),
            "error: unknown subcommand 'add'"
        ));
    }

    #[test]
    fn classify_manifest_runtime_conflict_detects_unknown_json_flag() {
        assert!(classify_manifest_runtime_conflict_text(
            &codex_mcp_list_argv(),
            "error: unexpected argument '--json' found"
        ));
    }

    #[test]
    fn classify_manifest_runtime_conflict_detects_legacy_add_usage_error() {
        let transport = AgentWrapperMcpAddTransport::Stdio {
            command: vec!["node".to_string()],
            args: vec!["server.js".to_string()],
            env: BTreeMap::new(),
        };

        assert!(classify_manifest_runtime_conflict_text(
            &codex_mcp_add_argv("demo", &transport),
            "error: unexpected argument '--env' found\n\nusage: codex mcp add <name> --url <url>"
        ));
    }

    #[test]
    fn classify_manifest_runtime_conflict_ignores_normal_domain_failures() {
        assert!(!classify_manifest_runtime_conflict_text(
            &codex_mcp_get_argv("demo"),
            "server demo not found"
        ));
        assert!(!classify_manifest_runtime_conflict_text(
            &codex_mcp_get_argv("demo"),
            "unknown server demo"
        ));

        let transport = AgentWrapperMcpAddTransport::Stdio {
            command: vec!["node".to_string()],
            args: vec!["server.js".to_string()],
            env: BTreeMap::new(),
        };

        assert!(!classify_manifest_runtime_conflict_text(
            &codex_mcp_add_argv("demo", &transport),
            "error: unexpected argument '--env' found"
        ));
    }

    #[test]
    fn codex_unknown_get_subcommand_drift_maps_to_pinned_backend_error() {
        let err = finalize_codex_mcp_output(
            &codex_mcp_get_argv("demo"),
            CapturedCodexMcpCommandOutput {
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
    fn codex_success_exit_skips_drift_classification() {
        let output = finalize_codex_mcp_output(
            &codex_mcp_get_argv("demo"),
            CapturedCodexMcpCommandOutput {
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
}
