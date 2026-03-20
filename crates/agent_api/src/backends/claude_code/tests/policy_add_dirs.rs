use std::fs;

use tempfile::tempdir;

use super::support::*;

const EXT_ADD_DIRS_V1: &str = "agent_api.exec.add_dirs.v1";

#[test]
fn claude_policy_add_dirs_absent_key_returns_empty_vec() {
    let adapter = new_adapter();
    let request = AgentWrapperRunRequest {
        prompt: "hello".to_string(),
        ..Default::default()
    };

    let policy = adapter
        .validate_and_extract_policy(&request)
        .expect("policy extraction should succeed");

    assert!(policy.add_dirs.is_empty());
}

#[test]
fn claude_policy_add_dirs_request_working_dir_beats_default_and_run_start_cwd() {
    let temp = tempdir().expect("tempdir");
    let request_root = temp.path().join("request-root");
    let default_root = temp.path().join("default-root");
    let run_start_root = temp.path().join("run-start-root");
    let request_docs = request_root.join("docs");
    let default_docs = default_root.join("docs");
    let run_start_docs = run_start_root.join("docs");
    fs::create_dir_all(&request_docs).expect("create request docs");
    fs::create_dir_all(&default_docs).expect("create default docs");
    fs::create_dir_all(&run_start_docs).expect("create run-start docs");

    let adapter = new_adapter_with_config_and_run_start_cwd(
        ClaudeCodeBackendConfig {
            default_working_dir: Some(default_root),
            ..Default::default()
        },
        Some(run_start_root),
    );
    let request = AgentWrapperRunRequest {
        prompt: "hello".to_string(),
        working_dir: Some(request_root),
        extensions: [(EXT_ADD_DIRS_V1.to_string(), add_dirs_payload(&["docs"]))]
            .into_iter()
            .collect(),
        ..Default::default()
    };

    let policy = adapter
        .validate_and_extract_policy(&request)
        .expect("policy extraction should succeed");

    assert_eq!(policy.add_dirs, vec![request_docs]);
}

#[test]
fn claude_policy_add_dirs_backend_default_beats_run_start_cwd() {
    let temp = tempdir().expect("tempdir");
    let default_root = temp.path().join("default-root");
    let run_start_root = temp.path().join("run-start-root");
    let default_docs = default_root.join("docs");
    let run_start_docs = run_start_root.join("docs");
    fs::create_dir_all(&default_docs).expect("create default docs");
    fs::create_dir_all(&run_start_docs).expect("create run-start docs");

    let adapter = new_adapter_with_config_and_run_start_cwd(
        ClaudeCodeBackendConfig {
            default_working_dir: Some(default_root),
            ..Default::default()
        },
        Some(run_start_root),
    );
    let request = AgentWrapperRunRequest {
        prompt: "hello".to_string(),
        extensions: [(EXT_ADD_DIRS_V1.to_string(), add_dirs_payload(&["docs"]))]
            .into_iter()
            .collect(),
        ..Default::default()
    };

    let policy = adapter
        .validate_and_extract_policy(&request)
        .expect("policy extraction should succeed");

    assert_eq!(policy.add_dirs, vec![default_docs]);
}

#[test]
fn claude_policy_add_dirs_run_start_cwd_is_final_fallback() {
    let temp = tempdir().expect("tempdir");
    let run_start_root = temp.path().join("run-start-root");
    let run_start_docs = run_start_root.join("docs");
    fs::create_dir_all(&run_start_docs).expect("create run-start docs");

    let adapter = new_adapter_with_run_start_cwd(Some(run_start_root));
    let request = AgentWrapperRunRequest {
        prompt: "hello".to_string(),
        extensions: [(EXT_ADD_DIRS_V1.to_string(), add_dirs_payload(&["docs"]))]
            .into_iter()
            .collect(),
        ..Default::default()
    };

    let policy = adapter
        .validate_and_extract_policy(&request)
        .expect("policy extraction should succeed");

    assert_eq!(policy.add_dirs, vec![run_start_docs]);
}

#[test]
fn claude_policy_add_dirs_invalid_input_uses_safe_message_without_leakage() {
    let temp = tempdir().expect("tempdir");
    let request_root = temp.path().join("request-root");
    fs::create_dir_all(&request_root).expect("create request root");
    let leaked = "missing-secret-dir";
    let request = AgentWrapperRunRequest {
        prompt: "hello".to_string(),
        working_dir: Some(request_root),
        extensions: [(EXT_ADD_DIRS_V1.to_string(), add_dirs_payload(&[leaked]))]
            .into_iter()
            .collect(),
        ..Default::default()
    };

    let err = adapter_error(new_adapter().validate_and_extract_policy(&request));
    match &err {
        AgentWrapperError::InvalidRequest { message } => {
            assert_eq!(message, "invalid agent_api.exec.add_dirs.v1.dirs[0]");
        }
        other => panic!("expected InvalidRequest, got: {other:?}"),
    }
    assert!(
        !err.to_string().contains(leaked),
        "error display leaked raw path text"
    );
}

fn adapter_error(
    result: Result<super::super::harness::ClaudeExecPolicy, AgentWrapperError>,
) -> AgentWrapperError {
    result.expect_err("policy extraction should fail")
}
