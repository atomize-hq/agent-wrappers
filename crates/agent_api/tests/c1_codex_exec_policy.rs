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
    PathBuf::from(env!(
        "CARGO_BIN_EXE_fake_codex_stream_exec_scenarios_agent_api"
    ))
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
