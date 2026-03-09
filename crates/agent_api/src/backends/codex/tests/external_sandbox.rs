use super::support::*;
use serde_json::json;

#[test]
fn codex_backend_does_not_advertise_external_sandbox_exec_by_default() {
    assert!(!CodexBackendConfig::default().allow_external_sandbox_exec);

    let backend = CodexBackend::new(CodexBackendConfig::default());
    let capabilities = backend.capabilities();
    assert!(!capabilities.contains(EXT_EXTERNAL_SANDBOX_V1));

    let adapter = test_adapter();
    assert!(!adapter
        .supported_extension_keys()
        .contains(&EXT_EXTERNAL_SANDBOX_V1));
}

#[test]
fn codex_backend_advertises_external_sandbox_exec_when_opted_in_and_normalize_allows_key() {
    let config = CodexBackendConfig {
        allow_external_sandbox_exec: true,
        ..Default::default()
    };

    let backend = CodexBackend::new(config.clone());
    let capabilities = backend.capabilities();
    assert!(capabilities.contains(EXT_EXTERNAL_SANDBOX_V1));

    let adapter = test_adapter_with_config(config);
    assert!(adapter
        .supported_extension_keys()
        .contains(&EXT_EXTERNAL_SANDBOX_V1));

    let defaults = BackendDefaults::default();
    let mut request = AgentWrapperRunRequest {
        prompt: "hello".to_string(),
        ..Default::default()
    };
    request
        .extensions
        .insert(EXT_EXTERNAL_SANDBOX_V1.to_string(), json!(true));

    crate::backend_harness::normalize_request(&adapter, &defaults, request)
        .expect("expected external sandbox key to pass allowlist gate when opted in");
}

#[test]
fn external_sandbox_extension_key_fails_closed_when_opt_in_disabled() {
    let adapter = test_adapter();
    let defaults = BackendDefaults::default();
    let mut request = AgentWrapperRunRequest {
        prompt: "hello".to_string(),
        ..Default::default()
    };
    request
        .extensions
        .insert(EXT_EXTERNAL_SANDBOX_V1.to_string(), json!(true));

    let err = match crate::backend_harness::normalize_request(&adapter, &defaults, request) {
        Ok(_) => panic!("expected UnsupportedCapability when opt-in is disabled"),
        Err(err) => err,
    };
    match err {
        AgentWrapperError::UnsupportedCapability { capability, .. } => {
            assert_eq!(capability, EXT_EXTERNAL_SANDBOX_V1);
        }
        other => panic!("expected UnsupportedCapability, got: {other:?}"),
    }
}

#[tokio::test]
async fn external_sandbox_spawn_failure_emits_warning_before_terminal_error() {
    let adapter = test_adapter_with_config(CodexBackendConfig {
        allow_external_sandbox_exec: true,
        ..Default::default()
    });

    let spawned = adapter
        .spawn(crate::backend_harness::NormalizedRequest {
            agent_kind: adapter.kind(),
            prompt: "hello".to_string(),
            working_dir: None,
            effective_timeout: None,
            env: std::collections::BTreeMap::new(),
            policy: CodexExecPolicy {
                non_interactive: true,
                external_sandbox: true,
                approval_policy: None,
                sandbox_mode: CodexSandboxMode::WorkspaceWrite,
                resume: None,
                fork: None,
            },
        })
        .await
        .expect("startup failure should still return a stream when external sandbox is enabled");

    let backend_events: Vec<CodexBackendEvent> = spawned
        .events
        .map(|result| result.expect("synthetic startup-failure events should be infallible"))
        .collect()
        .await;
    assert_eq!(backend_events.len(), 2);
    assert!(matches!(
        backend_events[0],
        CodexBackendEvent::ExternalSandboxWarning
    ));
    assert!(matches!(
        &backend_events[1],
        CodexBackendEvent::TerminalError { message }
        if message == "codex backend failed to resolve working directory"
    ));

    let mapped: Vec<_> = backend_events
        .into_iter()
        .flat_map(|event| adapter.map_event(event))
        .collect();
    assert_eq!(mapped.len(), 2);
    assert_eq!(mapped[0].kind, AgentWrapperEventKind::Status);
    assert_eq!(
        mapped[0].message.as_deref(),
        Some(PINNED_EXTERNAL_SANDBOX_WARNING)
    );
    assert_eq!(mapped[1].kind, AgentWrapperEventKind::Error);
    assert_eq!(
        mapped[1].message.as_deref(),
        Some("codex backend failed to resolve working directory")
    );

    let err = spawned
        .completion
        .await
        .expect_err("startup failure completion should preserve the backend error");
    assert!(matches!(err, CodexBackendError::WorkingDirectoryUnresolved));
}
