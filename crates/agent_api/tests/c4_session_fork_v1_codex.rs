#![cfg(feature = "codex")]

use std::{path::PathBuf, pin::Pin, time::Duration};

use agent_api::{
    backends::codex::{CodexBackend, CodexBackendConfig},
    AgentWrapperBackend, AgentWrapperError, AgentWrapperEvent, AgentWrapperEventKind,
    AgentWrapperGateway, AgentWrapperKind, AgentWrapperRunRequest,
};
use futures_core::Stream;
use serde_json::json;

fn fake_codex_app_server_binary() -> PathBuf {
    PathBuf::from(env!(
        "CARGO_BIN_EXE_fake_codex_app_server_jsonrpc_agent_api"
    ))
}

fn make_temp_working_dir() -> PathBuf {
    let mut path = std::env::temp_dir();
    let unique = format!(
        "agent_api-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    );
    path.push(unique);
    std::fs::create_dir_all(&path).expect("create temp working dir");
    path
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

fn any_event_contains(events: &[AgentWrapperEvent], needle: &str) -> bool {
    events.iter().any(|ev| {
        ev.message
            .as_deref()
            .is_some_and(|message| message.contains(needle))
            || ev.text.as_deref().is_some_and(|text| text.contains(needle))
            || ev
                .data
                .as_ref()
                .and_then(|data| serde_json::to_string(data).ok())
                .is_some_and(|data| data.contains(needle))
    })
}

#[tokio::test]
async fn fork_id_does_not_list_and_starts_turn_on_forked_thread() {
    let prompt = "hello world";
    let source_thread_id = "thread-123";

    let backend = CodexBackend::new(CodexBackendConfig {
        binary: Some(fake_codex_app_server_binary()),
        env: [
            (
                "FAKE_CODEX_APP_SERVER_SCENARIO".to_string(),
                "fork_id_success".to_string(),
            ),
            (
                "FAKE_CODEX_APP_SERVER_EXPECT_PROMPT".to_string(),
                prompt.to_string(),
            ),
            (
                "FAKE_CODEX_APP_SERVER_EXPECT_SOURCE_THREAD_ID".to_string(),
                source_thread_id.to_string(),
            ),
        ]
        .into_iter()
        .collect(),
        ..Default::default()
    });

    let handle = backend
        .run(AgentWrapperRunRequest {
            prompt: prompt.to_string(),
            extensions: [(
                "agent_api.session.fork.v1".to_string(),
                json!({"selector":"id","id": source_thread_id}),
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
    let handle_events: Vec<&AgentWrapperEvent> = seen
        .iter()
        .filter(|ev| ev.kind == AgentWrapperEventKind::Status)
        .filter(|ev| {
            ev.data
                .as_ref()
                .and_then(|data| data.get("schema"))
                .and_then(serde_json::Value::as_str)
                == Some("agent_api.session.handle.v1")
        })
        .collect();
    assert_eq!(
        handle_events.len(),
        1,
        "expected exactly one Status event with the session handle facet"
    );
    assert_eq!(
        handle_events[0]
            .data
            .as_ref()
            .and_then(|data| data.get("session"))
            .and_then(|session| session.get("id"))
            .and_then(serde_json::Value::as_str),
        Some("forked-1")
    );

    let completion = tokio::time::timeout(Duration::from_secs(2), completion)
        .await
        .expect("completion resolves")
        .expect("completion ok");
    assert!(completion.status.success());
    assert_eq!(
        completion
            .data
            .as_ref()
            .and_then(|data| data.get("schema"))
            .and_then(serde_json::Value::as_str),
        Some("agent_api.session.handle.v1")
    );
    assert_eq!(
        completion
            .data
            .as_ref()
            .and_then(|data| data.get("session"))
            .and_then(|session| session.get("id"))
            .and_then(serde_json::Value::as_str),
        Some("forked-1")
    );
}

#[tokio::test]
async fn fork_id_oversize_forked_thread_id_is_treated_as_unknown_omits_handle_facet_and_warns_once()
{
    let prompt = "hello world";
    let source_thread_id = "thread-123";
    let oversize = "a".repeat(1025);

    let backend = CodexBackend::new(CodexBackendConfig {
        binary: Some(fake_codex_app_server_binary()),
        env: [
            (
                "FAKE_CODEX_APP_SERVER_SCENARIO".to_string(),
                "fork_id_success_oversize_thread_id".to_string(),
            ),
            (
                "FAKE_CODEX_APP_SERVER_EXPECT_PROMPT".to_string(),
                prompt.to_string(),
            ),
            (
                "FAKE_CODEX_APP_SERVER_EXPECT_SOURCE_THREAD_ID".to_string(),
                source_thread_id.to_string(),
            ),
        ]
        .into_iter()
        .collect(),
        ..Default::default()
    });

    let handle = backend
        .run(AgentWrapperRunRequest {
            prompt: prompt.to_string(),
            extensions: [(
                "agent_api.session.fork.v1".to_string(),
                json!({"selector":"id","id": source_thread_id}),
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
    assert!(
        !seen.iter().any(|ev| {
            ev.data
                .as_ref()
                .and_then(|data| data.get("schema"))
                .and_then(serde_json::Value::as_str)
                == Some("agent_api.session.handle.v1")
        }),
        "expected oversize forked thread id to be treated as unknown (no handle facet)"
    );

    let warnings: Vec<&AgentWrapperEvent> = seen
        .iter()
        .filter(|ev| ev.kind == AgentWrapperEventKind::Status)
        .filter(|ev| {
            ev.message.as_deref().is_some_and(|message| {
                message.contains("session handle id oversize")
                    && message.contains("len_bytes=1025")
                    && !message.contains(&oversize)
            })
        })
        .collect();
    assert_eq!(warnings.len(), 1, "expected exactly one oversize warning");

    assert!(
        !any_event_contains(&seen, &oversize),
        "expected oversize id to not appear in message/text/data"
    );

    let completion = tokio::time::timeout(Duration::from_secs(2), completion)
        .await
        .expect("completion resolves")
        .expect("completion ok");
    assert!(completion.status.success());
    assert_eq!(completion.data, None);
}

#[tokio::test]
async fn fork_id_forked_thread_id_len_1024_emits_handle_facet_and_does_not_warn() {
    let prompt = "hello world";
    let source_thread_id = "thread-123";

    let backend = CodexBackend::new(CodexBackendConfig {
        binary: Some(fake_codex_app_server_binary()),
        env: [
            (
                "FAKE_CODEX_APP_SERVER_SCENARIO".to_string(),
                "fork_id_success_thread_id_len_1024".to_string(),
            ),
            (
                "FAKE_CODEX_APP_SERVER_EXPECT_PROMPT".to_string(),
                prompt.to_string(),
            ),
            (
                "FAKE_CODEX_APP_SERVER_EXPECT_SOURCE_THREAD_ID".to_string(),
                source_thread_id.to_string(),
            ),
        ]
        .into_iter()
        .collect(),
        ..Default::default()
    });

    let handle = backend
        .run(AgentWrapperRunRequest {
            prompt: prompt.to_string(),
            extensions: [(
                "agent_api.session.fork.v1".to_string(),
                json!({"selector":"id","id": source_thread_id}),
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

    let handle_events: Vec<&AgentWrapperEvent> = seen
        .iter()
        .filter(|ev| ev.kind == AgentWrapperEventKind::Status)
        .filter(|ev| {
            ev.data
                .as_ref()
                .and_then(|data| data.get("schema"))
                .and_then(serde_json::Value::as_str)
                == Some("agent_api.session.handle.v1")
        })
        .collect();
    assert_eq!(handle_events.len(), 1);
    let id = handle_events[0]
        .data
        .as_ref()
        .and_then(|data| data.get("session"))
        .and_then(|session| session.get("id"))
        .and_then(serde_json::Value::as_str)
        .expect("handle facet session.id present");
    assert_eq!(id.len(), 1024);
    assert!(id.as_bytes().iter().all(|b| *b == b'a'));

    assert!(
        !seen.iter().any(|ev| {
            ev.message
                .as_deref()
                .is_some_and(|message| message.contains("session handle id oversize"))
        }),
        "expected no oversize warning for a 1024-byte id"
    );

    let completion = tokio::time::timeout(Duration::from_secs(2), completion)
        .await
        .expect("completion resolves")
        .expect("completion ok");
    assert!(completion.status.success());
    let completion_id = completion
        .data
        .as_ref()
        .and_then(|data| data.get("session"))
        .and_then(|session| session.get("id"))
        .and_then(serde_json::Value::as_str)
        .expect("completion handle facet session.id present");
    assert_eq!(completion_id.len(), 1024);
    assert!(completion_id.as_bytes().iter().all(|b| *b == b'a'));
}

#[tokio::test]
async fn fork_last_pages_thread_list_and_selects_max_tuple() {
    let prompt = "hello world";
    let working_dir = make_temp_working_dir();

    let backend = CodexBackend::new(CodexBackendConfig {
        binary: Some(fake_codex_app_server_binary()),
        env: [
            (
                "FAKE_CODEX_APP_SERVER_SCENARIO".to_string(),
                "fork_last_success_paged".to_string(),
            ),
            (
                "FAKE_CODEX_APP_SERVER_EXPECT_PROMPT".to_string(),
                prompt.to_string(),
            ),
            (
                "FAKE_CODEX_APP_SERVER_EXPECT_CWD".to_string(),
                working_dir.to_string_lossy().to_string(),
            ),
        ]
        .into_iter()
        .collect(),
        ..Default::default()
    });

    let handle = backend
        .run(AgentWrapperRunRequest {
            prompt: prompt.to_string(),
            working_dir: Some(working_dir),
            extensions: [(
                "agent_api.session.fork.v1".to_string(),
                json!({"selector":"last"}),
            )]
            .into_iter()
            .collect(),
            ..Default::default()
        })
        .await
        .expect("run");

    let mut events = handle.events;
    let completion = handle.completion;

    let _seen = drain_to_none(events.as_mut(), Duration::from_secs(2)).await;

    let completion = tokio::time::timeout(Duration::from_secs(2), completion)
        .await
        .expect("completion resolves")
        .expect("completion ok");
    assert!(completion.status.success());
}

#[tokio::test]
async fn fork_last_empty_list_translates_to_no_session_found_and_emits_terminal_error_event() {
    let prompt = "hello world";
    let working_dir = make_temp_working_dir();

    let backend = CodexBackend::new(CodexBackendConfig {
        binary: Some(fake_codex_app_server_binary()),
        env: [
            (
                "FAKE_CODEX_APP_SERVER_SCENARIO".to_string(),
                "fork_last_empty".to_string(),
            ),
            (
                "FAKE_CODEX_APP_SERVER_EXPECT_PROMPT".to_string(),
                prompt.to_string(),
            ),
            (
                "FAKE_CODEX_APP_SERVER_EXPECT_CWD".to_string(),
                working_dir.to_string_lossy().to_string(),
            ),
        ]
        .into_iter()
        .collect(),
        ..Default::default()
    });

    let handle = backend
        .run(AgentWrapperRunRequest {
            prompt: prompt.to_string(),
            working_dir: Some(working_dir),
            extensions: [(
                "agent_api.session.fork.v1".to_string(),
                json!({"selector":"last"}),
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
async fn fork_id_not_found_translates_to_session_not_found_and_never_leaks_backend_details() {
    let prompt = "hello world";
    let source_thread_id = "thread-123";
    let secret = "RAW-BACKEND-SECRET-DO-NOT-LEAK";

    let backend = CodexBackend::new(CodexBackendConfig {
        binary: Some(fake_codex_app_server_binary()),
        env: [
            (
                "FAKE_CODEX_APP_SERVER_SCENARIO".to_string(),
                "fork_id_not_found".to_string(),
            ),
            (
                "FAKE_CODEX_APP_SERVER_EXPECT_PROMPT".to_string(),
                prompt.to_string(),
            ),
            (
                "FAKE_CODEX_APP_SERVER_EXPECT_SOURCE_THREAD_ID".to_string(),
                source_thread_id.to_string(),
            ),
            (
                "FAKE_CODEX_APP_SERVER_SECRET_SENTINEL".to_string(),
                secret.to_string(),
            ),
        ]
        .into_iter()
        .collect(),
        ..Default::default()
    });

    let handle = backend
        .run(AgentWrapperRunRequest {
            prompt: prompt.to_string(),
            extensions: [(
                "agent_api.session.fork.v1".to_string(),
                json!({"selector":"id","id": source_thread_id}),
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
    assert!(
        !any_event_contains(&seen, secret),
        "expected backend secrets to never leak into message/text/data"
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
async fn approval_required_fails_fast_sends_cancel_request_and_surfaces_approval_required() {
    let prompt = "hello world";
    let source_thread_id = "thread-123";
    let secret = "RAW-BACKEND-SECRET-DO-NOT-LEAK";

    let backend = CodexBackend::new(CodexBackendConfig {
        binary: Some(fake_codex_app_server_binary()),
        env: [
            (
                "FAKE_CODEX_APP_SERVER_SCENARIO".to_string(),
                "approval_required_during_turn_start".to_string(),
            ),
            (
                "FAKE_CODEX_APP_SERVER_EXPECT_PROMPT".to_string(),
                prompt.to_string(),
            ),
            (
                "FAKE_CODEX_APP_SERVER_EXPECT_SOURCE_THREAD_ID".to_string(),
                source_thread_id.to_string(),
            ),
            (
                "FAKE_CODEX_APP_SERVER_SECRET_SENTINEL".to_string(),
                secret.to_string(),
            ),
        ]
        .into_iter()
        .collect(),
        ..Default::default()
    });

    let handle = backend
        .run(AgentWrapperRunRequest {
            prompt: prompt.to_string(),
            extensions: [(
                "agent_api.session.fork.v1".to_string(),
                json!({"selector":"id","id": source_thread_id}),
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
        Some("approval required")
    );
    assert!(
        !any_event_contains(&seen, secret),
        "expected backend secrets to never leak into message/text/data"
    );

    let err = tokio::time::timeout(Duration::from_secs(2), completion)
        .await
        .expect("completion resolves")
        .unwrap_err();
    match err {
        AgentWrapperError::Backend { message } => assert_eq!(message, "approval required"),
        other => panic!("expected Backend error, got: {other:?}"),
    }
}

#[tokio::test]
async fn explicit_cancel_sends_cancel_request_and_completion_is_cancelled() {
    let prompt = "hello world";
    let source_thread_id = "thread-123";

    let backend = std::sync::Arc::new(CodexBackend::new(CodexBackendConfig {
        binary: Some(fake_codex_app_server_binary()),
        env: [
            (
                "FAKE_CODEX_APP_SERVER_SCENARIO".to_string(),
                "block_until_cancel".to_string(),
            ),
            (
                "FAKE_CODEX_APP_SERVER_EXPECT_PROMPT".to_string(),
                prompt.to_string(),
            ),
            (
                "FAKE_CODEX_APP_SERVER_EXPECT_SOURCE_THREAD_ID".to_string(),
                source_thread_id.to_string(),
            ),
        ]
        .into_iter()
        .collect(),
        ..Default::default()
    }));

    let mut gateway = AgentWrapperGateway::new();
    gateway.register(backend).expect("register codex backend");
    let kind = AgentWrapperKind::new("codex").expect("valid kind");

    let run = gateway
        .run_control(
            &kind,
            AgentWrapperRunRequest {
                prompt: prompt.to_string(),
                extensions: [(
                    "agent_api.session.fork.v1".to_string(),
                    json!({"selector":"id","id": source_thread_id}),
                )]
                .into_iter()
                .collect(),
                ..Default::default()
            },
        )
        .await
        .expect("run_control");

    let agent_api::AgentWrapperRunControl { handle, cancel } = run;
    let agent_api::AgentWrapperRunHandle {
        mut events,
        completion,
    } = handle;

    let first = tokio::time::timeout(
        Duration::from_secs(1),
        std::future::poll_fn(|cx| events.as_mut().poll_next(cx)),
    )
    .await
    .expect("first event should arrive");
    let Some(_first) = first else {
        panic!("events stream ended before first event");
    };

    cancel.cancel();
    cancel.cancel();

    let (rest, completion_outcome) = tokio::time::timeout(Duration::from_secs(3), async {
        tokio::join!(
            drain_to_none(events.as_mut(), Duration::from_secs(2)),
            completion
        )
    })
    .await
    .expect("cancellation should terminate the backend and close the event stream");

    let _ = rest;

    match completion_outcome {
        Err(AgentWrapperError::Backend { ref message }) if message == "cancelled" => {}
        other => panic!("expected cancelled completion error, got {other:?}"),
    }
}
