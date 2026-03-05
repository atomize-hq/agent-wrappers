#![cfg(feature = "codex")]

use std::{collections::BTreeMap, path::PathBuf, pin::Pin, time::Duration};

use agent_api::{
    backends::codex::{CodexBackend, CodexBackendConfig},
    AgentWrapperBackend, AgentWrapperError, AgentWrapperEvent, AgentWrapperEventKind,
    AgentWrapperRunRequest,
};
use futures_core::Stream;
use serde_json::json;

const EXTERNAL_SANDBOX_WARNING: &str =
    "DANGEROUS: external sandbox exec policy enabled (agent_api.exec.external_sandbox.v1=true)";

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

fn assert_external_sandbox_warning_before_session_handle_facet(seen: &[AgentWrapperEvent]) {
    let mut warning_idx: Option<usize> = None;
    for (idx, event) in seen.iter().enumerate() {
        if event.kind == AgentWrapperEventKind::Status
            && event.message.as_deref() == Some(EXTERNAL_SANDBOX_WARNING)
        {
            assert!(
                warning_idx.is_none(),
                "expected exactly one external sandbox warning Status event"
            );
            warning_idx = Some(idx);
            assert_eq!(event.channel.as_deref(), Some("status"));
            assert_eq!(event.data, None);
        }
    }
    let warning_idx = warning_idx.expect("expected external sandbox warning Status event");

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
}

#[tokio::test]
async fn resume_last_maps_to_exec_json_resume_last_dash_and_stdin_prompt() {
    let prompt = "hello world";
    let mut env = base_env();
    env.insert(
        "FAKE_CODEX_SCENARIO".to_string(),
        "resume_last_assert".to_string(),
    );
    env.insert("FAKE_CODEX_EXPECT_PROMPT".to_string(), prompt.to_string());

    let backend = CodexBackend::new(CodexBackendConfig {
        binary: Some(fake_codex_binary()),
        env,
        ..Default::default()
    });

    let handle = backend
        .run(AgentWrapperRunRequest {
            prompt: prompt.to_string(),
            extensions: [(
                "agent_api.session.resume.v1".to_string(),
                json!({"selector": "last"}),
            )]
            .into_iter()
            .collect(),
            ..Default::default()
        })
        .await
        .unwrap();

    let mut events = handle.events;
    let completion = handle.completion;

    let _seen = drain_to_none(events.as_mut(), Duration::from_secs(2)).await;

    let completion = tokio::time::timeout(Duration::from_secs(2), completion)
        .await
        .expect("completion resolves")
        .unwrap();
    assert!(completion.status.success());
}

#[tokio::test]
async fn resume_id_maps_to_exec_json_resume_id_dash_and_stdin_prompt() {
    let prompt = "hello world";
    let resume_id = "thread-123";

    let mut env = base_env();
    env.insert(
        "FAKE_CODEX_SCENARIO".to_string(),
        "resume_id_assert".to_string(),
    );
    env.insert("FAKE_CODEX_EXPECT_PROMPT".to_string(), prompt.to_string());
    env.insert(
        "FAKE_CODEX_EXPECT_RESUME_ID".to_string(),
        resume_id.to_string(),
    );

    let backend = CodexBackend::new(CodexBackendConfig {
        binary: Some(fake_codex_binary()),
        env,
        ..Default::default()
    });

    let handle = backend
        .run(AgentWrapperRunRequest {
            prompt: prompt.to_string(),
            extensions: [(
                "agent_api.session.resume.v1".to_string(),
                json!({"selector": "id", "id": resume_id}),
            )]
            .into_iter()
            .collect(),
            ..Default::default()
        })
        .await
        .unwrap();

    let mut events = handle.events;
    let completion = handle.completion;

    let _seen = drain_to_none(events.as_mut(), Duration::from_secs(2)).await;

    let completion = tokio::time::timeout(Duration::from_secs(2), completion)
        .await
        .expect("completion resolves")
        .unwrap();
    assert!(completion.status.success());
}

#[tokio::test]
async fn invalid_resume_schema_is_rejected_pre_spawn() {
    let backend = CodexBackend::new(CodexBackendConfig {
        binary: Some(PathBuf::from("definitely-not-a-real-codex-binary")),
        ..Default::default()
    });

    let err = backend
        .run(AgentWrapperRunRequest {
            prompt: "hello".to_string(),
            extensions: [(
                "agent_api.session.resume.v1".to_string(),
                json!("not an object"),
            )]
            .into_iter()
            .collect(),
            ..Default::default()
        })
        .await
        .unwrap_err();

    assert!(matches!(err, AgentWrapperError::InvalidRequest { .. }));
}

#[tokio::test]
async fn resume_last_selection_failure_is_translated_and_emits_one_terminal_error_event() {
    let prompt = "hello world";
    let mut env = base_env();
    env.insert(
        "FAKE_CODEX_SCENARIO".to_string(),
        "resume_last_not_found".to_string(),
    );
    env.insert("FAKE_CODEX_EXPECT_PROMPT".to_string(), prompt.to_string());

    let backend = CodexBackend::new(CodexBackendConfig {
        binary: Some(fake_codex_binary()),
        env,
        ..Default::default()
    });

    let handle = backend
        .run(AgentWrapperRunRequest {
            prompt: prompt.to_string(),
            extensions: [(
                "agent_api.session.resume.v1".to_string(),
                json!({"selector": "last"}),
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
    let error_events: Vec<_> = seen
        .iter()
        .filter(|ev| ev.kind == AgentWrapperEventKind::Error)
        .collect();
    assert_eq!(error_events.len(), 1, "expected exactly one Error event");
    assert_eq!(
        seen.last().map(|ev| ev.kind.clone()),
        Some(AgentWrapperEventKind::Error),
        "expected Error event to be terminal"
    );
    assert_eq!(
        seen.last().and_then(|ev| ev.message.as_deref()),
        Some("no session found")
    );

    let err = tokio::time::timeout(Duration::from_secs(2), completion)
        .await
        .expect("completion resolves")
        .unwrap_err();
    match err {
        AgentWrapperError::Backend { message } => assert_eq!(message, "no session found"),
        other => panic!("expected Backend error, got: {other:?}"),
    }
}

#[tokio::test]
async fn resume_id_selection_failure_is_translated_and_emits_one_terminal_error_event() {
    let prompt = "hello world";
    let resume_id = "thread-123";

    let mut env = base_env();
    env.insert(
        "FAKE_CODEX_SCENARIO".to_string(),
        "resume_id_not_found".to_string(),
    );
    env.insert("FAKE_CODEX_EXPECT_PROMPT".to_string(), prompt.to_string());
    env.insert(
        "FAKE_CODEX_EXPECT_RESUME_ID".to_string(),
        resume_id.to_string(),
    );

    let backend = CodexBackend::new(CodexBackendConfig {
        binary: Some(fake_codex_binary()),
        env,
        ..Default::default()
    });

    let handle = backend
        .run(AgentWrapperRunRequest {
            prompt: prompt.to_string(),
            extensions: [(
                "agent_api.session.resume.v1".to_string(),
                json!({"selector": "id", "id": resume_id}),
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
    let error_events: Vec<_> = seen
        .iter()
        .filter(|ev| ev.kind == AgentWrapperEventKind::Error)
        .collect();
    assert_eq!(error_events.len(), 1, "expected exactly one Error event");
    assert_eq!(
        seen.last().map(|ev| ev.kind.clone()),
        Some(AgentWrapperEventKind::Error),
        "expected Error event to be terminal"
    );
    assert_eq!(
        seen.last().and_then(|ev| ev.message.as_deref()),
        Some("session not found")
    );

    let err = tokio::time::timeout(Duration::from_secs(2), completion)
        .await
        .expect("completion resolves")
        .unwrap_err();
    match err {
        AgentWrapperError::Backend { message } => assert_eq!(message, "session not found"),
        other => panic!("expected Backend error, got: {other:?}"),
    }
}

#[tokio::test]
async fn codex_resume_last_external_sandbox_maps_to_dangerous_bypass_argv_and_emits_warning_before_handle_facet(
) {
    let prompt = "hello world";
    let env: BTreeMap<String, String> = [
        (
            "FAKE_CODEX_SCENARIO".to_string(),
            "resume_last_assert".to_string(),
        ),
        ("FAKE_CODEX_EXPECT_PROMPT".to_string(), prompt.to_string()),
        (
            "FAKE_CODEX_EXPECT_DANGEROUS_BYPASS".to_string(),
            "1".to_string(),
        ),
    ]
    .into_iter()
    .collect();

    let backend = CodexBackend::new(CodexBackendConfig {
        allow_external_sandbox_exec: true,
        binary: Some(fake_codex_binary()),
        env,
        ..Default::default()
    });

    let handle = backend
        .run(AgentWrapperRunRequest {
            prompt: prompt.to_string(),
            extensions: [
                (
                    "agent_api.exec.external_sandbox.v1".to_string(),
                    json!(true),
                ),
                (
                    "agent_api.session.resume.v1".to_string(),
                    json!({"selector": "last"}),
                ),
            ]
            .into_iter()
            .collect(),
            ..Default::default()
        })
        .await
        .unwrap();

    let mut events = handle.events;
    let completion = handle.completion;

    let seen = drain_to_none(events.as_mut(), Duration::from_secs(2)).await;
    assert_external_sandbox_warning_before_session_handle_facet(&seen);

    let completion = tokio::time::timeout(Duration::from_secs(2), completion)
        .await
        .expect("completion resolves")
        .unwrap();
    assert!(completion.status.success());
}

#[tokio::test]
async fn codex_resume_id_external_sandbox_maps_to_dangerous_bypass_argv_and_emits_warning_before_handle_facet(
) {
    let prompt = "hello world";
    let resume_id = "thread-123";

    let env: BTreeMap<String, String> = [
        (
            "FAKE_CODEX_SCENARIO".to_string(),
            "resume_id_assert".to_string(),
        ),
        ("FAKE_CODEX_EXPECT_PROMPT".to_string(), prompt.to_string()),
        (
            "FAKE_CODEX_EXPECT_RESUME_ID".to_string(),
            resume_id.to_string(),
        ),
        (
            "FAKE_CODEX_EXPECT_DANGEROUS_BYPASS".to_string(),
            "1".to_string(),
        ),
    ]
    .into_iter()
    .collect();

    let backend = CodexBackend::new(CodexBackendConfig {
        allow_external_sandbox_exec: true,
        binary: Some(fake_codex_binary()),
        env,
        ..Default::default()
    });

    let handle = backend
        .run(AgentWrapperRunRequest {
            prompt: prompt.to_string(),
            extensions: [
                (
                    "agent_api.exec.external_sandbox.v1".to_string(),
                    json!(true),
                ),
                (
                    "agent_api.session.resume.v1".to_string(),
                    json!({"selector": "id", "id": resume_id}),
                ),
            ]
            .into_iter()
            .collect(),
            ..Default::default()
        })
        .await
        .unwrap();

    let mut events = handle.events;
    let completion = handle.completion;

    let seen = drain_to_none(events.as_mut(), Duration::from_secs(2)).await;
    assert_external_sandbox_warning_before_session_handle_facet(&seen);

    let completion = tokio::time::timeout(Duration::from_secs(2), completion)
        .await
        .expect("completion resolves")
        .unwrap();
    assert!(completion.status.success());
}
