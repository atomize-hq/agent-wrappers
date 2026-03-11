use std::time::Duration;

use agent_api::{
    mcp::{
        AgentWrapperMcpAddRequest, AgentWrapperMcpAddTransport, AgentWrapperMcpCommandContext,
        AgentWrapperMcpListRequest,
    },
    AgentWrapperBackend, AgentWrapperError,
};

use super::{
    claude_support::{
        backend_error_message, claude_config_env, claude_gateway, claude_get_supported,
        claude_list_supported, FAKE_CLAUDE_SCENARIO_ENV,
    },
    support::McpTestSandbox,
};

const TIMEOUT_STDOUT_SENTINEL: &str = "fake_claude_mcp timeout stdout sentinel";
const TIMEOUT_STDERR_SENTINEL: &str = "fake_claude_mcp timeout stderr sentinel";
const FAST_EXIT_STDOUT_SENTINEL: &str = "fake_claude_mcp fast-exit stdout sentinel";
const FAST_EXIT_STDERR_SENTINEL: &str = "fake_claude_mcp fast-exit stderr sentinel";

#[tokio::test]
async fn claude_mcp_add_url_rejects_bearer_env_var_without_spawning() {
    if !claude_get_supported() {
        return;
    }

    let sandbox = McpTestSandbox::new("claude_mcp_add_url_rejects_bearer_env").expect("sandbox");
    let (_backend, gateway, kind) = claude_gateway(
        &sandbox,
        true,
        claude_config_env(&sandbox, std::iter::empty()),
        None,
        None,
    );

    let err = gateway
        .mcp_add(
            &kind,
            AgentWrapperMcpAddRequest {
                name: "demo".to_string(),
                transport: AgentWrapperMcpAddTransport::Url {
                    url: "https://example.test/mcp".to_string(),
                    bearer_token_env_var: Some("TOKEN".to_string()),
                },
                context: Default::default(),
            },
        )
        .await
        .expect_err("bearer_token_env_var must be rejected for claude");

    match err {
        AgentWrapperError::InvalidRequest { message } => {
            assert_eq!(
                message,
                "claude mcp add url transport does not support bearer_token_env_var"
            );
        }
        other => panic!("expected InvalidRequest, got {other:?}"),
    }
    assert!(
        !sandbox.record_path().exists(),
        "bearer_token_env_var rejection must happen before spawning the fake claude binary"
    );
}

#[tokio::test]
async fn claude_mcp_drift_returns_backend_error_without_mutating_capabilities() {
    if !claude_list_supported() {
        return;
    }

    let sandbox = McpTestSandbox::new("claude_mcp_drift").expect("sandbox");
    let (backend, gateway, kind) = claude_gateway(
        &sandbox,
        false,
        claude_config_env(
            &sandbox,
            [(FAKE_CLAUDE_SCENARIO_ENV.to_string(), "drift".to_string())],
        ),
        None,
        None,
    );
    let before = backend.capabilities().ids.clone();

    let err = gateway
        .mcp_list(&kind, AgentWrapperMcpListRequest::default())
        .await
        .expect_err("drift must fail closed");

    let message = backend_error_message(err);
    assert!(
        !message.contains("unknown subcommand 'list'"),
        "drift error leaked subprocess stderr: {message}"
    );
    assert_eq!(backend.capabilities().ids, before);
}

#[tokio::test]
async fn claude_mcp_add_flag_drift_returns_backend_error_without_mutating_capabilities() {
    if !claude_get_supported() {
        return;
    }

    let sandbox = McpTestSandbox::new("claude_mcp_add_flag_drift").expect("sandbox");
    let (backend, gateway, kind) = claude_gateway(
        &sandbox,
        true,
        claude_config_env(
            &sandbox,
            [(
                FAKE_CLAUDE_SCENARIO_ENV.to_string(),
                "add_flag_drift".to_string(),
            )],
        ),
        None,
        None,
    );
    let before = backend.capabilities().ids.clone();

    let err = gateway
        .mcp_add(
            &kind,
            AgentWrapperMcpAddRequest {
                name: "demo".to_string(),
                transport: AgentWrapperMcpAddTransport::Stdio {
                    command: vec!["node".to_string()],
                    args: vec!["server.js".to_string()],
                    env: Default::default(),
                },
                context: Default::default(),
            },
        )
        .await
        .expect_err("add flag drift must fail closed");

    let message = backend_error_message(err);
    assert!(
        !message.contains("unexpected argument '--transport'"),
        "drift error leaked subprocess stderr: {message}"
    );
    assert_eq!(backend.capabilities().ids, before);
    assert!(
        sandbox.record_path().exists(),
        "add flag drift path should have spawned the fake claude binary"
    );
}

#[tokio::test]
async fn claude_mcp_timeout_returns_backend_error_without_leaking_partial_output() {
    if !claude_list_supported() {
        return;
    }

    let sandbox = McpTestSandbox::new("claude_mcp_timeout").expect("sandbox");
    let (_backend, gateway, kind) = claude_gateway(
        &sandbox,
        false,
        claude_config_env(
            &sandbox,
            [(
                FAKE_CLAUDE_SCENARIO_ENV.to_string(),
                "sleep_for_timeout".to_string(),
            )],
        ),
        None,
        None,
    );

    let err = gateway
        .mcp_list(
            &kind,
            AgentWrapperMcpListRequest {
                context: AgentWrapperMcpCommandContext {
                    timeout: Some(Duration::from_millis(500)),
                    ..Default::default()
                },
            },
        )
        .await
        .expect_err("timeout should return Backend error");

    let message = backend_error_message(err);
    assert!(
        !message.contains(TIMEOUT_STDOUT_SENTINEL),
        "timeout error leaked stdout sentinel: {message}"
    );
    assert!(
        !message.contains(TIMEOUT_STDERR_SENTINEL),
        "timeout error leaked stderr sentinel: {message}"
    );
    assert!(
        message.contains("timeout"),
        "timeout failures should stay redacted but mention timeout: {message}"
    );
}

#[tokio::test]
async fn claude_mcp_zero_timeout_returns_backend_error_without_leaking_partial_output() {
    if !claude_list_supported() {
        return;
    }

    let sandbox = McpTestSandbox::new("claude_mcp_zero_timeout").expect("sandbox");
    let (_backend, gateway, kind) = claude_gateway(
        &sandbox,
        false,
        claude_config_env(
            &sandbox,
            [(
                FAKE_CLAUDE_SCENARIO_ENV.to_string(),
                "sleep_for_timeout".to_string(),
            )],
        ),
        None,
        None,
    );

    let err = gateway
        .mcp_list(
            &kind,
            AgentWrapperMcpListRequest {
                context: AgentWrapperMcpCommandContext {
                    timeout: Some(Duration::ZERO),
                    ..Default::default()
                },
            },
        )
        .await
        .expect_err("zero timeout should return Backend error");

    let message = backend_error_message(err);
    assert!(
        !message.contains(TIMEOUT_STDOUT_SENTINEL),
        "zero-timeout error leaked stdout sentinel: {message}"
    );
    assert!(
        !message.contains(TIMEOUT_STDERR_SENTINEL),
        "zero-timeout error leaked stderr sentinel: {message}"
    );
    assert!(
        message.contains("timeout"),
        "zero-timeout failures should stay redacted but mention timeout: {message}"
    );
}

#[tokio::test]
async fn claude_mcp_zero_timeout_fast_exit_still_returns_timeout_error() {
    if !claude_list_supported() {
        return;
    }

    let sandbox = McpTestSandbox::new("claude_mcp_zero_timeout_fast_exit").expect("sandbox");
    let (_backend, gateway, kind) = claude_gateway(
        &sandbox,
        false,
        claude_config_env(
            &sandbox,
            [(
                FAKE_CLAUDE_SCENARIO_ENV.to_string(),
                "fast_exit_with_output".to_string(),
            )],
        ),
        None,
        None,
    );

    let err = gateway
        .mcp_list(
            &kind,
            AgentWrapperMcpListRequest {
                context: AgentWrapperMcpCommandContext {
                    timeout: Some(Duration::ZERO),
                    ..Default::default()
                },
            },
        )
        .await
        .expect_err("zero timeout should fail even for fast-exit commands");

    let message = backend_error_message(err);
    assert!(
        !message.contains(FAST_EXIT_STDOUT_SENTINEL),
        "zero-timeout error leaked fast-exit stdout sentinel: {message}"
    );
    assert!(
        !message.contains(FAST_EXIT_STDERR_SENTINEL),
        "zero-timeout error leaked fast-exit stderr sentinel: {message}"
    );
    assert!(
        message.contains("timeout"),
        "zero-timeout fast-exit failures should stay redacted but mention timeout: {message}"
    );
}
