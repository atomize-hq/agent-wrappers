use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("CARGO_MANIFEST_DIR has 2 parents (crates/xtask)")
        .to_path_buf()
}

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("c1")
}

fn make_temp_dir(prefix: &str) -> PathBuf {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time after unix epoch");
    let unique = format!("{}-{}-{}", prefix, std::process::id(), now.as_nanos());
    let dir = std::env::temp_dir().join(unique);
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn write_rules_json(codex_root: &Path) {
    fs::create_dir_all(codex_root).expect("create codex root dir");
    let src = repo_root()
        .join("cli_manifests")
        .join("codex")
        .join("RULES.json");
    let dst = codex_root.join("RULES.json");
    fs::copy(src, dst).expect("copy RULES.json");
}

fn expected_targets_from_rules(codex_root: &Path) -> Vec<String> {
    let rules_text = fs::read_to_string(codex_root.join("RULES.json")).expect("read RULES.json");
    let rules: Value = serde_json::from_str(&rules_text).expect("parse RULES.json");
    rules
        .get("union")
        .and_then(|u| u.get("expected_targets"))
        .and_then(Value::as_array)
        .expect("RULES.json.union.expected_targets is array")
        .iter()
        .map(|v| v.as_str().expect("target triple is string").to_string())
        .collect()
}

fn write_snapshot_fixture(codex_root: &Path, version: &str, target_triple: &str) {
    let src = fixtures_dir().join(format!("{target_triple}.json"));
    assert!(src.is_file(), "missing fixture file: {}", src.display());

    let dst_dir = codex_root.join("snapshots").join(version);
    fs::create_dir_all(&dst_dir).expect("create snapshots/<version> dir");
    let dst = dst_dir.join(format!("{target_triple}.json"));
    fs::copy(src, dst).expect("copy snapshot fixture");
}

fn write_snapshot_json(codex_root: &Path, version: &str, target_triple: &str, snapshot: &Value) {
    let dst_dir = codex_root.join("snapshots").join(version);
    fs::create_dir_all(&dst_dir).expect("create snapshots/<version> dir");
    let dst = dst_dir.join(format!("{target_triple}.json"));
    fs::write(&dst, format!("{}\n", serde_json::to_string_pretty(snapshot).unwrap()))
        .expect("write snapshot json");
}

fn run_xtask_union(codex_root: &Path, version: &str) -> std::process::Output {
    let xtask_bin = PathBuf::from(env!("CARGO_BIN_EXE_xtask"));
    let output = Command::new(xtask_bin)
        .arg("codex-union")
        .arg("--root")
        .arg(codex_root)
        .arg("--version")
        .arg(version)
        .env("SOURCE_DATE_EPOCH", "0")
        .output()
        .expect("spawn xtask codex-union");

    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.contains("unrecognized subcommand 'codex-union'") {
        panic!("xtask is missing `codex-union` (C1-code must add the subcommand)");
    }

    output
}

fn read_union_json(codex_root: &Path, version: &str) -> Value {
    let union_path = codex_root
        .join("snapshots")
        .join(version)
        .join("union.json");
    let text = fs::read_to_string(&union_path).expect("read union.json");
    serde_json::from_str(&text).expect("parse union.json")
}

fn get_union_command<'a>(union: &'a Value, command_path: &[&str]) -> &'a Value {
    let commands = union
        .get("commands")
        .and_then(Value::as_array)
        .expect("union.commands is array");
    commands
        .iter()
        .find(|c| {
            c.get("path").and_then(Value::as_array).is_some_and(|p| {
                p.iter()
                    .filter_map(Value::as_str)
                    .eq(command_path.iter().copied())
            })
        })
        .unwrap_or_else(|| panic!("missing union command path {:?}", command_path))
}

#[test]
fn c1_union_fails_when_required_target_missing() {
    let temp = make_temp_dir("ccm-c1-union-required-missing");
    let codex_root = temp.join("cli_manifests").join("codex");
    write_rules_json(&codex_root);

    let version = "0.77.0";
    let output = run_xtask_union(&codex_root, version);
    assert!(
        !output.status.success(),
        "expected failure when required target snapshot missing:\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("x86_64-unknown-linux-musl"),
        "stderr should mention required target triple; got:\n{stderr}"
    );
}

#[test]
fn c1_union_emits_complete_false_and_missing_targets_for_non_required_missing() {
    let temp = make_temp_dir("ccm-c1-union-partial");
    let codex_root = temp.join("cli_manifests").join("codex");
    write_rules_json(&codex_root);

    let version = "0.77.0";
    write_snapshot_fixture(&codex_root, version, "x86_64-unknown-linux-musl");

    let output = run_xtask_union(&codex_root, version);
    if !output.status.success() {
        panic!(
            "xtask codex-union failed:\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let union = read_union_json(&codex_root, version);
    let expected_targets = expected_targets_from_rules(&codex_root);

    assert_eq!(
        union
            .get("expected_targets")
            .and_then(Value::as_array)
            .map(|a| a
                .iter()
                .filter_map(Value::as_str)
                .map(|s| s.to_string())
                .collect::<Vec<_>>()),
        Some(expected_targets.clone()),
        "union.expected_targets matches RULES.json.union.expected_targets order"
    );
    assert_eq!(
        union.get("complete").and_then(Value::as_bool),
        Some(false),
        "union.complete must be false when some expected targets are missing"
    );

    let missing = union
        .get("missing_targets")
        .and_then(Value::as_array)
        .expect("union.missing_targets exists when complete=false")
        .iter()
        .filter_map(Value::as_str)
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    assert_eq!(
        missing,
        vec![
            "aarch64-apple-darwin".to_string(),
            "x86_64-pc-windows-msvc".to_string()
        ],
        "missing_targets lists missing non-required targets in expected_targets order"
    );

    let inputs = union
        .get("inputs")
        .and_then(Value::as_array)
        .expect("union.inputs is array");
    assert_eq!(
        inputs
            .iter()
            .filter_map(|i| i.get("target_triple").and_then(Value::as_str))
            .collect::<Vec<_>>(),
        vec!["x86_64-unknown-linux-musl"],
        "inputs include only the present targets (in expected_targets order)"
    );
}

#[test]
fn c1_union_records_conflicts_and_is_deterministic_with_source_date_epoch() {
    let temp = make_temp_dir("ccm-c1-union-conflicts");
    let codex_root = temp.join("cli_manifests").join("codex");
    write_rules_json(&codex_root);

    let version = "0.77.0";
    for target in [
        "x86_64-unknown-linux-musl",
        "aarch64-apple-darwin",
        "x86_64-pc-windows-msvc",
    ] {
        write_snapshot_fixture(&codex_root, version, target);
    }

    let output_a = run_xtask_union(&codex_root, version);
    if !output_a.status.success() {
        panic!(
            "xtask codex-union failed:\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
            output_a.status,
            String::from_utf8_lossy(&output_a.stdout),
            String::from_utf8_lossy(&output_a.stderr)
        );
    }
    let union_path = codex_root
        .join("snapshots")
        .join(version)
        .join("union.json");
    let bytes_a = fs::read(&union_path).expect("read union.json bytes after run A");

    let output_b = run_xtask_union(&codex_root, version);
    if !output_b.status.success() {
        panic!(
            "xtask codex-union failed:\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
            output_b.status,
            String::from_utf8_lossy(&output_b.stdout),
            String::from_utf8_lossy(&output_b.stderr)
        );
    }
    let bytes_b = fs::read(&union_path).expect("read union.json bytes after run B");
    assert_eq!(
        bytes_a, bytes_b,
        "union.json must be deterministic with SOURCE_DATE_EPOCH set"
    );

    let union = read_union_json(&codex_root, version);
    assert_eq!(
        union.get("complete").and_then(Value::as_bool),
        Some(true),
        "union.complete must be true when all expected targets are present"
    );
    assert!(
        union.get("missing_targets").is_none(),
        "union.missing_targets must be absent when complete=true"
    );

    let alpha = get_union_command(&union, &["alpha"]);
    assert_eq!(
        alpha
            .get("available_on")
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(Value::as_str).collect::<Vec<_>>()),
        Some(vec![
            "x86_64-unknown-linux-musl",
            "aarch64-apple-darwin",
            "x86_64-pc-windows-msvc"
        ]),
        "command available_on follows RULES.json.union.expected_targets order"
    );

    let conflicts = alpha
        .get("conflicts")
        .and_then(Value::as_array)
        .expect("alpha.conflicts exists");

    let flag_takes_value = conflicts.iter().find(|c| {
        c.get("unit").and_then(Value::as_str) == Some("flag")
            && c.get("field").and_then(Value::as_str) == Some("takes_value")
            && c.get("key").and_then(Value::as_str) == Some("--config")
    });
    assert!(
        flag_takes_value.is_some(),
        "expected a flag takes_value conflict for --config; conflicts={:?}",
        conflicts
    );

    let arg_required = conflicts.iter().find(|c| {
        c.get("unit").and_then(Value::as_str) == Some("arg")
            && c.get("field").and_then(Value::as_str) == Some("required")
            && c.get("name").and_then(Value::as_str) == Some("INPUT")
    });
    assert!(
        arg_required.is_some(),
        "expected an arg required conflict for INPUT; conflicts={:?}",
        conflicts
    );
}

#[test]
fn c1_union_dedupes_per_command_flags_against_root() {
    let temp = make_temp_dir("ccm-c1-union-dedupe-root-flags");
    let codex_root = temp.join("cli_manifests").join("codex");
    write_rules_json(&codex_root);

    let version = "0.77.0";
    let target = "x86_64-unknown-linux-musl";

    let snapshot = json!({
      "snapshot_schema_version": 1,
      "tool": "codex-cli",
      "collected_at": "1970-01-01T00:00:00Z",
      "binary": {
        "sha256": "00",
        "size_bytes": 1,
        "platform": { "os": "linux", "arch": "x86_64" },
        "target_triple": target,
        "version_output": format!("codex {version}"),
        "semantic_version": version,
        "channel": "stable"
      },
      "commands": [
        {
          "path": [],
          "usage": "codex [OPTIONS] <COMMAND>",
          "flags": [
            { "long": "--help", "takes_value": false },
            { "long": "--config", "takes_value": true, "value_name": "key=value" }
          ]
        },
        {
          "path": ["alpha"],
          "usage": "codex alpha [OPTIONS]",
          "flags": [
            { "long": "--help", "takes_value": false },
            { "long": "--config", "takes_value": true, "value_name": "key=value" },
            { "long": "--alpha-only", "takes_value": false }
          ]
        }
      ]
    });

    write_snapshot_json(&codex_root, version, target, &snapshot);

    let output = run_xtask_union(&codex_root, version);
    if !output.status.success() {
        panic!(
            "xtask codex-union failed:\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let union = read_union_json(&codex_root, version);
    let alpha = get_union_command(&union, &["alpha"]);
    let flags = alpha
        .get("flags")
        .and_then(Value::as_array)
        .expect("alpha.flags is array");
    let keys = flags
        .iter()
        .filter_map(|f| f.get("key").and_then(Value::as_str))
        .collect::<Vec<_>>();

    assert!(
        !keys.contains(&"--help"),
        "expected per-command --help to be deduped against root; flags={keys:?}"
    );
    assert!(
        !keys.contains(&"--config"),
        "expected per-command --config to be deduped against root; flags={keys:?}"
    );
    assert!(
        keys.contains(&"--alpha-only"),
        "expected alpha-only flag to remain; flags={keys:?}"
    );
}
