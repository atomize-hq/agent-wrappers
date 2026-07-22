#![cfg(unix)]

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

use codex::{
    AppServerCodegenRequest, AppServerProxyRequest, AppServerRequest, CodexClient, CodexError,
    DebugAppServerSendMessageV2Request, DebugModelsRequest, DebugPromptInputRequest,
    ExecServerRequest, FeaturesDisableRequest, FeaturesEnableRequest, NonTuiCommand,
    NonTuiCommandRequest, PluginCommandRequest, PluginMarketplaceAddRequest,
    PluginMarketplaceCommandRequest, PluginMarketplaceRemoveRequest,
    PluginMarketplaceUpgradeRequest, SandboxCommandRequest, SandboxPlatform, UpdateCommandRequest,
};
use serde::Deserialize;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Debug, Deserialize)]
struct Invocation {
    argv: Vec<String>,
}

#[tokio::test]
async fn features_enable_disable_spawn_expected_subcommands(
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let log_path = temp.path().join("invocations.jsonl");
    let fake_codex = write_fake_codex(&log_path)?;

    let client = CodexClient::builder()
        .binary(&fake_codex)
        .mirror_stdout(false)
        .quiet(true)
        .build();

    client
        .features_enable(FeaturesEnableRequest::new("unified_exec"))
        .await?;
    client
        .features_disable(FeaturesDisableRequest::new("unified_exec"))
        .await?;

    let invocations = read_invocations(&log_path)?;
    assert!(
        invocations
            .iter()
            .any(|inv| inv.argv == ["features", "enable", "unified_exec"]),
        "missing features enable invocation: {:?}",
        invocations
            .iter()
            .map(|inv| inv.argv.as_slice())
            .collect::<Vec<_>>()
    );
    assert!(
        invocations
            .iter()
            .any(|inv| inv.argv == ["features", "disable", "unified_exec"]),
        "missing features disable invocation: {:?}",
        invocations
            .iter()
            .map(|inv| inv.argv.as_slice())
            .collect::<Vec<_>>()
    );

    Ok(())
}

#[tokio::test]
async fn debug_app_server_send_message_v2_spawns_expected_subcommand(
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let log_path = temp.path().join("invocations.jsonl");
    let fake_codex = write_fake_codex(&log_path)?;

    let client = CodexClient::builder()
        .binary(&fake_codex)
        .mirror_stdout(false)
        .quiet(true)
        .build();

    client
        .debug_app_server_send_message_v2(DebugAppServerSendMessageV2Request::new("hello"))
        .await?;

    let invocations = read_invocations(&log_path)?;
    assert!(
        invocations
            .iter()
            .any(|inv| inv.argv == ["debug", "app-server", "send-message-v2", "hello"]),
        "missing debug send-message-v2 invocation: {:?}",
        invocations
            .iter()
            .map(|inv| inv.argv.as_slice())
            .collect::<Vec<_>>()
    );
    Ok(())
}

#[tokio::test]
async fn app_server_codegen_experimental_emits_flag() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let log_path = temp.path().join("invocations.jsonl");
    let fake_codex = write_fake_codex(&log_path)?;

    let client = CodexClient::builder()
        .binary(&fake_codex)
        .mirror_stdout(false)
        .quiet(true)
        .build();

    let out_dir = temp.path().join("app-server-schema");
    client
        .generate_app_server_bindings(
            AppServerCodegenRequest::json_schema(&out_dir).experimental(true),
        )
        .await?;

    let invocations = read_invocations(&log_path)?;
    let invocation = invocations
        .iter()
        .find(|inv| inv.argv.first().map(|v| v.as_str()) == Some("app-server"))
        .expect("expected an app-server invocation");

    assert!(
        invocation.argv.iter().any(|arg| arg == "--experimental"),
        "--experimental missing from argv: {:?}",
        invocation.argv
    );

    Ok(())
}

#[tokio::test]
async fn new_0125_surfaces_spawn_expected_subcommands() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let log_path = temp.path().join("invocations.jsonl");
    let fake_codex = write_fake_codex(&log_path)?;

    let client = CodexClient::builder()
        .binary(&fake_codex)
        .mirror_stdout(false)
        .quiet(true)
        .build();

    client
        .debug_models(DebugModelsRequest::new().bundled(true))
        .await?;
    client
        .debug_prompt_input(
            DebugPromptInputRequest::new()
                .image(temp.path().join("one.png"))
                .image(temp.path().join("two.png"))
                .prompt("hello"),
        )
        .await?;
    client.plugin(PluginCommandRequest::new()).await?;
    client
        .plugin_marketplace(PluginMarketplaceCommandRequest::new())
        .await?;
    client
        .plugin_marketplace_add(
            PluginMarketplaceAddRequest::new("owner/repo")
                .source_ref("main")
                .sparse_path("marketplaces/core"),
        )
        .await?;
    client
        .plugin_marketplace_remove(PluginMarketplaceRemoveRequest::new("primary"))
        .await?;
    client
        .plugin_marketplace_upgrade(
            PluginMarketplaceUpgradeRequest::new().marketplace_name("primary"),
        )
        .await?;

    let mut app_server_proxy = client.start_app_server_proxy(
        AppServerProxyRequest::new().socket_path(temp.path().join("app-server.sock")),
    )?;
    let app_server_proxy_status = app_server_proxy.wait().await?;
    assert!(app_server_proxy_status.success());

    let mut app_server = client.start_app_server(
        AppServerRequest::new()
            .listen("127.0.0.1:9090")
            .ws_audience("aud")
            .ws_auth("shared-secret")
            .ws_issuer("issuer")
            .ws_max_clock_skew_seconds(15)
            .ws_shared_secret_file(temp.path().join("shared.secret"))
            .ws_token_file(temp.path().join("token.jwt"))
            .ws_token_sha256("abc123"),
    )?;
    let app_server_status = app_server.wait().await?;
    assert!(app_server_status.success());

    let mut exec_server = client.start_exec_server(ExecServerRequest::new().listen("stdio"))?;
    let exec_server_status = exec_server.wait().await?;
    assert!(exec_server_status.success());

    let invocations = read_invocations(&log_path)?;
    let argv_sets: Vec<_> = invocations
        .iter()
        .map(|inv| inv.argv.as_slice())
        .collect::<Vec<_>>();

    assert!(
        invocations
            .iter()
            .any(|inv| inv.argv == ["debug", "models", "--bundled"]),
        "missing debug models invocation: {:?}",
        argv_sets
    );
    assert!(
        invocations.iter().any(|inv| {
            inv.argv.first().map(|value| value.as_str()) == Some("debug")
                && inv.argv.get(1).map(|value| value.as_str()) == Some("prompt-input")
                && inv.argv.iter().any(|value| value == "--image")
                && inv.argv.last().map(|value| value.as_str()) == Some("hello")
        }),
        "missing debug prompt-input invocation: {:?}",
        argv_sets
    );
    assert!(
        invocations.iter().any(|inv| inv.argv == ["plugin"]),
        "missing plugin invocation: {:?}",
        argv_sets
    );
    assert!(
        invocations
            .iter()
            .any(|inv| inv.argv == ["plugin", "marketplace"]),
        "missing plugin marketplace invocation: {:?}",
        argv_sets
    );
    assert!(
        invocations.iter().any(|inv| {
            inv.argv
                == [
                    "plugin",
                    "marketplace",
                    "add",
                    "--ref",
                    "main",
                    "--sparse",
                    "marketplaces/core",
                    "owner/repo",
                ]
        }),
        "missing plugin marketplace add invocation: {:?}",
        argv_sets
    );
    assert!(
        invocations
            .iter()
            .any(|inv| { inv.argv == ["plugin", "marketplace", "remove", "primary"] }),
        "missing plugin marketplace remove invocation: {:?}",
        argv_sets
    );
    assert!(
        invocations
            .iter()
            .any(|inv| { inv.argv == ["plugin", "marketplace", "upgrade", "primary"] }),
        "missing plugin marketplace upgrade invocation: {:?}",
        argv_sets
    );
    assert!(
        invocations.iter().any(|inv| {
            inv.argv.first().map(|value| value.as_str()) == Some("app-server")
                && inv.argv.get(1).map(|value| value.as_str()) == Some("proxy")
                && inv.argv.iter().any(|value| value == "--sock")
        }),
        "missing app-server proxy invocation: {:?}",
        argv_sets
    );
    assert!(
        invocations.iter().any(|inv| {
            inv.argv.first().map(|value| value.as_str()) == Some("app-server")
                && inv.argv.get(1).map(|value| value.as_str()) != Some("proxy")
                && inv.argv.iter().any(|value| value == "--listen")
                && inv.argv.iter().any(|value| value == "--ws-audience")
                && inv.argv.iter().any(|value| value == "--ws-auth")
                && inv.argv.iter().any(|value| value == "--ws-issuer")
                && inv
                    .argv
                    .iter()
                    .any(|value| value == "--ws-max-clock-skew-seconds")
                && inv
                    .argv
                    .iter()
                    .any(|value| value == "--ws-shared-secret-file")
                && inv.argv.iter().any(|value| value == "--ws-token-file")
                && inv.argv.iter().any(|value| value == "--ws-token-sha256")
        }),
        "missing app-server invocation: {:?}",
        argv_sets
    );
    assert!(
        invocations.iter().any(|inv| {
            inv.argv.first().map(|value| value.as_str()) == Some("exec-server")
                && inv.argv.iter().any(|value| value == "--listen")
        }),
        "missing exec-server listen invocation: {:?}",
        argv_sets
    );

    Ok(())
}

#[tokio::test]
async fn packet_non_tui_commands_are_bounded_and_forward_arguments(
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let log_path = temp.path().join("invocations.jsonl");
    let fake_codex = write_fake_codex(&log_path)?;
    let client = CodexClient::builder()
        .binary(&fake_codex)
        .mirror_stdout(false)
        .quiet(true)
        .build();

    for command in NonTuiCommand::all() {
        let request = NonTuiCommandRequest::new(*command).args(non_tui_passthrough_args(*command));
        if matches!(
            command,
            NonTuiCommand::AppServer | NonTuiCommand::ExecServer
        ) {
            let mut child = client.start_non_tui_server(request)?;
            assert!(child.wait().await?.success());
        } else {
            client.run_non_tui_command(request).await?;
        }
    }
    client
        .run_non_tui_command(
            NonTuiCommandRequest::new(NonTuiCommand::RemoteControlHelp)
                .arg("sessions")
                .dangerously_bypass_hook_trust(true)
                .strict_config(true),
        )
        .await?;

    let invocations = read_invocations(&log_path)?;
    assert_eq!(invocations.len(), NonTuiCommand::all().len() + 1);

    for (command, invocation) in NonTuiCommand::all()
        .iter()
        .zip(invocations.iter().take(NonTuiCommand::all().len()))
    {
        let expected_path = command.path();
        let expected_args = non_tui_passthrough_args(*command);
        let actual_path = invocation.argv[..expected_path.len()]
            .iter()
            .map(|value| value.as_str())
            .collect::<Vec<_>>();
        let actual_args = invocation.argv[expected_path.len()..]
            .iter()
            .map(|value| value.as_str())
            .collect::<Vec<_>>();

        assert_eq!(
            actual_path, expected_path,
            "command path must lead argv for {command:?}: {:?}",
            invocation.argv
        );
        assert_eq!(
            actual_args, expected_args,
            "verbatim args must immediately follow command path for {command:?}: {:?}",
            invocation.argv
        );
    }

    assert_eq!(
        invocations.last().expect("root-flag invocation").argv,
        [
            "--dangerously-bypass-hook-trust",
            "--strict-config",
            "remote-control",
            "help",
            "sessions",
        ]
    );
    Ok(())
}

#[tokio::test]
async fn packet_non_tui_parent_variants_reject_bare_descendant_tokens(
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let log_path = temp.path().join("invocations.jsonl");
    let fake_codex = write_fake_codex(&log_path)?;
    let client = CodexClient::builder()
        .binary(&fake_codex)
        .mirror_stdout(false)
        .quiet(true)
        .build();

    for (command, bare_token) in [
        (NonTuiCommand::AppServer, "proxy"),
        (NonTuiCommand::AppServerDaemon, "help"),
        (NonTuiCommand::RemoteControl, "start"),
    ] {
        let error = client
            .run_non_tui_command(NonTuiCommandRequest::new(command).arg(bare_token))
            .await
            .expect_err("parent variant should reject bare descendant token");

        match error {
            CodexError::InvalidNonTuiPassthrough {
                command: actual_command,
                token,
            } => {
                assert_eq!(actual_command, command.path().join(" "));
                assert_eq!(token, bare_token);
            }
            other => panic!("unexpected error for {command:?}: {other:?}"),
        }
    }

    assert!(
        !log_path.exists(),
        "validation should reject before spawn, but got invocations: {:?}",
        read_invocations(&log_path).unwrap_or_default()
    );
    Ok(())
}

#[tokio::test]
async fn packet_non_tui_parent_variants_accept_flag_only_passthrough_arguments(
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let log_path = temp.path().join("invocations.jsonl");
    let fake_codex = write_fake_codex(&log_path)?;
    let client = CodexClient::builder()
        .binary(&fake_codex)
        .mirror_stdout(false)
        .quiet(true)
        .build();

    let mut child = client.start_non_tui_server(
        NonTuiCommandRequest::new(NonTuiCommand::AppServer).arg("--listen=127.0.0.1:9090"),
    )?;
    assert!(child.wait().await?.success());

    let invocations = read_invocations(&log_path)?;
    assert_eq!(invocations.len(), 1);
    assert_eq!(
        invocations[0].argv,
        ["app-server", "--listen=127.0.0.1:9090"]
    );
    Ok(())
}

#[tokio::test]
async fn packet_non_tui_parent_variant_predicate_edge_cases_preserve_spawn_behavior(
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let log_path = temp.path().join("invocations.jsonl");
    let fake_codex = write_fake_codex(&log_path)?;
    let client = CodexClient::builder()
        .binary(&fake_codex)
        .mirror_stdout(false)
        .quiet(true)
        .build();

    let mut dash_child = client
        .start_non_tui_server(NonTuiCommandRequest::new(NonTuiCommand::AppServer).arg("-"))?;
    assert!(dash_child.wait().await?.success());
    let mut double_dash_child = client
        .start_non_tui_server(NonTuiCommandRequest::new(NonTuiCommand::AppServer).arg("--"))?;
    assert!(double_dash_child.wait().await?.success());

    let error = client
        .run_non_tui_command(NonTuiCommandRequest::new(NonTuiCommand::AppServer).arg(""))
        .await
        .expect_err("empty passthrough should be rejected before spawn");

    match error {
        CodexError::InvalidNonTuiPassthrough { command, token } => {
            assert_eq!(command, NonTuiCommand::AppServer.path().join(" "));
            assert_eq!(token, "");
        }
        other => panic!("unexpected error for empty passthrough: {other:?}"),
    }

    let invocations = read_invocations(&log_path)?;
    assert_eq!(invocations.len(), 2);
    assert_eq!(invocations[0].argv, ["app-server", "-"]);
    assert_eq!(invocations[1].argv, ["app-server", "--"]);

    Ok(())
}

#[tokio::test]
async fn packet_non_tui_servers_return_piped_handles_for_protocol_passthrough(
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let log_path = temp.path().join("invocations.jsonl");
    let fake_codex = write_fake_codex(&log_path)?;
    let client = CodexClient::builder()
        .binary(&fake_codex)
        .mirror_stdout(false)
        .quiet(true)
        .build();

    for (command, arguments) in [
        (
            NonTuiCommand::AppServer,
            vec!["--stdio", "--environment-id=environment-123"],
        ),
        (
            NonTuiCommand::ExecServer,
            vec!["--stdio", "--use-agent-identity-auth"],
        ),
    ] {
        let mut child =
            client.start_non_tui_server(NonTuiCommandRequest::new(command).args(arguments))?;
        let mut stdin = child.stdin.take().expect("server stdin must be piped");
        let mut stdout = child.stdout.take().expect("server stdout must be piped");
        assert!(child.stderr.take().is_some(), "server stderr must be piped");

        stdin.write_all(b"protocol-message\n").await?;
        stdin.shutdown().await?;

        let mut response = String::new();
        stdout.read_to_string(&mut response).await?;
        assert_eq!(response, "server:protocol-message\n");
        assert!(child.wait().await?.success());
    }

    let invocations = read_invocations(&log_path)?;
    assert_eq!(invocations.len(), 2);
    assert_eq!(
        invocations[0].argv,
        ["app-server", "--stdio", "--environment-id=environment-123"]
    );
    assert_eq!(
        invocations[1].argv,
        ["exec-server", "--stdio", "--use-agent-identity-auth"]
    );

    let error = client
        .run_non_tui_command(NonTuiCommandRequest::new(NonTuiCommand::AppServer).arg("--stdio"))
        .await
        .expect_err("server commands must not use the output-capturing path");
    assert!(matches!(
        error,
        CodexError::NonTuiServerRequiresSpawn { .. }
    ));

    Ok(())
}

#[tokio::test]
async fn new_0129_surfaces_spawn_expected_subcommands() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let log_path = temp.path().join("invocations.jsonl");
    let fake_codex = write_fake_codex(&log_path)?;

    let client = CodexClient::builder()
        .binary(&fake_codex)
        .mirror_stdout(false)
        .quiet(true)
        .build();

    let mut exec_server = client.start_exec_server(
        ExecServerRequest::new()
            .listen("stdio")
            .executor_id("executor-1")
            .name("background-worker"),
    )?;
    let exec_server_status = exec_server.wait().await?;
    assert!(exec_server_status.success());

    let access_token_login = client.spawn_with_access_token_login_process()?;
    let access_token_login_output = access_token_login.wait_with_output().await?;
    assert!(access_token_login_output.status.success());

    let sandbox_linux = client
        .run_sandbox(
            SandboxCommandRequest::new(SandboxPlatform::Linux, ["echo", "linux"])
                .include_managed_config(true)
                .permissions_profile("linux-profile"),
        )
        .await?;
    assert!(sandbox_linux.status.success());

    let sandbox_macos = client
        .run_sandbox(
            SandboxCommandRequest::new(SandboxPlatform::Macos, ["echo", "macos"])
                .include_managed_config(true)
                .permissions_profile("macos-profile"),
        )
        .await?;
    assert!(sandbox_macos.status.success());

    let sandbox_windows = client
        .run_sandbox(
            SandboxCommandRequest::new(SandboxPlatform::Windows, ["echo", "windows"])
                .include_managed_config(true)
                .permissions_profile("windows-profile"),
        )
        .await?;
    assert!(sandbox_windows.status.success());

    let update = client.update(UpdateCommandRequest::new()).await?;
    assert!(update.status.success());

    let invocations = read_invocations(&log_path)?;
    let argv_sets: Vec<_> = invocations
        .iter()
        .map(|inv| inv.argv.as_slice())
        .collect::<Vec<_>>();

    assert!(
        invocations.iter().any(|inv| {
            inv.argv.first().map(|value| value.as_str()) == Some("exec-server")
                && inv.argv.iter().any(|value| value == "--listen")
                && inv.argv.iter().any(|value| value == "--executor-id")
                && inv.argv.iter().any(|value| value == "--name")
        }),
        "missing 0.129 exec-server invocation: {:?}",
        argv_sets
    );
    assert!(
        invocations
            .iter()
            .any(|inv| inv.argv == ["login", "--with-access-token"]),
        "missing access-token login invocation: {:?}",
        argv_sets
    );
    assert!(
        invocations.iter().any(|inv| {
            inv.argv.first().map(|value| value.as_str()) == Some("sandbox")
                && inv.argv.get(1).map(|value| value.as_str()) == Some("linux")
                && inv
                    .argv
                    .iter()
                    .any(|value| value == "--include-managed-config")
                && inv
                    .argv
                    .iter()
                    .any(|value| value == "--permissions-profile")
        }),
        "missing sandbox linux 0.129 invocation: {:?}",
        argv_sets
    );
    assert!(
        invocations.iter().any(|inv| {
            inv.argv.first().map(|value| value.as_str()) == Some("sandbox")
                && inv.argv.get(1).map(|value| value.as_str()) == Some("macos")
                && inv
                    .argv
                    .iter()
                    .any(|value| value == "--include-managed-config")
                && inv
                    .argv
                    .iter()
                    .any(|value| value == "--permissions-profile")
        }),
        "missing sandbox macos 0.129 invocation: {:?}",
        argv_sets
    );
    assert!(
        invocations.iter().any(|inv| {
            inv.argv.first().map(|value| value.as_str()) == Some("sandbox")
                && inv.argv.get(1).map(|value| value.as_str()) == Some("windows")
                && inv
                    .argv
                    .iter()
                    .any(|value| value == "--include-managed-config")
                && inv
                    .argv
                    .iter()
                    .any(|value| value == "--permissions-profile")
        }),
        "missing sandbox windows 0.129 invocation: {:?}",
        argv_sets
    );
    assert!(
        invocations.iter().any(|inv| inv.argv == ["update"]),
        "missing update invocation: {:?}",
        argv_sets
    );

    Ok(())
}

fn write_fake_codex(log_path: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let script_path = log_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("fake_codex.sh");
    let script = format!(
        r#"#!/usr/bin/env bash
set -euo pipefail

LOG_PATH="{log}"

python3 - "$LOG_PATH" "$@" <<'PY'
import json
import sys

log_path = sys.argv[1]
argv = sys.argv[2:]

with open(log_path, 'a', encoding='utf-8') as handle:
    handle.write(json.dumps({{'argv': argv}}))
    handle.write('\n')
PY

if [[ $# -ge 2 && $1 == "features" && ( $2 == "enable" || $2 == "disable" ) ]]; then
  echo "features-ok"
  exit 0
fi

if [[ $# -ge 4 && $1 == "debug" && $2 == "app-server" && $3 == "send-message-v2" ]]; then
  echo "debug-ok"
  exit 0
fi

if [[ $# -ge 2 && $1 == "debug" && $2 == "models" ]]; then
  echo "debug-models-ok"
  exit 0
fi

if [[ $# -ge 2 && $1 == "debug" && $2 == "prompt-input" ]]; then
  echo "debug-prompt-input-ok"
  exit 0
fi

if [[ $# -ge 2 && $1 == "app-server" && ( $2 == "generate-ts" || $2 == "generate-json-schema" ) ]]; then
  echo "app-server-ok"
  exit 0
fi

if [[ $# -ge 2 && $1 == "app-server" && $2 == "proxy" ]]; then
  echo "app-server-proxy-ok"
  exit 0
fi

if [[ $# -ge 1 && ( $1 == "app-server" || $1 == "exec-server" ) && " $* " == *" --stdio "* ]]; then
  IFS= read -r line
  printf 'server:%s\n' "$line"
  exit 0
fi

if [[ $# -ge 1 && $1 == "app-server" ]]; then
  echo "app-server-root-ok"
  exit 0
fi

if [[ $# -ge 1 && $1 == "exec-server" ]]; then
  echo "exec-server-ok"
  exit 0
fi

if [[ $# -ge 2 && $1 == "login" && $2 == "--with-access-token" ]]; then
  echo "login-with-access-token-ok"
  exit 0
fi

if [[ $# -ge 2 && $1 == "sandbox" ]]; then
  echo "sandbox-ok"
  exit 0
fi

if [[ $# -ge 1 && $1 == "plugin" ]]; then
  echo "plugin-ok"
  exit 0
fi

if [[ $# -ge 1 && $1 == "update" ]]; then
  echo "update-ok"
  exit 0
fi

echo "generic-ok"
exit 0
"#,
        log = log_path.display()
    );

    fs::write(&script_path, script)?;
    let mut permissions = fs::metadata(&script_path)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&script_path, permissions)?;
    Ok(script_path)
}

fn non_tui_passthrough_args(command: NonTuiCommand) -> &'static [&'static str] {
    match command {
        NonTuiCommand::AppServer
        | NonTuiCommand::AppServerDaemon
        | NonTuiCommand::RemoteControl => &["--packet-flag=packet-value"],
        _ => &["--packet-flag", "packet-value"],
    }
}

fn read_invocations(log_path: &Path) -> Result<Vec<Invocation>, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(log_path)?;
    let mut invocations = Vec::new();
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        invocations.push(serde_json::from_str(line)?);
    }
    Ok(invocations)
}
