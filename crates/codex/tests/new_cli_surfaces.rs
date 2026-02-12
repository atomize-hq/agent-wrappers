#![cfg(unix)]

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

use codex::{
    AppServerCodegenRequest, CodexClient, DebugAppServerSendMessageV2Request,
    FeaturesDisableRequest, FeaturesEnableRequest,
};
use serde::Deserialize;

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

if [[ $# -ge 2 && $1 == "app-server" && ( $2 == "generate-ts" || $2 == "generate-json-schema" ) ]]; then
  echo "app-server-ok"
  exit 0
fi

echo "unknown command: $@" >&2
exit 1
"#,
        log = log_path.display()
    );

    fs::write(&script_path, script)?;
    let mut permissions = fs::metadata(&script_path)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&script_path, permissions)?;
    Ok(script_path)
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
