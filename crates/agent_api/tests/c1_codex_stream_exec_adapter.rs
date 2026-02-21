#![cfg(feature = "codex")]

use std::{collections::BTreeMap, path::PathBuf, pin::Pin, time::Duration};

use agent_api::{
    backends::codex::{CodexBackend, CodexBackendConfig},
    AgentWrapperBackend, AgentWrapperError, AgentWrapperEvent, AgentWrapperEventKind,
    AgentWrapperRunRequest,
};
use futures_core::Stream;
use serde_json::Value;

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

fn find_first_kind(events: &[AgentWrapperEvent], kind: AgentWrapperEventKind) -> Option<usize> {
    events.iter().position(|ev| ev.kind == kind)
}

fn tool_schema(event: &AgentWrapperEvent) -> Option<&str> {
    event
        .data
        .as_ref()
        .and_then(|data| data.get("schema"))
        .and_then(Value::as_str)
}

fn tool_field<'a>(event: &'a AgentWrapperEvent, field: &str) -> Option<&'a Value> {
    event
        .data
        .as_ref()
        .and_then(|data| data.get("tool"))
        .and_then(|tool| tool.get(field))
}

#[tokio::test]
async fn empty_prompt_is_rejected_before_spawning() {
    let backend = CodexBackend::new(CodexBackendConfig::default());
    let err = backend
        .run(AgentWrapperRunRequest {
            prompt: "   ".to_string(),
            ..Default::default()
        })
        .await
        .unwrap_err();
    assert!(matches!(err, AgentWrapperError::InvalidRequest { .. }));
}

#[tokio::test]
async fn unknown_extension_key_is_rejected_fail_closed() {
    let backend = CodexBackend::new(CodexBackendConfig::default());
    let err = backend
        .run(AgentWrapperRunRequest {
            prompt: "hello".to_string(),
            extensions: [(
                "backend.codex.exec.unknown_key".to_string(),
                serde_json::Value::Bool(true),
            )]
            .into_iter()
            .collect(),
            ..Default::default()
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        AgentWrapperError::UnsupportedCapability { .. }
    ));
}

#[tokio::test]
async fn extension_types_are_validated() {
    let backend = CodexBackend::new(CodexBackendConfig::default());
    let err = backend
        .run(AgentWrapperRunRequest {
            prompt: "hello".to_string(),
            extensions: [(
                "agent_api.exec.non_interactive".to_string(),
                serde_json::Value::String("true".to_string()),
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
async fn non_interactive_true_rejects_contradictory_approval_policy() {
    let backend = CodexBackend::new(CodexBackendConfig::default());
    let err = backend
        .run(AgentWrapperRunRequest {
            prompt: "hello".to_string(),
            extensions: [
                (
                    "agent_api.exec.non_interactive".to_string(),
                    serde_json::Value::Bool(true),
                ),
                (
                    "backend.codex.exec.approval_policy".to_string(),
                    serde_json::Value::String("untrusted".to_string()),
                ),
            ]
            .into_iter()
            .collect(),
            ..Default::default()
        })
        .await
        .unwrap_err();
    assert!(matches!(err, AgentWrapperError::InvalidRequest { .. }));
}

#[tokio::test]
async fn parse_errors_do_not_leak_raw_lines_and_stream_continues() {
    let backend = CodexBackend::new(CodexBackendConfig {
        binary: Some(fake_codex_binary()),
        env: [(
            "FAKE_CODEX_SCENARIO".to_string(),
            "parse_error_midstream".to_string(),
        )]
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
        .unwrap();

    let mut events = handle.events;
    let completion = handle.completion;

    let seen = drain_to_none(events.as_mut(), Duration::from_secs(2)).await;
    assert!(
        find_first_kind(&seen, AgentWrapperEventKind::Error).is_some(),
        "expected an Error event for the parse failure"
    );
    assert!(
        !any_event_contains(&seen, "RAW-LINE-SECRET-PARSE"),
        "expected redaction to avoid raw JSONL line content"
    );

    let first_error = find_first_kind(&seen, AgentWrapperEventKind::Error).unwrap();
    assert!(
        seen.iter()
            .skip(first_error + 1)
            .any(|ev| ev.kind == AgentWrapperEventKind::Status),
        "expected the stream to continue after a per-line error"
    );

    let completion = tokio::time::timeout(Duration::from_secs(2), completion)
        .await
        .expect("completion resolves")
        .unwrap();
    assert!(completion.status.success());
}

#[tokio::test]
async fn normalize_errors_do_not_leak_raw_lines_and_stream_continues() {
    let backend = CodexBackend::new(CodexBackendConfig {
        binary: Some(fake_codex_binary()),
        env: [(
            "FAKE_CODEX_SCENARIO".to_string(),
            "normalize_error_midstream".to_string(),
        )]
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
        .unwrap();

    let mut events = handle.events;
    let completion = handle.completion;

    let seen = drain_to_none(events.as_mut(), Duration::from_secs(2)).await;
    assert!(
        find_first_kind(&seen, AgentWrapperEventKind::Error).is_some(),
        "expected an Error event for the normalize failure"
    );
    assert!(
        !any_event_contains(&seen, "RAW-LINE-SECRET-NORM"),
        "expected redaction to avoid raw JSONL line content"
    );

    let first_error = find_first_kind(&seen, AgentWrapperEventKind::Error).unwrap();
    assert!(
        seen.iter()
            .skip(first_error + 1)
            .any(|ev| ev.kind == AgentWrapperEventKind::Status),
        "expected the stream to continue after a per-line error"
    );

    let completion = tokio::time::timeout(Duration::from_secs(2), completion)
        .await
        .expect("completion resolves")
        .unwrap();
    assert!(completion.status.success());
}

#[tokio::test]
async fn nonzero_exit_is_redacted_and_completion_is_ok_with_nonzero_status() {
    let backend = CodexBackend::new(CodexBackendConfig {
        binary: Some(fake_codex_binary()),
        env: [(
            "FAKE_CODEX_SCENARIO".to_string(),
            "nonzero_exit".to_string(),
        )]
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
        .unwrap();

    let mut events = handle.events;
    let completion = handle.completion;

    let seen = drain_to_none(events.as_mut(), Duration::from_secs(2)).await;
    assert!(
        find_first_kind(&seen, AgentWrapperEventKind::Error).is_some(),
        "expected an Error event for the non-zero exit"
    );
    assert!(
        !any_event_contains(&seen, "RAW-STDERR-SECRET"),
        "expected stderr redaction on non-zero exit"
    );

    let completion = tokio::time::timeout(Duration::from_secs(2), completion)
        .await
        .expect("completion resolves")
        .unwrap();
    assert!(!completion.status.success());
    assert_eq!(completion.final_text, None);
}

#[tokio::test]
async fn request_env_overrides_config_env_and_parent_env_is_unchanged() {
    struct EnvGuard {
        key: &'static str,
        previous: Option<String>,
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            if let Some(value) = self.previous.as_ref() {
                std::env::set_var(self.key, value);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }

    let key = "C1_PARENT_ENV_SENTINEL";
    let previous = std::env::var(key).ok();
    std::env::set_var(key, "original");
    let _guard = EnvGuard { key, previous };

    let backend = CodexBackend::new(CodexBackendConfig {
        binary: Some(fake_codex_binary()),
        env: [
            ("FAKE_CODEX_SCENARIO".to_string(), "env_assert".to_string()),
            ("C1_TEST_KEY".to_string(), "config".to_string()),
            ("C1_ONLY_CONFIG".to_string(), "config-only".to_string()),
            (
                "FAKE_CODEX_ASSERT_ENV_C1_TEST_KEY".to_string(),
                "request".to_string(),
            ),
            (
                "FAKE_CODEX_ASSERT_ENV_C1_ONLY_CONFIG".to_string(),
                "config-only".to_string(),
            ),
        ]
        .into_iter()
        .collect(),
        ..Default::default()
    });

    let handle = backend
        .run(AgentWrapperRunRequest {
            prompt: "hello".to_string(),
            env: [("C1_TEST_KEY".to_string(), "request".to_string())]
                .into_iter()
                .collect::<BTreeMap<_, _>>(),
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

    assert_eq!(
        std::env::var(key).ok().as_deref(),
        Some("original"),
        "expected backend to not mutate parent process environment"
    );
}

#[tokio::test]
async fn tool_lifecycle_ok() {
    let backend = CodexBackend::new(CodexBackendConfig {
        binary: Some(fake_codex_binary()),
        env: [(
            "FAKE_CODEX_SCENARIO".to_string(),
            "tool_lifecycle_ok".to_string(),
        )]
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
        .unwrap();

    let mut events = handle.events;
    let completion = handle.completion;

    let seen = drain_to_none(events.as_mut(), Duration::from_secs(2)).await;

    let first_tool_call =
        find_first_kind(&seen, AgentWrapperEventKind::ToolCall).expect("expected a ToolCall event");
    let first_tool_result = find_first_kind(&seen, AgentWrapperEventKind::ToolResult)
        .expect("expected a ToolResult event");
    assert!(
        first_tool_call < first_tool_result,
        "expected ToolCall to occur before ToolResult"
    );

    for ev in seen.iter() {
        if matches!(
            ev.kind,
            AgentWrapperEventKind::ToolCall | AgentWrapperEventKind::ToolResult
        ) {
            assert_eq!(
                tool_schema(ev),
                Some("agent_api.tools.structured.v1"),
                "expected tools facet schema on every ToolCall/ToolResult"
            );
        }
    }

    assert!(
        !any_event_contains(&seen, "STDOUT-SENTINEL-DO-NOT-LEAK"),
        "expected tool output sentinel to not appear in text/message/data"
    );
    assert!(
        !any_event_contains(&seen, "STDERR-SENTINEL-DO-NOT-LEAK"),
        "expected tool output sentinel to not appear in text/message/data"
    );

    let completion = tokio::time::timeout(Duration::from_secs(2), completion)
        .await
        .expect("completion resolves")
        .unwrap();
    assert!(completion.status.success());
}

#[tokio::test]
async fn tool_lifecycle_fail_unknown_type() {
    let backend = CodexBackend::new(CodexBackendConfig {
        binary: Some(fake_codex_binary()),
        env: [(
            "FAKE_CODEX_SCENARIO".to_string(),
            "tool_lifecycle_fail_unknown_type".to_string(),
        )]
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
        .unwrap();

    let mut events = handle.events;
    let completion = handle.completion;

    let seen = drain_to_none(events.as_mut(), Duration::from_secs(2)).await;
    assert!(
        find_first_kind(&seen, AgentWrapperEventKind::Error).is_some(),
        "expected an Error event when item.failed has no deterministically-attributable item_type"
    );
    assert!(
        !seen.iter().any(|ev| {
            ev.kind == AgentWrapperEventKind::ToolResult
                && tool_field(ev, "phase").and_then(Value::as_str) == Some("fail")
        }),
        "expected no failure ToolResult when item_type is absent"
    );

    let completion = tokio::time::timeout(Duration::from_secs(2), completion)
        .await
        .expect("completion resolves")
        .unwrap();
    assert!(completion.status.success());
}

#[tokio::test]
async fn tool_lifecycle_fail_known_type() {
    let backend = CodexBackend::new(CodexBackendConfig {
        binary: Some(fake_codex_binary()),
        env: [(
            "FAKE_CODEX_SCENARIO".to_string(),
            "tool_lifecycle_fail_known_type".to_string(),
        )]
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
        .unwrap();

    let mut events = handle.events;
    let completion = handle.completion;

    let seen = drain_to_none(events.as_mut(), Duration::from_secs(2)).await;
    assert!(
        seen.iter().any(|ev| {
            ev.kind == AgentWrapperEventKind::ToolResult
                && tool_field(ev, "phase").and_then(Value::as_str) == Some("fail")
                && tool_field(ev, "status").and_then(Value::as_str) == Some("failed")
                && tool_field(ev, "kind").and_then(Value::as_str) == Some("command_execution")
        }),
        "expected failure ToolResult when item.failed has deterministically-attributable item_type"
    );

    let completion = tokio::time::timeout(Duration::from_secs(2), completion)
        .await
        .expect("completion resolves")
        .unwrap();
    assert!(completion.status.success());
}
