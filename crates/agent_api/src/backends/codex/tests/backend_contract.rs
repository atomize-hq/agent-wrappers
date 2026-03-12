use super::support::*;
use codex::ExecStreamError;

#[test]
fn codex_adapter_implements_backend_harness_adapter_contract() {
    fn assert_impl<T: crate::backend_harness::BackendHarnessAdapter>() {}
    assert_impl::<CodexHarnessAdapter>();
}

#[test]
fn codex_backend_routes_through_harness_and_does_not_reintroduce_orchestration_primitives() {
    const SOURCE: &str = include_str!("../backend.rs");

    assert!(
        SOURCE.contains("run_harnessed_backend("),
        "expected Codex backend to route through the harness entrypoint"
    );
    assert!(
        SOURCE.contains("run_harnessed_backend_control("),
        "expected Codex backend to route cancellation through the harness control entrypoint"
    );
    assert!(
        SOURCE.contains("TerminationState::new"),
        "expected Codex backend control path to register a termination hook"
    );

    assert!(
        !SOURCE.contains("build_gated_run_handle("),
        "expected Codex backend to not bypass harness-owned completion gating"
    );
    assert!(
        !SOURCE.contains("mpsc::channel::<AgentWrapperEvent>(32)"),
        "expected Codex backend to not create a backend-local events channel"
    );
    assert!(
        !SOURCE.contains("tokio::time::timeout("),
        "expected Codex backend to not wrap runs with backend-local timeout orchestration"
    );
}

#[test]
fn codex_backend_mcp_write_hooks_route_through_shared_mcp_runner() {
    const SOURCE: &str = include_str!("../backend.rs");

    assert!(SOURCE.contains("fn mcp_add("));
    assert!(SOURCE.contains("mcp_management::codex_mcp_add_argv"));
    assert!(SOURCE.contains("fn mcp_remove("));
    assert!(SOURCE.contains("mcp_management::codex_mcp_remove_argv"));
    assert!(
        SOURCE.matches("mcp_management::run_codex_mcp(").count() >= 4,
        "expected list/get/add/remove hooks to reuse the shared Codex MCP runner"
    );
}

#[test]
fn redact_exec_stream_error_does_not_leak_raw_jsonl_line() {
    let secret = "RAW-LINE-SECRET-DO-NOT-LEAK";
    let err = ExecStreamError::Normalize {
        line: secret.to_string(),
        message: "missing required context".to_string(),
    };

    let redacted = redact_exec_stream_error(&err);
    assert!(!redacted.contains(secret));
    assert!(redacted.contains("line_bytes="));
}
