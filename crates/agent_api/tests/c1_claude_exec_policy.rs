#![cfg(feature = "claude_code")]

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    pin::Pin,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use agent_api::{
    backends::claude_code::{ClaudeCodeBackend, ClaudeCodeBackendConfig},
    AgentWrapperBackend, AgentWrapperError, AgentWrapperEvent, AgentWrapperEventKind,
    AgentWrapperRunRequest,
};
use claude_code::ClaudeHomeLayout;
use futures_core::Stream;
use serde_json::json;

const PINNED_EXTERNAL_SANDBOX_WARNING: &str =
    "DANGEROUS: external sandbox exec policy enabled (agent_api.exec.external_sandbox.v1=true)";

fn fake_claude_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_fake_claude_stream_json_agent_api"))
}

async fn drain_to_none(
    mut stream: Pin<&mut (dyn Stream<Item = AgentWrapperEvent> + Send)>,
    timeout: Duration,
) -> Vec<AgentWrapperEvent> {
    let mut out = Vec::new();
    let deadline = tokio::time::sleep(timeout);
    tokio::pin!(deadline);

    loop {
        tokio::select! {
            _ = &mut deadline => break,
            item = std::future::poll_fn(|cx| stream.as_mut().poll_next(cx)) => {
                match item {
                    Some(ev) => out.push(ev),
                    None => break,
                }
            }
        }
    }

    out
}

fn unique_temp_log_path(test_name: &str) -> PathBuf {
    let pid = std::process::id();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("{test_name}_{pid}_{nanos}.log"))
}

fn unique_missing_dir_path(test_name: &str) -> PathBuf {
    let pid = std::process::id();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("{test_name}_{pid}_{nanos}_missing"))
}

fn read_invocations(path: &Path) -> Vec<String> {
    let text = std::fs::read_to_string(path).expect("read FAKE_CLAUDE_INVOCATION_LOG");
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

fn read_env_snapshot(path: &Path) -> BTreeMap<String, String> {
    let text = std::fs::read_to_string(path).expect("read FAKE_CLAUDE_ENV_SNAPSHOT_PATH");
    let mut out = BTreeMap::new();
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        out.insert(key.to_string(), value.to_string());
    }
    out
}

fn warning_indices(events: &[AgentWrapperEvent]) -> Vec<usize> {
    events
        .iter()
        .enumerate()
        .filter(|(_, ev)| ev.kind == AgentWrapperEventKind::Status)
        .filter(|(_, ev)| ev.channel.as_deref() == Some("status"))
        .filter(|(_, ev)| ev.message.as_deref() == Some(PINNED_EXTERNAL_SANDBOX_WARNING))
        .filter(|(_, ev)| ev.data.is_none())
        .map(|(idx, _)| idx)
        .collect()
}

fn session_handle_facet_index(events: &[AgentWrapperEvent]) -> Option<usize> {
    events
        .iter()
        .enumerate()
        .find(|(_, ev)| {
            ev.kind == AgentWrapperEventKind::Status
                && ev
                    .data
                    .as_ref()
                    .and_then(|data| data.get("schema"))
                    .and_then(serde_json::Value::as_str)
                    == Some("agent_api.session.handle.v1")
        })
        .map(|(idx, _)| idx)
}

fn first_user_visible_index(events: &[AgentWrapperEvent]) -> Option<usize> {
    events
        .iter()
        .enumerate()
        .find(|(_, ev)| {
            matches!(
                ev.kind,
                AgentWrapperEventKind::TextOutput
                    | AgentWrapperEventKind::ToolCall
                    | AgentWrapperEventKind::ToolResult
            )
        })
        .map(|(idx, _)| idx)
}

fn first_error_index(events: &[AgentWrapperEvent]) -> Option<usize> {
    events
        .iter()
        .enumerate()
        .find(|(_, ev)| ev.kind == AgentWrapperEventKind::Error)
        .map(|(idx, _)| idx)
}

fn count(lines: &[String], needle: &str) -> usize {
    lines.iter().filter(|line| line.as_str() == needle).count()
}

fn first_index(lines: &[String], needle: &str) -> Option<usize> {
    lines.iter().position(|line| line.as_str() == needle)
}

#[tokio::test]
async fn claude_home_redirects_wrapper_managed_user_home_env_and_materializes_layout() {
    let home_root = unique_missing_dir_path("claude_home_redirect_root");
    let snapshot_path = unique_temp_log_path("claude_home_env_snapshot");

    let backend = ClaudeCodeBackend::new(ClaudeCodeBackendConfig {
        binary: Some(fake_claude_binary()),
        claude_home: Some(home_root.clone()),
        env: [
            (
                "FAKE_CLAUDE_SCENARIO".to_string(),
                "claude_home_env_snapshot".to_string(),
            ),
            (
                "FAKE_CLAUDE_ENV_SNAPSHOT_PATH".to_string(),
                snapshot_path.to_string_lossy().to_string(),
            ),
        ]
        .into_iter()
        .collect(),
        ..Default::default()
    });

    let handle = backend
        .run(AgentWrapperRunRequest {
            prompt: "hello".to_string(),
            ..Default::default()
        })
        .await
        .expect("run");

    let mut events = handle.events;
    let completion = handle.completion;

    let seen = drain_to_none(events.as_mut(), Duration::from_secs(2)).await;
    assert!(
        seen.iter()
            .any(|ev| ev.kind == AgentWrapperEventKind::Status),
        "expected at least one Status event"
    );

    let completion = tokio::time::timeout(Duration::from_secs(2), completion)
        .await
        .expect("completion resolves")
        .expect("completion ok");
    assert!(completion.status.success());

    let envs = read_env_snapshot(&snapshot_path);
    let layout = ClaudeHomeLayout::new(&home_root);

    assert_eq!(
        envs.get("CLAUDE_HOME").map(String::as_str),
        Some(layout.root().to_str().expect("utf-8 claude home root"))
    );
    assert_eq!(
        envs.get("HOME").map(String::as_str),
        Some(layout.root().to_str().expect("utf-8 home root"))
    );
    assert_eq!(
        envs.get("XDG_CONFIG_HOME").map(String::as_str),
        Some(
            layout
                .xdg_config_home()
                .to_str()
                .expect("utf-8 xdg config home")
        )
    );
    assert_eq!(
        envs.get("XDG_DATA_HOME").map(String::as_str),
        Some(
            layout
                .xdg_data_home()
                .to_str()
                .expect("utf-8 xdg data home")
        )
    );
    assert_eq!(
        envs.get("XDG_CACHE_HOME").map(String::as_str),
        Some(
            layout
                .xdg_cache_home()
                .to_str()
                .expect("utf-8 xdg cache home")
        )
    );

    assert!(
        layout.root().is_dir(),
        "expected isolated home root to exist"
    );
    assert!(
        layout.xdg_config_home().is_dir(),
        "expected XDG config home to exist"
    );
    assert!(
        layout.xdg_data_home().is_dir(),
        "expected XDG data home to exist"
    );
    assert!(
        layout.xdg_cache_home().is_dir(),
        "expected XDG cache home to exist"
    );
}

#[tokio::test]
async fn external_sandbox_allow_flag_supported_includes_allow_flag_and_emits_warning_before_handle_and_user_events(
) {
    let prompt = "hello";
    let log_path = unique_temp_log_path("claude_external_sandbox_allow_supported");

    let backend = ClaudeCodeBackend::new(ClaudeCodeBackendConfig {
        binary: Some(fake_claude_binary()),
        env: [
            (
                "FAKE_CLAUDE_SCENARIO".to_string(),
                "final_text_and_tools".to_string(),
            ),
            (
                "FAKE_CLAUDE_HELP_SUPPORTS_ALLOW_FLAG".to_string(),
                "1".to_string(),
            ),
            (
                "FAKE_CLAUDE_EXPECT_DANGEROUS_SKIP_PERMISSIONS".to_string(),
                "1".to_string(),
            ),
            (
                "FAKE_CLAUDE_EXPECT_ALLOW_DANGEROUSLY_SKIP_PERMISSIONS".to_string(),
                "1".to_string(),
            ),
            (
                "FAKE_CLAUDE_INVOCATION_LOG".to_string(),
                log_path.to_string_lossy().to_string(),
            ),
        ]
        .into_iter()
        .collect(),
        allow_external_sandbox_exec: true,
        ..Default::default()
    });

    let handle = backend
        .run(AgentWrapperRunRequest {
            prompt: prompt.to_string(),
            extensions: [(
                "agent_api.exec.external_sandbox.v1".to_string(),
                json!(true),
            )]
            .into_iter()
            .collect(),
            ..Default::default()
        })
        .await
        .expect("run");

    let mut events = handle.events;
    let completion = handle.completion;

    let seen = drain_to_none(events.as_mut(), Duration::from_secs(2)).await;

    let warnings = warning_indices(&seen);
    assert_eq!(
        warnings.len(),
        1,
        "expected exactly one pinned external sandbox warning Status event"
    );
    let warning_idx = warnings[0];

    let handle_idx = session_handle_facet_index(&seen)
        .expect("expected a Status event with the session handle facet");
    assert!(
        warning_idx < handle_idx,
        "expected warning to be emitted before the session handle facet Status event"
    );

    let first_user_visible = first_user_visible_index(&seen)
        .expect("expected at least one user-visible event (TextOutput/ToolCall/ToolResult)");
    assert!(
        warning_idx < first_user_visible,
        "expected warning to be emitted before any user-visible events"
    );

    let completion = tokio::time::timeout(Duration::from_secs(2), completion)
        .await
        .expect("completion resolves")
        .expect("completion ok");
    assert!(completion.status.success());

    let invocations = read_invocations(&log_path);
    assert_eq!(
        count(&invocations, "help"),
        1,
        "expected one help preflight"
    );
    assert_eq!(count(&invocations, "print"), 1, "expected one print spawn");
    let help_pos = first_index(&invocations, "help").unwrap();
    let print_pos = first_index(&invocations, "print").unwrap();
    assert!(
        help_pos < print_pos,
        "expected help preflight to occur before print spawn"
    );
}

#[tokio::test]
async fn external_sandbox_allow_flag_unsupported_excludes_allow_flag_and_emits_warning_before_handle_and_user_events(
) {
    let prompt = "hello";
    let log_path = unique_temp_log_path("claude_external_sandbox_allow_unsupported");

    let backend = ClaudeCodeBackend::new(ClaudeCodeBackendConfig {
        binary: Some(fake_claude_binary()),
        env: [
            (
                "FAKE_CLAUDE_SCENARIO".to_string(),
                "final_text_and_tools".to_string(),
            ),
            (
                "FAKE_CLAUDE_HELP_SUPPORTS_ALLOW_FLAG".to_string(),
                "0".to_string(),
            ),
            (
                "FAKE_CLAUDE_EXPECT_DANGEROUS_SKIP_PERMISSIONS".to_string(),
                "1".to_string(),
            ),
            (
                "FAKE_CLAUDE_EXPECT_ALLOW_DANGEROUSLY_SKIP_PERMISSIONS".to_string(),
                "0".to_string(),
            ),
            (
                "FAKE_CLAUDE_INVOCATION_LOG".to_string(),
                log_path.to_string_lossy().to_string(),
            ),
        ]
        .into_iter()
        .collect(),
        allow_external_sandbox_exec: true,
        ..Default::default()
    });

    let handle = backend
        .run(AgentWrapperRunRequest {
            prompt: prompt.to_string(),
            extensions: [(
                "agent_api.exec.external_sandbox.v1".to_string(),
                json!(true),
            )]
            .into_iter()
            .collect(),
            ..Default::default()
        })
        .await
        .expect("run");

    let mut events = handle.events;
    let completion = handle.completion;

    let seen = drain_to_none(events.as_mut(), Duration::from_secs(2)).await;

    let warnings = warning_indices(&seen);
    assert_eq!(
        warnings.len(),
        1,
        "expected exactly one pinned external sandbox warning Status event"
    );
    let warning_idx = warnings[0];

    let handle_idx = session_handle_facet_index(&seen)
        .expect("expected a Status event with the session handle facet");
    assert!(
        warning_idx < handle_idx,
        "expected warning to be emitted before the session handle facet Status event"
    );

    let first_user_visible = first_user_visible_index(&seen)
        .expect("expected at least one user-visible event (TextOutput/ToolCall/ToolResult)");
    assert!(
        warning_idx < first_user_visible,
        "expected warning to be emitted before any user-visible events"
    );

    let completion = tokio::time::timeout(Duration::from_secs(2), completion)
        .await
        .expect("completion resolves")
        .expect("completion ok");
    assert!(completion.status.success());

    let invocations = read_invocations(&log_path);
    assert_eq!(
        count(&invocations, "help"),
        1,
        "expected one help preflight"
    );
    assert_eq!(count(&invocations, "print"), 1, "expected one print spawn");
    let help_pos = first_index(&invocations, "help").unwrap();
    let print_pos = first_index(&invocations, "print").unwrap();
    assert!(
        help_pos < print_pos,
        "expected help preflight to occur before print spawn"
    );
}

#[tokio::test]
async fn external_sandbox_help_preflight_failure_returns_backend_error_before_print_and_redacts_output(
) {
    let secret = format!("S3C_SECRET_{}", std::process::id());
    let log_path = unique_temp_log_path("claude_external_sandbox_help_failure");

    let backend = ClaudeCodeBackend::new(ClaudeCodeBackendConfig {
        binary: Some(fake_claude_binary()),
        env: [
            ("FAKE_CLAUDE_HELP_FAIL".to_string(), "1".to_string()),
            ("FAKE_CLAUDE_HELP_FAIL_SECRET".to_string(), secret.clone()),
            (
                "FAKE_CLAUDE_PRINT_SHOULD_NOT_RUN".to_string(),
                "1".to_string(),
            ),
            (
                "FAKE_CLAUDE_INVOCATION_LOG".to_string(),
                log_path.to_string_lossy().to_string(),
            ),
        ]
        .into_iter()
        .collect(),
        allow_external_sandbox_exec: true,
        ..Default::default()
    });

    let handle = backend
        .run(AgentWrapperRunRequest {
            prompt: "hello".to_string(),
            extensions: [(
                "agent_api.exec.external_sandbox.v1".to_string(),
                json!(true),
            )]
            .into_iter()
            .collect(),
            ..Default::default()
        })
        .await
        .expect("run yields a handle; preflight failure is surfaced via events/completion");

    let mut events = handle.events;
    let completion = handle.completion;

    let seen = drain_to_none(events.as_mut(), Duration::from_secs(2)).await;
    let warnings = warning_indices(&seen);
    assert_eq!(
        warnings.len(),
        1,
        "expected exactly one pinned external sandbox warning Status event"
    );
    let warning_idx = warnings[0];
    let error_idx = first_error_index(&seen).expect("expected at least one Error event");
    assert!(
        warning_idx < error_idx,
        "expected warning to be emitted before the preflight failure Error event"
    );

    let error_messages: Vec<&str> = seen
        .iter()
        .filter(|ev| ev.kind == AgentWrapperEventKind::Error)
        .filter_map(|ev| ev.message.as_deref())
        .collect();
    assert!(
        !error_messages.is_empty(),
        "expected at least one Error event on preflight failure"
    );
    for message in error_messages {
        assert!(
            !message.contains(&secret),
            "expected redacted Error event message to not contain secret sentinel"
        );
    }

    let err = tokio::time::timeout(Duration::from_secs(2), completion)
        .await
        .expect("completion resolves")
        .expect_err("expected completion error on preflight failure");
    match err {
        AgentWrapperError::Backend { message } => {
            assert!(
                !message.contains(&secret),
                "expected redacted wrapper error message to not contain secret sentinel"
            );
        }
        other => panic!("expected AgentWrapperError::Backend, got: {other:?}"),
    }

    let invocations = read_invocations(&log_path);
    assert_eq!(
        count(&invocations, "help"),
        1,
        "expected one help preflight"
    );
    assert_eq!(
        count(&invocations, "print"),
        0,
        "expected no print spawn on preflight failure"
    );
}

#[tokio::test]
async fn external_sandbox_print_spawn_failure_after_cached_preflight_emits_warning_before_error() {
    let prompt = "hello";
    let log_path = unique_temp_log_path("claude_external_sandbox_print_spawn_failure");

    let backend = ClaudeCodeBackend::new(ClaudeCodeBackendConfig {
        binary: Some(fake_claude_binary()),
        env: [
            (
                "FAKE_CLAUDE_SCENARIO".to_string(),
                "final_text_and_tools".to_string(),
            ),
            (
                "FAKE_CLAUDE_HELP_SUPPORTS_ALLOW_FLAG".to_string(),
                "1".to_string(),
            ),
            (
                "FAKE_CLAUDE_EXPECT_DANGEROUS_SKIP_PERMISSIONS".to_string(),
                "1".to_string(),
            ),
            (
                "FAKE_CLAUDE_EXPECT_ALLOW_DANGEROUSLY_SKIP_PERMISSIONS".to_string(),
                "1".to_string(),
            ),
            (
                "FAKE_CLAUDE_INVOCATION_LOG".to_string(),
                log_path.to_string_lossy().to_string(),
            ),
        ]
        .into_iter()
        .collect(),
        allow_external_sandbox_exec: true,
        ..Default::default()
    });

    let warm_handle = backend
        .run(AgentWrapperRunRequest {
            prompt: prompt.to_string(),
            extensions: [(
                "agent_api.exec.external_sandbox.v1".to_string(),
                json!(true),
            )]
            .into_iter()
            .collect(),
            ..Default::default()
        })
        .await
        .expect("warm-up run");
    let mut warm_events = warm_handle.events;
    let warm_completion = warm_handle.completion;
    let _ = drain_to_none(warm_events.as_mut(), Duration::from_secs(2)).await;
    let warm_completion = tokio::time::timeout(Duration::from_secs(2), warm_completion)
        .await
        .expect("warm-up completion resolves")
        .expect("warm-up completion ok");
    assert!(warm_completion.status.success());

    let missing_dir = unique_missing_dir_path("claude_external_sandbox_print_spawn_failure");
    assert!(
        !missing_dir.exists(),
        "test requires a nonexistent working directory"
    );

    let handle = backend
        .run(AgentWrapperRunRequest {
            prompt: prompt.to_string(),
            working_dir: Some(missing_dir),
            extensions: [(
                "agent_api.exec.external_sandbox.v1".to_string(),
                json!(true),
            )]
            .into_iter()
            .collect(),
            ..Default::default()
        })
        .await
        .expect("run yields a handle; spawn failure is surfaced via events/completion");

    let mut events = handle.events;
    let completion = handle.completion;

    let seen = drain_to_none(events.as_mut(), Duration::from_secs(2)).await;
    let warnings = warning_indices(&seen);
    assert_eq!(
        warnings.len(),
        1,
        "expected exactly one pinned external sandbox warning Status event"
    );
    let warning_idx = warnings[0];
    let error_idx = first_error_index(&seen).expect("expected at least one Error event");
    assert!(
        warning_idx < error_idx,
        "expected warning to be emitted before the spawn failure Error event"
    );

    let err = tokio::time::timeout(Duration::from_secs(2), completion)
        .await
        .expect("completion resolves")
        .expect_err("expected completion error on spawn failure");
    match err {
        AgentWrapperError::Backend { .. } => {}
        other => panic!("expected AgentWrapperError::Backend, got: {other:?}"),
    }

    let invocations = read_invocations(&log_path);
    assert_eq!(
        count(&invocations, "help"),
        1,
        "expected cached preflight support to avoid a second help invocation"
    );
    assert_eq!(
        count(&invocations, "print"),
        1,
        "expected the failing run to abort before the print command is invoked"
    );
}

#[tokio::test]
async fn external_sandbox_help_preflight_is_cached_per_backend_instance() {
    let prompt = "hello";
    let log_path = unique_temp_log_path("claude_external_sandbox_help_cached");

    let backend = ClaudeCodeBackend::new(ClaudeCodeBackendConfig {
        binary: Some(fake_claude_binary()),
        env: [
            (
                "FAKE_CLAUDE_SCENARIO".to_string(),
                "final_text_and_tools".to_string(),
            ),
            (
                "FAKE_CLAUDE_HELP_SUPPORTS_ALLOW_FLAG".to_string(),
                "1".to_string(),
            ),
            (
                "FAKE_CLAUDE_EXPECT_DANGEROUS_SKIP_PERMISSIONS".to_string(),
                "1".to_string(),
            ),
            (
                "FAKE_CLAUDE_EXPECT_ALLOW_DANGEROUSLY_SKIP_PERMISSIONS".to_string(),
                "1".to_string(),
            ),
            (
                "FAKE_CLAUDE_INVOCATION_LOG".to_string(),
                log_path.to_string_lossy().to_string(),
            ),
        ]
        .into_iter()
        .collect(),
        allow_external_sandbox_exec: true,
        ..Default::default()
    });

    for run_idx in 0..2 {
        let handle = backend
            .run(AgentWrapperRunRequest {
                prompt: prompt.to_string(),
                extensions: [(
                    "agent_api.exec.external_sandbox.v1".to_string(),
                    json!(true),
                )]
                .into_iter()
                .collect(),
                ..Default::default()
            })
            .await
            .expect("run");

        let mut events = handle.events;
        let completion = handle.completion;

        let seen = drain_to_none(events.as_mut(), Duration::from_secs(2)).await;
        let warnings = warning_indices(&seen);
        assert_eq!(
            warnings.len(),
            1,
            "run {run_idx}: expected exactly one pinned external sandbox warning Status event"
        );

        let completion = tokio::time::timeout(Duration::from_secs(2), completion)
            .await
            .expect("completion resolves")
            .expect("completion ok");
        assert!(completion.status.success());
    }

    let invocations = read_invocations(&log_path);
    assert_eq!(
        count(&invocations, "help"),
        1,
        "expected cached help preflight"
    );
    assert_eq!(count(&invocations, "print"), 2, "expected two print spawns");
}
