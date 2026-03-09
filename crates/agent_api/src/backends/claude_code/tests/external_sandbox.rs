use std::sync::{atomic::Ordering, Arc};

use tokio::sync::OnceCell;

use super::support::*;

#[test]
fn claude_backend_does_not_advertise_external_sandbox_exec_by_default() {
    let backend = ClaudeCodeBackend::new(ClaudeCodeBackendConfig::default());
    let capabilities = backend.capabilities();
    assert!(!capabilities.contains(EXT_EXTERNAL_SANDBOX_V1));
}

#[test]
fn claude_backend_opt_in_advertises_external_sandbox_exec_and_allowlist_accepts_key() {
    let config = ClaudeCodeBackendConfig {
        allow_external_sandbox_exec: true,
        ..Default::default()
    };

    let backend = ClaudeCodeBackend::new(config.clone());
    let capabilities = backend.capabilities();
    assert!(capabilities.contains(EXT_EXTERNAL_SANDBOX_V1));

    let adapter = new_adapter_with_config(config);
    let defaults = crate::backend_harness::BackendDefaults::default();
    let request = AgentWrapperRunRequest {
        prompt: "hello".to_string(),
        extensions: [(EXT_EXTERNAL_SANDBOX_V1.to_string(), JsonValue::Bool(true))]
            .into_iter()
            .collect(),
        ..Default::default()
    };

    crate::backend_harness::normalize_request(&adapter, &defaults, request)
        .expect("external sandbox extension key should be allowlisted when opted-in");
}

#[test]
fn claude_backend_fails_closed_for_external_sandbox_extension_when_opt_in_disabled() {
    let adapter = new_adapter();
    let defaults = crate::backend_harness::BackendDefaults::default();
    let request = AgentWrapperRunRequest {
        prompt: "hello".to_string(),
        extensions: [(EXT_EXTERNAL_SANDBOX_V1.to_string(), JsonValue::Bool(true))]
            .into_iter()
            .collect(),
        ..Default::default()
    };

    let err = match crate::backend_harness::normalize_request(&adapter, &defaults, request) {
        Ok(_) => panic!("expected normalize_request to reject unsupported extension key"),
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
async fn allow_flag_preflight_retries_after_failure() {
    let cell = OnceCell::new();

    let result = super::super::util::preflight_allow_flag_support(&cell, || async {
        Ok::<_, claude_code::ClaudeCodeError>(claude_code::CommandOutput {
            status: exit_status_with_code(1),
            stdout: Vec::new(),
            stderr: Vec::new(),
        })
    })
    .await;

    assert!(result.is_err(), "preflight should surface the failure");
    assert!(
        cell.get().is_none(),
        "failed preflight should not initialize the OnceCell"
    );

    let supported = super::super::util::preflight_allow_flag_support(&cell, || async {
        Ok::<_, claude_code::ClaudeCodeError>(claude_code::CommandOutput {
            status: success_exit_status(),
            stdout: b"--allow-dangerously-skip-permissions".to_vec(),
            stderr: Vec::new(),
        })
    })
    .await
    .expect("preflight should succeed");

    assert!(supported);
    assert_eq!(cell.get().copied(), Some(true));

    let called = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let called_clone = Arc::clone(&called);
    let supported = super::super::util::preflight_allow_flag_support(&cell, move || {
        let called = Arc::clone(&called_clone);
        async move {
            called.fetch_add(1, Ordering::SeqCst);
            Ok::<_, claude_code::ClaudeCodeError>(claude_code::CommandOutput {
                status: success_exit_status(),
                stdout: Vec::new(),
                stderr: Vec::new(),
            })
        }
    })
    .await
    .expect("cached preflight should succeed");

    assert!(supported);
    assert_eq!(called.load(Ordering::SeqCst), 0);
}
