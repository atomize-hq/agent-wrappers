#![cfg(feature = "codex")]

use std::{collections::BTreeMap, path::PathBuf, pin::Pin, time::Duration};

use agent_api::{
    backends::codex::{CodexBackend, CodexBackendConfig},
    AgentWrapperBackend, AgentWrapperEvent, AgentWrapperEventKind, AgentWrapperRunRequest,
};
use futures_core::Stream;

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
    PathBuf::from(env!("CARGO_BIN_EXE_fake_codex_stream_json_agent_api"))
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

#[tokio::test]
async fn codex_backend_defaults_to_non_interactive_and_workspace_write_sandbox() {
    let backend = CodexBackend::new(CodexBackendConfig {
        binary: Some(fake_codex_binary()),
        codex_home: None,
        default_timeout: None,
        default_working_dir: None,
        env: base_env(),
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
        codex_home: None,
        default_timeout: None,
        default_working_dir: None,
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
        codex_home: None,
        default_timeout: None,
        default_working_dir: None,
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
