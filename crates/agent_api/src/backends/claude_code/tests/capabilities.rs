use super::support::*;

#[test]
fn claude_backend_reports_required_capabilities() {
    let backend = ClaudeCodeBackend::new(ClaudeCodeBackendConfig::default());
    let capabilities = backend.capabilities();
    assert!(capabilities.contains("agent_api.run"));
    assert!(capabilities.contains("agent_api.events"));
    assert!(capabilities.contains("agent_api.events.live"));
    assert!(capabilities.contains(crate::CAPABILITY_CONTROL_CANCEL_V1));
    assert!(capabilities.contains(CAP_TOOLS_STRUCTURED_V1));
    assert!(capabilities.contains(CAP_TOOLS_RESULTS_V1));
    assert!(capabilities.contains(CAP_ARTIFACTS_FINAL_TEXT_V1));
    assert!(capabilities.contains(CAP_SESSION_HANDLE_V1));
    assert!(capabilities.contains(EXT_SESSION_RESUME_V1));
    assert!(capabilities.contains(EXT_SESSION_FORK_V1));
}

#[test]
fn claude_backend_mcp_write_capabilities_are_disabled_by_default() {
    assert!(!ClaudeCodeBackendConfig::default().allow_mcp_write);

    let backend = ClaudeCodeBackend::new(ClaudeCodeBackendConfig::default());
    let capabilities = backend.capabilities();
    assert_eq!(
        capabilities.contains(CAPABILITY_MCP_LIST_V1),
        claude_mcp_list_supported_on_target()
    );
    assert_eq!(
        capabilities.contains(CAPABILITY_MCP_GET_V1),
        claude_mcp_get_supported_on_target()
    );
    assert!(!capabilities.contains(CAPABILITY_MCP_ADD_V1));
    assert!(!capabilities.contains(CAPABILITY_MCP_REMOVE_V1));
}

#[test]
fn claude_backend_mcp_write_capabilities_require_opt_in_and_target_support() {
    let backend = ClaudeCodeBackend::new(ClaudeCodeBackendConfig {
        allow_mcp_write: true,
        ..Default::default()
    });
    let capabilities = backend.capabilities();
    assert_eq!(
        capabilities.contains(CAPABILITY_MCP_LIST_V1),
        claude_mcp_list_supported_on_target()
    );
    assert_eq!(
        capabilities.contains(CAPABILITY_MCP_GET_V1),
        claude_mcp_get_supported_on_target()
    );
    assert_eq!(
        capabilities.contains(CAPABILITY_MCP_ADD_V1),
        claude_mcp_get_supported_on_target()
    );
    assert_eq!(
        capabilities.contains(CAPABILITY_MCP_REMOVE_V1),
        claude_mcp_get_supported_on_target()
    );
}

#[tokio::test]
async fn claude_backend_mcp_list_fails_closed_when_read_capability_is_unavailable() {
    if claude_mcp_list_supported_on_target() {
        return;
    }

    let backend = ClaudeCodeBackend::new(ClaudeCodeBackendConfig::default());
    let err = backend
        .mcp_list(crate::mcp::AgentWrapperMcpListRequest::default())
        .await
        .expect_err("unsupported target should fail closed");

    match err {
        AgentWrapperError::UnsupportedCapability {
            agent_kind,
            capability,
        } => {
            assert_eq!(agent_kind, "claude_code");
            assert_eq!(capability, CAPABILITY_MCP_LIST_V1);
        }
        other => panic!("expected UnsupportedCapability, got: {other:?}"),
    }
}

#[tokio::test]
async fn claude_backend_mcp_get_fails_closed_when_read_capability_is_unavailable() {
    if claude_mcp_get_supported_on_target() {
        return;
    }

    let backend = ClaudeCodeBackend::new(ClaudeCodeBackendConfig::default());
    let err = backend
        .mcp_get(crate::mcp::AgentWrapperMcpGetRequest {
            name: "demo".to_string(),
            context: Default::default(),
        })
        .await
        .expect_err("unsupported target should fail closed");

    match err {
        AgentWrapperError::UnsupportedCapability {
            agent_kind,
            capability,
        } => {
            assert_eq!(agent_kind, "claude_code");
            assert_eq!(capability, CAPABILITY_MCP_GET_V1);
        }
        other => panic!("expected UnsupportedCapability, got: {other:?}"),
    }
}

#[tokio::test]
async fn claude_backend_mcp_add_fails_closed_when_write_capability_is_disabled() {
    let backend = ClaudeCodeBackend::new(ClaudeCodeBackendConfig::default());
    let err = backend
        .mcp_add(sample_mcp_add_request())
        .await
        .expect_err("write support should stay disabled by default");

    match err {
        AgentWrapperError::UnsupportedCapability {
            agent_kind,
            capability,
        } => {
            assert_eq!(agent_kind, "claude_code");
            assert_eq!(capability, CAPABILITY_MCP_ADD_V1);
        }
        other => panic!("expected UnsupportedCapability, got: {other:?}"),
    }
}

#[tokio::test]
async fn claude_backend_mcp_remove_fails_closed_when_write_capability_is_disabled() {
    let backend = ClaudeCodeBackend::new(ClaudeCodeBackendConfig::default());
    let err = backend
        .mcp_remove(sample_mcp_remove_request())
        .await
        .expect_err("write support should stay disabled by default");

    match err {
        AgentWrapperError::UnsupportedCapability {
            agent_kind,
            capability,
        } => {
            assert_eq!(agent_kind, "claude_code");
            assert_eq!(capability, CAPABILITY_MCP_REMOVE_V1);
        }
        other => panic!("expected UnsupportedCapability, got: {other:?}"),
    }
}

#[test]
fn claude_backend_registers_under_claude_code_kind_id() {
    let backend = ClaudeCodeBackend::new(ClaudeCodeBackendConfig::default());
    assert_eq!(backend.kind().as_str(), "claude_code");
}
