use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use jsonschema::{Draft, JSONSchema};
use serde_json::{json, Value};

const VERSION: &str = "0.61.0";
const TS: &str = "1970-01-01T00:00:00Z";

const REQUIRED_TARGET: &str = "x86_64-unknown-linux-musl";
const TARGET_LINUX: &str = "x86_64-unknown-linux-musl";
const TARGET_MACOS: &str = "aarch64-apple-darwin";
const TARGET_WINDOWS: &str = "x86_64-pc-windows-msvc";
const TARGETS: [&str; 3] = [TARGET_LINUX, TARGET_MACOS, TARGET_WINDOWS];

type CommandPath = Vec<String>;
type MissingCommandPaths = Vec<CommandPath>;
type MissingFlagPaths = Vec<(CommandPath, String)>;
type MissingArgPaths = Vec<(CommandPath, String)>;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("CARGO_MANIFEST_DIR has crates/<crate> parent structure")
        .to_path_buf()
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

fn write_text(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent dirs");
    }
    fs::write(path, contents).expect("write file");
}

fn write_json(path: &Path, value: &Value) {
    let text = serde_json::to_string_pretty(value).expect("serialize json");
    write_text(path, &format!("{text}\n"));
}

fn read_json(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap_or_else(|e| panic!("read {path:?}: {e}")))
        .unwrap_or_else(|e| panic!("parse json {path:?}: {e}"))
}

fn copy_from_repo(codex_dir: &Path, filename: &str) {
    let src = repo_root()
        .join("cli_manifests")
        .join("codex")
        .join(filename);
    let dst = codex_dir.join(filename);
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent).expect("mkdir dst parent");
    }
    fs::copy(&src, &dst).unwrap_or_else(|e| panic!("copy {src:?} -> {dst:?}: {e}"));
}

fn compile_schema_with_file_id(path: &Path) -> JSONSchema {
    let abs = path
        .canonicalize()
        .unwrap_or_else(|e| panic!("canonicalize {path:?}: {e}"));
    let mut schema_value: Value =
        serde_json::from_slice(&fs::read(&abs).unwrap_or_else(|e| panic!("read {abs:?}: {e}")))
            .unwrap_or_else(|e| panic!("parse schema {abs:?}: {e}"));

    if let Some(obj) = schema_value.as_object_mut() {
        obj.insert(
            "$id".to_string(),
            Value::String(format!("file://{}", abs.display())),
        );
    }

    JSONSchema::options()
        .with_draft(Draft::Draft202012)
        .compile(&schema_value)
        .unwrap_or_else(|e| panic!("compile schema {abs:?}: {e}"))
}

fn assert_schema_valid(schema: &JSONSchema, instance: &Value) {
    if let Err(errors) = schema.validate(instance) {
        let messages = errors.map(|e| e.to_string()).collect::<Vec<_>>();
        panic!("schema validation failed:\n{}", messages.join("\n"));
    }
}

fn minimal_binary_for_target(target: &str) -> Value {
    let (os, arch) = match target {
        TARGET_LINUX => ("linux", "x86_64"),
        TARGET_MACOS => ("macos", "aarch64"),
        TARGET_WINDOWS => ("windows", "x86_64"),
        _ => ("unknown", "unknown"),
    };
    json!({
        "sha256": "00",
        "size_bytes": 0,
        "platform": { "os": os, "arch": arch },
        "target_triple": target,
        "version_output": format!("codex-cli {VERSION}"),
        "semantic_version": VERSION,
        "channel": "stable",
    })
}

fn write_wrapper_coverage_empty(codex_dir: &Path) {
    let wrapper_coverage = json!({
        "schema_version": 1,
        "generated_at": TS,
        "wrapper_version": "0.0.0-test",
        "coverage": []
    });
    write_json(&codex_dir.join("wrapper_coverage.json"), &wrapper_coverage);
}

fn write_union_snapshot(codex_dir: &Path, complete: bool) {
    let union_inputs = if complete {
        TARGETS.to_vec()
    } else {
        vec![REQUIRED_TARGET]
    };

    let inputs = union_inputs
        .iter()
        .map(|target| {
            json!({
                "target_triple": target,
                "collected_at": TS,
                "binary": minimal_binary_for_target(target),
            })
        })
        .collect::<Vec<_>>();

    let commands = if complete {
        json!([
            {
                "path": ["root"],
                "available_on": TARGETS,
                "flags": [
                    { "key": "--all", "long": "--all", "takes_value": false, "available_on": TARGETS },
                    { "key": "--linux-only", "long": "--linux-only", "takes_value": false, "available_on": [TARGET_LINUX] }
                ],
                "args": [
                    { "name": "INPUT", "available_on": TARGETS },
                    { "name": "WIN", "available_on": [TARGET_WINDOWS] }
                ]
            },
            { "path": ["linux-only"], "available_on": [TARGET_LINUX] },
            { "path": ["macos-only"], "available_on": [TARGET_MACOS] },
            {
                "path": ["two"],
                "available_on": [TARGET_LINUX, TARGET_MACOS],
                "args": [
                    { "name": "LM", "available_on": [TARGET_LINUX, TARGET_MACOS] }
                ]
            }
        ])
    } else {
        json!([
            {
                "path": ["root"],
                "available_on": [TARGET_LINUX],
                "flags": [
                    { "key": "--linux-only", "long": "--linux-only", "takes_value": false, "available_on": [TARGET_LINUX] }
                ],
                "args": [
                    { "name": "INPUT", "available_on": [TARGET_LINUX] }
                ]
            },
            { "path": ["linux-only"], "available_on": [TARGET_LINUX] }
        ])
    };

    let union = if complete {
        json!({
            "snapshot_schema_version": 2,
            "tool": "codex-cli",
            "mode": "union",
            "collected_at": TS,
            "expected_targets": TARGETS,
            "complete": true,
            "inputs": inputs,
            "commands": commands,
        })
    } else {
        json!({
            "snapshot_schema_version": 2,
            "tool": "codex-cli",
            "mode": "union",
            "collected_at": TS,
            "expected_targets": TARGETS,
            "complete": false,
            "missing_targets": [TARGET_MACOS, TARGET_WINDOWS],
            "inputs": inputs,
            "commands": commands,
        })
    };

    let union_path = codex_dir.join("snapshots").join(VERSION).join("union.json");
    write_json(&union_path, &union);
}

fn materialize_codex_root_for_reports(codex_dir: &Path, union_complete: bool) {
    fs::create_dir_all(codex_dir).expect("mkdir codex dir");
    copy_from_repo(codex_dir, "SCHEMA.json");
    copy_from_repo(codex_dir, "RULES.json");
    copy_from_repo(codex_dir, "VERSION_METADATA_SCHEMA.json");

    write_union_snapshot(codex_dir, union_complete);
    write_wrapper_coverage_empty(codex_dir);
}

fn assert_xtask_subcommand_exists(subcommand: &str, fixture_root: &Path) -> String {
    let xtask_bin = PathBuf::from(env!("CARGO_BIN_EXE_xtask"));
    let output = Command::new(&xtask_bin)
        .arg(subcommand)
        .arg("--help")
        .current_dir(fixture_root)
        .output()
        .unwrap_or_else(|e| panic!("spawn xtask {subcommand} --help: {e}"));

    assert!(
        output.status.success(),
        "xtask is missing `{subcommand}` (C3-code must add the subcommand).\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn root_flag_from_help(help_text: &str) -> &'static str {
    if help_text.contains("--root") {
        "--root"
    } else if help_text.contains("--codex-dir") {
        "--codex-dir"
    } else {
        panic!("help did not contain --root or --codex-dir:\n{help_text}");
    }
}

fn run_xtask_codex_report(codex_dir: &Path) -> std::process::Output {
    let fixture_root = codex_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("codex_dir is <fixture_root>/cli_manifests/codex");
    let help_text = assert_xtask_subcommand_exists("codex-report", fixture_root);
    let root_flag = root_flag_from_help(&help_text);

    let xtask_bin = PathBuf::from(env!("CARGO_BIN_EXE_xtask"));
    Command::new(xtask_bin)
        .arg("codex-report")
        .arg(root_flag)
        .arg(codex_dir)
        .arg("--version")
        .arg(VERSION)
        .env("SOURCE_DATE_EPOCH", "0")
        .current_dir(fixture_root)
        .output()
        .expect("spawn xtask codex-report")
}

fn run_xtask_codex_version_metadata(codex_dir: &Path, status: &str) -> std::process::Output {
    let fixture_root = codex_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("codex_dir is <fixture_root>/cli_manifests/codex");
    let help_text = assert_xtask_subcommand_exists("codex-version-metadata", fixture_root);
    let root_flag = root_flag_from_help(&help_text);

    let xtask_bin = PathBuf::from(env!("CARGO_BIN_EXE_xtask"));
    Command::new(xtask_bin)
        .arg("codex-version-metadata")
        .arg(root_flag)
        .arg(codex_dir)
        .arg("--version")
        .arg(VERSION)
        .arg("--status")
        .arg(status)
        .env("SOURCE_DATE_EPOCH", "0")
        .current_dir(fixture_root)
        .output()
        .expect("spawn xtask codex-version-metadata")
}

fn run_xtask_codex_retain(codex_dir: &Path, apply: bool) -> std::process::Output {
    let fixture_root = codex_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("codex_dir is <fixture_root>/cli_manifests/codex");
    let help_text = assert_xtask_subcommand_exists("codex-retain", fixture_root);
    let root_flag = root_flag_from_help(&help_text);

    let xtask_bin = PathBuf::from(env!("CARGO_BIN_EXE_xtask"));
    let mut cmd = Command::new(xtask_bin);
    cmd.arg("codex-retain").arg(root_flag).arg(codex_dir);
    if apply {
        cmd.arg("--apply");
    }
    cmd.current_dir(fixture_root)
        .output()
        .expect("spawn xtask codex-retain")
}

fn extract_report_paths(
    report: &Value,
) -> (MissingCommandPaths, MissingFlagPaths, MissingArgPaths) {
    let deltas = report
        .get("deltas")
        .and_then(|v| v.as_object())
        .expect("report.deltas object");

    let missing_commands = deltas
        .get("missing_commands")
        .and_then(|v| v.as_array())
        .expect("missing_commands array")
        .iter()
        .map(|d| {
            d.get("path")
                .and_then(|p| p.as_array())
                .expect("missing_command.path array")
                .iter()
                .map(|t| t.as_str().expect("path token string").to_string())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    let missing_flags = deltas
        .get("missing_flags")
        .and_then(|v| v.as_array())
        .expect("missing_flags array")
        .iter()
        .map(|d| {
            let path = d
                .get("path")
                .and_then(|p| p.as_array())
                .expect("missing_flag.path array")
                .iter()
                .map(|t| t.as_str().expect("path token string").to_string())
                .collect::<Vec<_>>();
            let key = d
                .get("key")
                .and_then(|k| k.as_str())
                .expect("missing_flag.key string")
                .to_string();
            (path, key)
        })
        .collect::<Vec<_>>();

    let missing_args = deltas
        .get("missing_args")
        .and_then(|v| v.as_array())
        .expect("missing_args array")
        .iter()
        .map(|d| {
            let path = d
                .get("path")
                .and_then(|p| p.as_array())
                .expect("missing_arg.path array")
                .iter()
                .map(|t| t.as_str().expect("path token string").to_string())
                .collect::<Vec<_>>();
            let name = d
                .get("name")
                .and_then(|k| k.as_str())
                .expect("missing_arg.name string")
                .to_string();
            (path, name)
        })
        .collect::<Vec<_>>();

    (missing_commands, missing_flags, missing_args)
}

fn assert_report_common(report: &Value, expected_mode: &str, expected_target: Option<&str>) {
    assert_eq!(
        report.get("generated_at").and_then(|v| v.as_str()),
        Some(TS),
        "expected deterministic generated_at when SOURCE_DATE_EPOCH=0"
    );

    let platform_filter = report
        .get("platform_filter")
        .and_then(|v| v.as_object())
        .expect("platform_filter object");
    assert_eq!(
        platform_filter.get("mode").and_then(|v| v.as_str()),
        Some(expected_mode)
    );
    if let Some(target) = expected_target {
        assert_eq!(
            platform_filter
                .get("target_triple")
                .and_then(|v| v.as_str()),
            Some(target)
        );
    }
}

fn write_versions_metadata(codex_dir: &Path, versions: &HashMap<&str, &str>) {
    for (v, status) in versions {
        let metadata = json!({
            "schema_version": 1,
            "semantic_version": v,
            "status": status,
            "updated_at": TS
        });
        write_json(
            &codex_dir.join("versions").join(format!("{v}.json")),
            &metadata,
        );
    }
}

fn touch_dir_with_marker(path: &Path, marker: &str) {
    fs::create_dir_all(path).expect("create dir");
    write_text(&path.join("marker.txt"), marker);
}

fn parse_retain_output_lists(output: &str) -> (Vec<String>, Vec<String>) {
    let mut state: Option<&str> = None;
    let mut keep = Vec::<String>::new();
    let mut delete = Vec::<String>::new();

    for raw_line in output.lines() {
        let line = raw_line.trim();
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("keep") || lower == "keep:" {
            state = Some("keep");
        }
        if lower.starts_with("delete") || lower == "delete:" {
            state = Some("delete");
        }

        for token in line.split(|c: char| !c.is_ascii_alphanumeric() && c != '.' && c != '-') {
            if !token.contains('.') {
                continue;
            }
            if token.chars().next().is_some_and(|c| c.is_ascii_digit())
                && token
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
                && token.split('.').count() >= 3
            {
                match state {
                    Some("keep") => keep.push(token.to_string()),
                    Some("delete") => delete.push(token.to_string()),
                    _ => {}
                }
            }
        }
    }

    (keep, delete)
}

#[path = "c3_spec_reports_metadata_retain/report_filter_semantics.rs"]
mod report_filter_semantics;
#[path = "c3_spec_reports_metadata_retain/report_incomplete_union.rs"]
mod report_incomplete_union;
#[path = "c3_spec_reports_metadata_retain/retain_behavior.rs"]
mod retain_behavior;
#[path = "c3_spec_reports_metadata_retain/version_metadata_requirements.rs"]
mod version_metadata_requirements;
