use std::path::PathBuf;

use agent_api::{
    backends::codex::CodexBackendConfig, AgentWrapperBackend, AgentWrapperError,
    AgentWrapperEventKind,
};
use serde_json::json;

use super::support::{
    assert_backend_error_message, base_env, build_backend, drain_to_none, run_request,
    STREAM_TIMEOUT,
};

#[tokio::test]
async fn invalid_resume_schema_is_rejected_pre_spawn() {
    let backend = agent_api::backends::codex::CodexBackend::new(CodexBackendConfig {
        binary: Some(PathBuf::from("definitely-not-a-real-codex-binary")),
        ..Default::default()
    });

    let err = backend
        .run(run_request(
            "hello",
            [(
                "agent_api.session.resume.v1".to_string(),
                json!("not an object"),
            )],
        ))
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

    let backend = build_backend(env, None, false);
    let handle = backend
        .run(run_request(
            prompt,
            [(
                "agent_api.session.resume.v1".to_string(),
                json!({"selector": "last"}),
            )],
        ))
        .await
        .unwrap();

    let mut events = handle.events;
    let seen = drain_to_none(events.as_mut(), STREAM_TIMEOUT).await;
    let error_events: Vec<_> = seen
        .iter()
        .filter(|event| event.kind == AgentWrapperEventKind::Error)
        .collect();
    assert_eq!(error_events.len(), 1, "expected exactly one Error event");
    assert_eq!(
        seen.last().map(|event| event.kind.clone()),
        Some(AgentWrapperEventKind::Error),
        "expected Error event to be terminal"
    );
    assert_eq!(
        seen.last().and_then(|event| event.message.as_deref()),
        Some("no session found")
    );

    assert_backend_error_message(handle.completion, "no session found").await;
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

    let backend = build_backend(env, None, false);
    let handle = backend
        .run(run_request(
            prompt,
            [(
                "agent_api.session.resume.v1".to_string(),
                json!({"selector": "id", "id": resume_id}),
            )],
        ))
        .await
        .unwrap();

    let mut events = handle.events;
    let seen = drain_to_none(events.as_mut(), STREAM_TIMEOUT).await;
    let error_events: Vec<_> = seen
        .iter()
        .filter(|event| event.kind == AgentWrapperEventKind::Error)
        .collect();
    assert_eq!(error_events.len(), 1, "expected exactly one Error event");
    assert_eq!(
        seen.last().map(|event| event.kind.clone()),
        Some(AgentWrapperEventKind::Error),
        "expected Error event to be terminal"
    );
    assert_eq!(
        seen.last().and_then(|event| event.message.as_deref()),
        Some("session not found")
    );

    assert_backend_error_message(handle.completion, "session not found").await;
}
