use std::{collections::BTreeMap, sync::Arc};

use agent_api::{
    backends::codex::{CodexBackend, CodexBackendConfig},
    mcp::{
        AgentWrapperMcpAddRequest, AgentWrapperMcpAddTransport, AgentWrapperMcpCommandContext,
        AgentWrapperMcpRemoveRequest,
    },
    AgentWrapperBackend, AgentWrapperError, AgentWrapperGateway, AgentWrapperKind,
};

use super::support::{collect_fake_mcp_sentinels, fake_mcp_sentinel, McpTestSandbox};

const CAPABILITY_MCP_ADD_V1: &str = "agent_api.tools.mcp.add.v1";
const CAPABILITY_MCP_REMOVE_V1: &str = "agent_api.tools.mcp.remove.v1";
const FAKE_CODEX_RECORD_PATH_ENV: &str = "FAKE_CODEX_MCP_RECORD_PATH";
const FAKE_CODEX_RECORD_ENV_KEYS_ENV: &str = "FAKE_CODEX_MCP_RECORD_ENV_KEYS";
const ALL_RECORDED_ENV_KEYS: &str =
    "CLI_ONLY,CONFIG_ONLY,OVERRIDE_ME,REQUEST_ONLY,MY_TOKEN,MCP_SERVER_ENV";

#[tokio::test]
async fn codex_mcp_add_fails_closed_without_write_enablement() {
    if !codex_mcp_supported() {
        return;
    }

    let sandbox = McpTestSandbox::new("codex_mcp_add_fails_closed").expect("sandbox");
    let (_backend, gateway, kind) = codex_gateway(
        &sandbox,
        false,
        codex_config_env(&sandbox, std::iter::empty()),
    );

    let err = gateway
        .mcp_add(
            &kind,
            AgentWrapperMcpAddRequest {
                name: "demo".to_string(),
                transport: AgentWrapperMcpAddTransport::Stdio {
                    command: vec!["node".to_string()],
                    args: vec!["server.js".to_string()],
                    env: BTreeMap::from([("MCP_SERVER_ENV".to_string(), "1".to_string())]),
                },
                context: AgentWrapperMcpCommandContext::default(),
            },
        )
        .await
        .expect_err("write-disabled add should fail closed");

    assert_unsupported_capability(err, CAPABILITY_MCP_ADD_V1);
    assert!(
        !sandbox.record_path().exists(),
        "write-disabled add should not spawn the fake codex binary"
    );
}

#[tokio::test]
async fn codex_mcp_remove_fails_closed_without_write_enablement() {
    if !codex_mcp_supported() {
        return;
    }

    let sandbox = McpTestSandbox::new("codex_mcp_remove_fails_closed").expect("sandbox");
    let (_backend, gateway, kind) = codex_gateway(
        &sandbox,
        false,
        codex_config_env(&sandbox, std::iter::empty()),
    );

    let err = gateway
        .mcp_remove(
            &kind,
            AgentWrapperMcpRemoveRequest {
                name: "demo".to_string(),
                context: AgentWrapperMcpCommandContext::default(),
            },
        )
        .await
        .expect_err("write-disabled remove should fail closed");

    assert_unsupported_capability(err, CAPABILITY_MCP_REMOVE_V1);
    assert!(
        !sandbox.record_path().exists(),
        "write-disabled remove should not spawn the fake codex binary"
    );
}

#[tokio::test]
async fn codex_mcp_add_stdio_uses_typed_transport_without_leaking_server_env() {
    if !codex_mcp_supported() {
        return;
    }

    let sandbox = McpTestSandbox::new("codex_mcp_add_stdio_records").expect("sandbox");
    let (_backend, gateway, kind) = codex_gateway(
        &sandbox,
        true,
        codex_config_env(
            &sandbox,
            [("CLI_ONLY".to_string(), "config-cli".to_string())],
        ),
    );

    let output = gateway
        .mcp_add(
            &kind,
            AgentWrapperMcpAddRequest {
                name: "demo".to_string(),
                transport: AgentWrapperMcpAddTransport::Stdio {
                    command: vec!["node".to_string()],
                    args: vec!["server.js".to_string()],
                    env: BTreeMap::from([("MCP_SERVER_ENV".to_string(), "1".to_string())]),
                },
                context: AgentWrapperMcpCommandContext {
                    env: BTreeMap::from([("CLI_ONLY".to_string(), "cli-value".to_string())]),
                    ..Default::default()
                },
            },
        )
        .await
        .expect("stdio add should succeed");

    assert!(output.status.success(), "expected success status");

    let record = sandbox
        .read_single_record()
        .expect("single invocation record");
    assert_eq!(
        record.args,
        vec![
            "mcp",
            "add",
            "demo",
            "--env",
            "MCP_SERVER_ENV=1",
            "--",
            "node",
            "server.js",
        ]
    );
    assert!(
        !record.env.contains_key("MCP_SERVER_ENV"),
        "stdio transport env must not leak into the CLI process env"
    );
    assert_eq!(
        record.env.get("CLI_ONLY").map(String::as_str),
        Some("cli-value")
    );

    let expected = fake_mcp_sentinel(sandbox.codex_home(), "add");
    assert!(
        expected.is_file(),
        "expected add sentinel at {:?}",
        expected
    );
    assert_eq!(
        collect_fake_mcp_sentinels(sandbox.root()).expect("collect sentinels"),
        vec![expected]
    );
}

#[tokio::test]
async fn codex_mcp_add_url_records_bearer_env_var_without_exposing_secret_in_argv() {
    if !codex_mcp_supported() {
        return;
    }

    let sandbox = McpTestSandbox::new("codex_mcp_add_url_records").expect("sandbox");
    let (_backend, gateway, kind) = codex_gateway(
        &sandbox,
        true,
        codex_config_env(&sandbox, std::iter::empty()),
    );

    let output = gateway
        .mcp_add(
            &kind,
            AgentWrapperMcpAddRequest {
                name: "demo".to_string(),
                transport: AgentWrapperMcpAddTransport::Url {
                    url: "https://example.test/mcp".to_string(),
                    bearer_token_env_var: Some("MY_TOKEN".to_string()),
                },
                context: AgentWrapperMcpCommandContext {
                    env: BTreeMap::from([("MY_TOKEN".to_string(), "SECRET".to_string())]),
                    ..Default::default()
                },
            },
        )
        .await
        .expect("url add should succeed");

    assert!(output.status.success(), "expected success status");

    let record = sandbox
        .read_single_record()
        .expect("single invocation record");
    assert_eq!(
        record.args,
        vec![
            "mcp",
            "add",
            "demo",
            "--url",
            "https://example.test/mcp",
            "--bearer-token-env-var",
            "MY_TOKEN",
        ]
    );
    assert!(
        record.args.iter().all(|arg| arg != "SECRET"),
        "argv must not contain the bearer token secret"
    );
    assert_eq!(
        record.env.get("MY_TOKEN").map(String::as_str),
        Some("SECRET")
    );

    let expected = fake_mcp_sentinel(sandbox.codex_home(), "add");
    assert!(
        expected.is_file(),
        "expected add sentinel at {:?}",
        expected
    );
    assert_eq!(
        collect_fake_mcp_sentinels(sandbox.root()).expect("collect sentinels"),
        vec![expected]
    );
}

#[tokio::test]
async fn codex_mcp_remove_records_pinned_argv_and_writes_sentinel() {
    if !codex_mcp_supported() {
        return;
    }

    let sandbox = McpTestSandbox::new("codex_mcp_remove_records").expect("sandbox");
    let (_backend, gateway, kind) = codex_gateway(
        &sandbox,
        true,
        codex_config_env(&sandbox, std::iter::empty()),
    );

    let output = gateway
        .mcp_remove(
            &kind,
            AgentWrapperMcpRemoveRequest {
                name: "demo".to_string(),
                context: AgentWrapperMcpCommandContext::default(),
            },
        )
        .await
        .expect("remove should succeed");

    assert!(output.status.success(), "expected success status");

    let record = sandbox
        .read_single_record()
        .expect("single invocation record");
    assert_eq!(record.args, vec!["mcp", "remove", "demo"]);

    let expected = fake_mcp_sentinel(sandbox.codex_home(), "remove");
    assert!(
        expected.is_file(),
        "expected remove sentinel at {:?}",
        expected
    );
    assert_eq!(
        collect_fake_mcp_sentinels(sandbox.root()).expect("collect sentinels"),
        vec![expected]
    );
}

fn codex_gateway(
    sandbox: &McpTestSandbox,
    allow_mcp_write: bool,
    env: BTreeMap<String, String>,
) -> (Arc<CodexBackend>, AgentWrapperGateway, AgentWrapperKind) {
    let binary = sandbox.install_fake_codex().expect("install fake codex");
    let backend = Arc::new(CodexBackend::new(CodexBackendConfig {
        binary: Some(binary),
        codex_home: Some(sandbox.codex_home().to_path_buf()),
        env,
        allow_mcp_write,
        ..Default::default()
    }));

    let kind = backend.kind();
    let mut gateway = AgentWrapperGateway::new();
    gateway
        .register(backend.clone())
        .expect("register codex backend");
    (backend, gateway, kind)
}

fn codex_config_env(
    sandbox: &McpTestSandbox,
    extra: impl IntoIterator<Item = (String, String)>,
) -> BTreeMap<String, String> {
    let mut env = BTreeMap::from([
        (
            FAKE_CODEX_RECORD_PATH_ENV.to_string(),
            sandbox.record_path().to_string_lossy().into_owned(),
        ),
        (
            FAKE_CODEX_RECORD_ENV_KEYS_ENV.to_string(),
            ALL_RECORDED_ENV_KEYS.to_string(),
        ),
    ]);
    env.extend(extra);
    env
}

fn assert_unsupported_capability(err: AgentWrapperError, expected_capability: &str) {
    match err {
        AgentWrapperError::UnsupportedCapability {
            agent_kind,
            capability,
        } => {
            assert_eq!(agent_kind, "codex");
            assert_eq!(capability, expected_capability);
        }
        other => panic!("expected UnsupportedCapability, got {other:?}"),
    }
}

fn codex_mcp_supported() -> bool {
    cfg!(all(
        target_os = "linux",
        target_arch = "x86_64",
        target_env = "musl"
    ))
}
