#![cfg(feature = "codex")]

use std::{collections::BTreeMap, fs, path::PathBuf, pin::Pin, time::Duration};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(unix)]
use std::path::Path;

use agent_api::{
    backends::codex::{CodexBackend, CodexBackendConfig},
    AgentWrapperBackend, AgentWrapperError, AgentWrapperEvent, AgentWrapperEventKind,
    AgentWrapperRunRequest,
};
use futures_core::Stream;
use serde_json::json;
use tempfile::tempdir;

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

fn fake_codex_binary() -> PathBuf {
    PathBuf::from(env!(
        "CARGO_BIN_EXE_fake_codex_stream_exec_scenarios_agent_api"
    ))
}

#[cfg(unix)]
#[derive(Copy, Clone)]
enum AddDirProbeMode {
    Unsupported,
}

#[cfg(unix)]
fn write_probe_only_codex(dir: &Path, mode: AddDirProbeMode, exec_log_path: &Path) -> PathBuf {
    let (json_probe, text_probe, help_output) = match mode {
        AddDirProbeMode::Unsupported => (
            r#"echo '{"features":["output_schema"]}'"#,
            r#"echo "output_schema""#,
            r#"echo "Usage: codex exec""#,
        ),
    };
    let script = format!(
        r#"#!/bin/bash
log="{log}"
if [[ "$1" == "--version" ]]; then
  echo "codex 0.9.0"
elif [[ "$1" == "features" && "$2" == "list" && "$3" == "--json" ]]; then
  {json_probe}
elif [[ "$1" == "features" && "$2" == "list" ]]; then
  {text_probe}
elif [[ "$1" == "--help" ]]; then
  {help_output}
elif [[ "$1" == "exec" ]]; then
  echo "$@" >> "$log"
  exit 99
fi
"#,
        log = exec_log_path.display(),
        json_probe = json_probe,
        text_probe = text_probe,
        help_output = help_output,
    );
    let binary = dir.join("fake-probe-only-codex.sh");
    fs::write(&binary, script).expect("write fake codex script");
    let mut perms = fs::metadata(&binary).expect("metadata").permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&binary, perms).expect("set permissions");
    binary
}

fn base_env() -> BTreeMap<String, String> {
    [
        (
            "FAKE_CODEX_EXPECT_SANDBOX".to_string(),
            "workspace-write".to_string(),
        ),
        (
            "FAKE_CODEX_EXPECT_APPROVAL".to_string(),
            "never".to_string(),
        ),
    ]
    .into_iter()
    .collect()
}

fn add_dir_expectations(dirs: &[PathBuf]) -> BTreeMap<String, String> {
    let mut env = BTreeMap::from([(
        "FAKE_CODEX_EXPECT_ADD_DIR_COUNT".to_string(),
        dirs.len().to_string(),
    )]);
    for (index, dir) in dirs.iter().enumerate() {
        env.insert(
            format!("FAKE_CODEX_EXPECT_ADD_DIR_{index}"),
            dir.display().to_string(),
        );
    }
    env
}

fn model_expectations(model: &str) -> BTreeMap<String, String> {
    [("FAKE_CODEX_EXPECT_MODEL".to_string(), model.to_string())]
        .into_iter()
        .collect()
}

fn handle_facet_index(events: &[AgentWrapperEvent]) -> Option<usize> {
    events.iter().position(|event| {
        event.kind == AgentWrapperEventKind::Status
            && event
                .data
                .as_ref()
                .and_then(|data| data.get("schema"))
                .and_then(serde_json::Value::as_str)
                == Some("agent_api.session.handle.v1")
    })
}

const EXTERNAL_SANDBOX_WARNING: &str =
    "DANGEROUS: external sandbox exec policy enabled (agent_api.exec.external_sandbox.v1=true)";

#[tokio::test]
async fn codex_backend_defaults_to_non_interactive_and_workspace_write_sandbox() {
    let backend = CodexBackend::new(CodexBackendConfig {
        binary: Some(fake_codex_binary()),
        env: base_env(),
        ..Default::default()
    });

    let handle = backend
        .run(AgentWrapperRunRequest {
            prompt: "hello".to_string(),
            ..Default::default()
        })
        .await
        .unwrap();

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
        .unwrap();
    assert!(completion.status.success());
}

#[tokio::test]
async fn sandbox_mode_extension_overrides_codex_sandbox() {
    let backend = CodexBackend::new(CodexBackendConfig {
        binary: Some(fake_codex_binary()),
        env: [
            (
                "FAKE_CODEX_EXPECT_SANDBOX".to_string(),
                "danger-full-access".to_string(),
            ),
            (
                "FAKE_CODEX_EXPECT_APPROVAL".to_string(),
                "never".to_string(),
            ),
        ]
        .into_iter()
        .collect(),
        ..Default::default()
    });

    let mut extensions = BTreeMap::new();
    extensions.insert(
        "backend.codex.exec.sandbox_mode".to_string(),
        serde_json::Value::String("danger-full-access".to_string()),
    );

    let handle = backend
        .run(AgentWrapperRunRequest {
            prompt: "hello".to_string(),
            extensions,
            ..Default::default()
        })
        .await
        .unwrap();

    let mut events = handle.events;
    let completion = handle.completion;
    let _ = drain_to_none(events.as_mut(), Duration::from_secs(2)).await;

    let completion = tokio::time::timeout(Duration::from_secs(2), completion)
        .await
        .expect("completion resolves")
        .unwrap();
    assert!(completion.status.success());
}

#[tokio::test]
async fn non_interactive_false_does_not_force_ask_for_approval() {
    let backend = CodexBackend::new(CodexBackendConfig {
        binary: Some(fake_codex_binary()),
        env: [
            (
                "FAKE_CODEX_EXPECT_SANDBOX".to_string(),
                "workspace-write".to_string(),
            ),
            (
                "FAKE_CODEX_EXPECT_APPROVAL".to_string(),
                "<absent>".to_string(),
            ),
        ]
        .into_iter()
        .collect(),
        ..Default::default()
    });

    let mut extensions = BTreeMap::new();
    extensions.insert(
        "agent_api.exec.non_interactive".to_string(),
        serde_json::Value::Bool(false),
    );

    let handle = backend
        .run(AgentWrapperRunRequest {
            prompt: "hello".to_string(),
            extensions,
            ..Default::default()
        })
        .await
        .unwrap();

    let mut events = handle.events;
    let completion = handle.completion;
    let seen = drain_to_none(events.as_mut(), Duration::from_secs(2)).await;
    assert!(
        seen.iter()
            .any(|ev| ev.kind == AgentWrapperEventKind::Status),
        "expected status events even when interactive"
    );

    let completion = tokio::time::timeout(Duration::from_secs(2), completion)
        .await
        .expect("completion resolves")
        .unwrap();
    assert!(completion.status.success());
}

#[tokio::test]
async fn codex_external_sandbox_exec_maps_to_dangerous_bypass_argv_and_emits_warning_before_handle_facet(
) {
    let backend = CodexBackend::new(CodexBackendConfig {
        allow_external_sandbox_exec: true,
        binary: Some(fake_codex_binary()),
        env: [
            ("FAKE_CODEX_SCENARIO".to_string(), "ok".to_string()),
            (
                "FAKE_CODEX_EXPECT_DANGEROUS_BYPASS".to_string(),
                "1".to_string(),
            ),
        ]
        .into_iter()
        .collect(),
        ..Default::default()
    });

    let mut extensions = BTreeMap::new();
    extensions.insert(
        "agent_api.exec.external_sandbox.v1".to_string(),
        serde_json::Value::Bool(true),
    );

    let handle = backend
        .run(AgentWrapperRunRequest {
            prompt: "hello".to_string(),
            extensions,
            ..Default::default()
        })
        .await
        .unwrap();

    let mut events = handle.events;
    let completion = handle.completion;

    let seen = drain_to_none(events.as_mut(), Duration::from_secs(2)).await;

    let mut warning_idx: Option<usize> = None;
    for (idx, event) in seen.iter().enumerate() {
        if event.kind == AgentWrapperEventKind::Status
            && event.message.as_deref() == Some(EXTERNAL_SANDBOX_WARNING)
        {
            assert!(
                warning_idx.is_none(),
                "expected exactly one warning Status event"
            );
            warning_idx = Some(idx);
            assert_eq!(event.channel.as_deref(), Some("status"));
            assert_eq!(event.data, None);
        }
    }
    let warning_idx = warning_idx.expect("expected warning Status event");

    let handle_idx = seen
        .iter()
        .enumerate()
        .find(|(_, event)| {
            event.kind == AgentWrapperEventKind::Status
                && event
                    .data
                    .as_ref()
                    .and_then(|data| data.get("schema"))
                    .and_then(serde_json::Value::as_str)
                    == Some("agent_api.session.handle.v1")
        })
        .map(|(idx, _)| idx)
        .expect("expected session handle facet Status event");

    assert!(
        warning_idx < handle_idx,
        "expected warning to be emitted before the session handle facet Status event"
    );

    let completion = tokio::time::timeout(Duration::from_secs(2), completion)
        .await
        .expect("completion resolves")
        .unwrap();
    assert!(completion.status.success());
}

#[tokio::test]
async fn codex_exec_without_add_dirs_emits_no_add_dir_flags() {
    let backend = CodexBackend::new(CodexBackendConfig {
        binary: Some(fake_codex_binary()),
        env: base_env()
            .into_iter()
            .chain(add_dir_expectations(&[]))
            .collect(),
        ..Default::default()
    });

    let handle = backend
        .run(AgentWrapperRunRequest {
            prompt: "hello".to_string(),
            ..Default::default()
        })
        .await
        .unwrap();

    let mut events = handle.events;
    let completion = handle.completion;
    let _ = drain_to_none(events.as_mut(), Duration::from_secs(2)).await;

    let completion = tokio::time::timeout(Duration::from_secs(2), completion)
        .await
        .expect("completion resolves")
        .unwrap();
    assert!(completion.status.success());
}

#[tokio::test]
async fn codex_exec_with_add_dirs_emits_repeated_flags_in_order() {
    let temp = tempdir().expect("tempdir");
    let dir_a = temp.path().join("alpha");
    let dir_b = temp.path().join("beta");
    fs::create_dir_all(&dir_a).expect("alpha dir");
    fs::create_dir_all(&dir_b).expect("beta dir");
    let add_dirs = vec![dir_a, dir_b];

    let backend = CodexBackend::new(CodexBackendConfig {
        binary: Some(fake_codex_binary()),
        env: base_env()
            .into_iter()
            .chain(add_dir_expectations(&add_dirs))
            .collect(),
        ..Default::default()
    });

    let handle = backend
        .run(AgentWrapperRunRequest {
            prompt: "hello".to_string(),
            extensions: [(
                "agent_api.exec.add_dirs.v1".to_string(),
                json!({"dirs": add_dirs.iter().map(|dir| dir.display().to_string()).collect::<Vec<_>>() }),
            )]
            .into_iter()
            .collect(),
            ..Default::default()
        })
        .await
        .unwrap();

    let mut events = handle.events;
    let completion = handle.completion;
    let _ = drain_to_none(events.as_mut(), Duration::from_secs(2)).await;

    let completion = tokio::time::timeout(Duration::from_secs(2), completion)
        .await
        .expect("completion resolves")
        .unwrap();
    assert!(completion.status.success());
}

#[cfg(unix)]
#[tokio::test]
async fn codex_exec_with_add_dirs_rejects_before_spawn_when_probe_cannot_confirm_support() {
    let temp = tempdir().expect("tempdir");
    let dir_a = temp.path().join("alpha");
    let dir_b = temp.path().join("beta");
    let exec_log = temp.path().join("exec.log");
    fs::create_dir_all(&dir_a).expect("alpha dir");
    fs::create_dir_all(&dir_b).expect("beta dir");
    let add_dirs = vec![dir_a, dir_b];
    let binary = write_probe_only_codex(temp.path(), AddDirProbeMode::Unsupported, &exec_log);

    let backend = CodexBackend::new(CodexBackendConfig {
        binary: Some(binary),
        env: base_env(),
        ..Default::default()
    });

    let handle = backend
        .run(AgentWrapperRunRequest {
            prompt: "hello".to_string(),
            extensions: [(
                "agent_api.exec.add_dirs.v1".to_string(),
                json!({"dirs": add_dirs.iter().map(|dir| dir.display().to_string()).collect::<Vec<_>>() }),
            )]
            .into_iter()
            .collect(),
            ..Default::default()
        })
        .await
        .expect("preflight rejection still returns a terminal handle");

    let mut events = handle.events;
    let seen = drain_to_none(events.as_mut(), Duration::from_secs(2)).await;
    assert_eq!(seen.len(), 1, "expected exactly one terminal error event");
    assert_eq!(seen[0].kind, AgentWrapperEventKind::Error);
    assert_eq!(
        seen[0].message.as_deref(),
        Some("add_dirs rejected by runtime")
    );

    let err = tokio::time::timeout(Duration::from_secs(2), handle.completion)
        .await
        .expect("completion resolves")
        .unwrap_err();
    match err {
        AgentWrapperError::Backend { message } => {
            assert_eq!(message, "add_dirs rejected by runtime");
        }
        other => panic!("expected Backend error, got: {other:?}"),
    }
    assert!(
        !exec_log.exists(),
        "preflight rejection should not invoke codex exec"
    );
}

#[tokio::test]
async fn codex_exec_with_model_emits_model_before_add_dirs() {
    let model = "gpt-5-codex";
    let temp = tempdir().expect("tempdir");
    let dir_a = temp.path().join("alpha");
    let dir_b = temp.path().join("beta");
    fs::create_dir_all(&dir_a).expect("alpha dir");
    fs::create_dir_all(&dir_b).expect("beta dir");
    let add_dirs = vec![dir_a, dir_b];

    let backend = CodexBackend::new(CodexBackendConfig {
        binary: Some(fake_codex_binary()),
        model: Some(model.to_string()),
        env: base_env()
            .into_iter()
            .chain(add_dir_expectations(&add_dirs))
            .chain(model_expectations(model))
            .collect(),
        ..Default::default()
    });

    let handle = backend
        .run(AgentWrapperRunRequest {
            prompt: "hello".to_string(),
            extensions: [(
                "agent_api.exec.add_dirs.v1".to_string(),
                json!({"dirs": add_dirs.iter().map(|dir| dir.display().to_string()).collect::<Vec<_>>() }),
            )]
            .into_iter()
            .collect(),
            ..Default::default()
        })
        .await
        .unwrap();

    let mut events = handle.events;
    let completion = handle.completion;
    let _ = drain_to_none(events.as_mut(), Duration::from_secs(2)).await;

    let completion = tokio::time::timeout(Duration::from_secs(2), completion)
        .await
        .expect("completion resolves")
        .unwrap();
    assert!(completion.status.success());
}

#[tokio::test]
async fn codex_exec_add_dirs_runtime_rejection_emits_handle_before_backend_error() {
    let temp = tempdir().expect("tempdir");
    let dir_a = temp.path().join("alpha");
    let dir_b = temp.path().join("beta");
    fs::create_dir_all(&dir_a).expect("alpha dir");
    fs::create_dir_all(&dir_b).expect("beta dir");
    let add_dirs = vec![dir_a, dir_b];

    let backend = CodexBackend::new(CodexBackendConfig {
        binary: Some(fake_codex_binary()),
        env: base_env()
            .into_iter()
            .chain(add_dir_expectations(&add_dirs))
            .chain([(
                "FAKE_CODEX_SCENARIO".to_string(),
                "add_dirs_runtime_rejection_exec".to_string(),
            )])
            .collect(),
        ..Default::default()
    });

    let handle = backend
        .run(AgentWrapperRunRequest {
            prompt: "hello".to_string(),
            extensions: [(
                "agent_api.exec.add_dirs.v1".to_string(),
                json!({"dirs": add_dirs.iter().map(|dir| dir.display().to_string()).collect::<Vec<_>>() }),
            )]
            .into_iter()
            .collect(),
            ..Default::default()
        })
        .await
        .unwrap();

    let mut events = handle.events;
    let completion = handle.completion;

    let seen = drain_to_none(events.as_mut(), Duration::from_secs(2)).await;
    let handle_idx = handle_facet_index(&seen).expect("expected session handle facet");
    let error_idx = seen
        .iter()
        .position(|event| event.kind == AgentWrapperEventKind::Error)
        .expect("expected backend error event");
    assert!(
        handle_idx < error_idx,
        "expected handle facet status event before backend error"
    );
    assert_eq!(
        seen[error_idx].message.as_deref(),
        Some("add_dirs rejected by runtime")
    );

    let err = tokio::time::timeout(Duration::from_secs(2), completion)
        .await
        .expect("completion resolves")
        .unwrap_err();
    match err {
        AgentWrapperError::Backend { message } => {
            assert_eq!(message, "add_dirs rejected by runtime");
        }
        other => panic!("expected Backend error, got: {other:?}"),
    }
}
