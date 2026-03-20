use super::support::*;

#[test]
fn claude_adapter_implements_backend_harness_adapter_contract() {
    fn assert_impl<T: crate::backend_harness::BackendHarnessAdapter>() {}
    assert_impl::<ClaudeHarnessAdapter>();
}

#[test]
fn claude_backend_routes_through_harness_and_does_not_reintroduce_orchestration_primitives() {
    const SOURCE: &str = include_str!("../backend.rs");

    assert!(
        SOURCE.contains("run_harnessed_backend("),
        "expected Claude backend to route through the harness entrypoint"
    );
    assert!(
        SOURCE.contains("run_harnessed_backend_control("),
        "expected Claude backend to route cancellation through the harness control entrypoint"
    );
    assert!(
        SOURCE.contains("TerminationState::new"),
        "expected Claude backend control path to register a termination hook"
    );

    assert!(
        !SOURCE.contains("build_gated_run_handle("),
        "expected Claude backend to not bypass harness-owned completion gating"
    );
    assert!(
        !SOURCE.contains("mpsc::channel::<AgentWrapperEvent>(32)"),
        "expected Claude backend to not create a backend-local events channel"
    );
    assert!(
        !SOURCE.contains("tokio::time::timeout("),
        "expected Claude backend to not wrap runs with backend-local timeout orchestration"
    );
}

#[test]
fn claude_backend_mcp_write_hooks_route_through_shared_mcp_runner() {
    const SOURCE: &str = include_str!("../backend.rs");

    assert!(SOURCE.contains("fn mcp_add("));
    assert!(SOURCE.contains("mcp_management::claude_mcp_add_argv"));
    assert!(SOURCE.contains("fn mcp_remove("));
    assert!(SOURCE.contains("mcp_management::claude_mcp_remove_argv"));
    assert!(
        SOURCE.matches("mcp_management::run_claude_mcp(").count() >= 4,
        "expected list/get/add/remove hooks to reuse the shared Claude MCP runner"
    );
}

#[test]
fn claude_downstream_mapping_surfaces_do_not_reopen_raw_add_dirs_parsing() {
    const RAW_KEY: &str = "agent_api.exec.add_dirs.v1";
    const BACKEND_SOURCE: &str = include_str!("../backend.rs");
    const MAPPING_SOURCE: &str = include_str!("../mapping.rs");
    const MCP_ARGV_SOURCE: &str = include_str!("../mcp_management/argv.rs");
    const MCP_RESOLVE_SOURCE: &str = include_str!("../mcp_management/resolve.rs");
    const MCP_RUNNER_SOURCE: &str = include_str!("../mcp_management/runner.rs");

    assert!(
        !BACKEND_SOURCE.contains(RAW_KEY),
        "expected backend.rs to avoid reopening raw add-dir payload parsing"
    );
    assert!(
        !MAPPING_SOURCE.contains(RAW_KEY),
        "expected mapping.rs to avoid reopening raw add-dir payload parsing"
    );
    assert!(
        !MCP_ARGV_SOURCE.contains(RAW_KEY),
        "expected mcp argv helpers to avoid raw add-dir payload parsing"
    );
    assert!(
        !MCP_RESOLVE_SOURCE.contains(RAW_KEY),
        "expected mcp resolve helpers to avoid raw add-dir payload parsing"
    );
    assert!(
        !MCP_RUNNER_SOURCE.contains(RAW_KEY),
        "expected mcp runner helpers to avoid raw add-dir payload parsing"
    );
}
