use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use jsonschema::{Draft, JSONSchema};
use semver::Version as SemverVersion;
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

#[test]
fn c3_report_filter_semantics_complete_union_empty_wrapper() {
    let temp = make_temp_dir("ccm-c3-report-complete");
    let codex_dir = temp.join("cli_manifests").join("codex");
    materialize_codex_root_for_reports(&codex_dir, true);

    let output = run_xtask_codex_report(&codex_dir);
    assert!(
        output.status.success(),
        "expected codex-report success on complete union:\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let reports_dir = codex_dir.join("reports").join(VERSION);
    let schema = compile_schema_with_file_id(&codex_dir.join("SCHEMA.json"));

    let any_path = reports_dir.join("coverage.any.json");
    let any = read_json(&any_path);
    assert_schema_valid(&schema, &any);
    assert_report_common(&any, "any", None);
    let (any_cmds, any_flags, any_args) = extract_report_paths(&any);
    assert_eq!(any_cmds.len(), 4, "any should include all commands");
    assert_eq!(any_flags.len(), 2, "any should include all flags");
    assert_eq!(any_args.len(), 3, "any should include all args");

    let all_path = reports_dir.join("coverage.all.json");
    let all = read_json(&all_path);
    assert_schema_valid(&schema, &all);
    assert_report_common(&all, "all", None);
    let (all_cmds, all_flags, all_args) = extract_report_paths(&all);
    assert_eq!(
        all_cmds,
        vec![vec!["root".to_string()]],
        "all should include only all-target commands"
    );
    assert_eq!(
        all_flags,
        vec![(vec!["root".to_string()], "--all".to_string())],
        "all should include only all-target flags"
    );
    assert_eq!(
        all_args,
        vec![(vec!["root".to_string()], "INPUT".to_string())],
        "all should include only all-target args"
    );

    for target in TARGETS {
        let per_target_path = reports_dir.join(format!("coverage.{target}.json"));
        let report = read_json(&per_target_path);
        assert_schema_valid(&schema, &report);
        assert_report_common(&report, "exact_target", Some(target));
    }

    let linux = read_json(&reports_dir.join(format!("coverage.{TARGET_LINUX}.json")));
    let (linux_cmds, _linux_flags, linux_args) = extract_report_paths(&linux);
    assert_eq!(linux_cmds.len(), 3, "linux should see linux+all commands");
    assert_eq!(linux_args.len(), 2, "linux should not see windows-only arg");

    let windows = read_json(&reports_dir.join(format!("coverage.{TARGET_WINDOWS}.json")));
    let (windows_cmds, _windows_flags, windows_args) = extract_report_paths(&windows);
    assert_eq!(
        windows_cmds.len(),
        1,
        "windows should only see root command"
    );
    assert_eq!(windows_args.len(), 2, "windows should see windows-only arg");
}

#[test]
fn c3_report_skips_coverage_all_when_union_incomplete() {
    let temp = make_temp_dir("ccm-c3-report-incomplete");
    let codex_dir = temp.join("cli_manifests").join("codex");
    materialize_codex_root_for_reports(&codex_dir, false);

    let _output = run_xtask_codex_report(&codex_dir);

    let reports_dir = codex_dir.join("reports").join(VERSION);
    assert!(
        reports_dir.join("coverage.any.json").exists(),
        "codex-report must write coverage.any.json even when union is incomplete"
    );
    assert!(
        reports_dir
            .join(format!("coverage.{TARGET_LINUX}.json"))
            .exists(),
        "codex-report must write per-target report for included input targets"
    );
    assert!(
        !reports_dir
            .join(format!("coverage.{TARGET_MACOS}.json"))
            .exists(),
        "per-target reports must only be generated for union.inputs[].target_triple"
    );
    assert!(
        !reports_dir
            .join(format!("coverage.{TARGET_WINDOWS}.json"))
            .exists(),
        "per-target reports must only be generated for union.inputs[].target_triple"
    );
    assert!(
        !reports_dir.join("coverage.all.json").exists(),
        "coverage.all.json must not be generated when union complete=false"
    );
}

#[test]
fn c3_version_metadata_reported_requires_union_and_any_report() {
    let temp = make_temp_dir("ccm-c3-version-metadata");
    let codex_dir = temp.join("cli_manifests").join("codex");

    fs::create_dir_all(&codex_dir).expect("mkdir codex dir");
    copy_from_repo(&codex_dir, "SCHEMA.json");
    copy_from_repo(&codex_dir, "RULES.json");
    copy_from_repo(&codex_dir, "VERSION_METADATA_SCHEMA.json");
    write_union_snapshot(&codex_dir, false);
    write_wrapper_coverage_empty(&codex_dir);

    let missing_reports = run_xtask_codex_version_metadata(&codex_dir, "reported");
    assert!(
        !missing_reports.status.success(),
        "expected failure when coverage.any.json is missing:\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
        missing_reports.status,
        String::from_utf8_lossy(&missing_reports.stdout),
        String::from_utf8_lossy(&missing_reports.stderr)
    );
    let err = format!(
        "{}\n{}",
        String::from_utf8_lossy(&missing_reports.stdout),
        String::from_utf8_lossy(&missing_reports.stderr)
    );
    assert!(
        err.contains("coverage.any.json"),
        "expected missing coverage.any.json error, got:\n{err}"
    );

    let report_out = run_xtask_codex_report(&codex_dir);
    assert!(
        report_out.status.success() || !report_out.status.success(),
        "codex-report must run to materialize required report files"
    );

    let output = run_xtask_codex_version_metadata(&codex_dir, "reported");
    assert!(
        output.status.success(),
        "expected success after adding reports:\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let version_path = codex_dir.join("versions").join(format!("{VERSION}.json"));
    assert!(
        version_path.exists(),
        "expected versions/<version>.json written"
    );

    let schema = compile_schema_with_file_id(&codex_dir.join("VERSION_METADATA_SCHEMA.json"));
    let metadata = read_json(&version_path);
    assert_schema_valid(&schema, &metadata);
    assert_eq!(
        metadata.get("semantic_version").and_then(|v| v.as_str()),
        Some(VERSION)
    );
    assert_eq!(
        metadata.get("status").and_then(|v| v.as_str()),
        Some("reported")
    );
    assert_eq!(
        metadata.get("updated_at").and_then(|v| v.as_str()),
        Some(TS),
        "expected deterministic updated_at when SOURCE_DATE_EPOCH=0"
    );
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

#[test]
fn c3_retain_deletes_only_snapshots_and_reports_outside_keep_set() {
    let temp = make_temp_dir("ccm-c3-retain");
    let codex_dir = temp.join("cli_manifests").join("codex");
    fs::create_dir_all(&codex_dir).expect("mkdir codex dir");
    copy_from_repo(&codex_dir, "RULES.json");

    let all_versions = ["0.1.0", "0.2.0", "0.3.0", "0.4.0", "0.5.0", "0.6.0"];

    write_text(&codex_dir.join("min_supported.txt"), "0.2.0\n");
    write_text(&codex_dir.join("latest_validated.txt"), "0.5.0\n");
    for target in TARGETS {
        let supported = codex_dir
            .join("pointers")
            .join("latest_supported")
            .join(format!("{target}.txt"));
        let validated = codex_dir
            .join("pointers")
            .join("latest_validated")
            .join(format!("{target}.txt"));
        if target == TARGET_LINUX {
            write_text(&supported, "0.3.0\n");
            write_text(&validated, "0.5.0\n");
        } else {
            write_text(&supported, "none\n");
            write_text(&validated, "none\n");
        }
    }

    let versions = HashMap::from([
        ("0.1.0", "snapshotted"),
        ("0.2.0", "validated"),
        ("0.3.0", "reported"),
        ("0.4.0", "supported"),
        ("0.5.0", "validated"),
        ("0.6.0", "validated"),
    ]);
    write_versions_metadata(&codex_dir, &versions);

    for v in all_versions {
        touch_dir_with_marker(&codex_dir.join("snapshots").join(v), v);
        touch_dir_with_marker(&codex_dir.join("reports").join(v), v);
        touch_dir_with_marker(&codex_dir.join("raw_help").join(v), v);
    }

    let expected_keep =
        BTreeSet::from_iter(["0.2.0", "0.3.0", "0.4.0", "0.5.0", "0.6.0"].map(|v| v.to_string()));
    let expected_delete = BTreeSet::from_iter(["0.1.0"].map(|v| v.to_string()));

    let dry = run_xtask_codex_retain(&codex_dir, false);
    let dry_text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&dry.stdout),
        String::from_utf8_lossy(&dry.stderr)
    );
    for v in &expected_keep {
        assert!(
            dry_text.contains(v),
            "dry-run output must include keep version {v}:\n{dry_text}"
        );
    }
    for v in &expected_delete {
        assert!(
            dry_text.contains(v),
            "dry-run output must include delete version {v}:\n{dry_text}"
        );
    }

    for v in all_versions {
        assert!(
            codex_dir.join("snapshots").join(v).exists(),
            "dry-run must not delete snapshots/{v}"
        );
        assert!(
            codex_dir.join("reports").join(v).exists(),
            "dry-run must not delete reports/{v}"
        );
        assert!(
            codex_dir.join("raw_help").join(v).exists(),
            "dry-run must not delete raw_help/{v}"
        );
        assert!(
            codex_dir
                .join("versions")
                .join(format!("{v}.json"))
                .exists(),
            "retain must not delete versions/{v}.json"
        );
    }

    let apply = run_xtask_codex_retain(&codex_dir, true);
    assert!(
        apply.status.success(),
        "expected codex-retain --apply success:\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
        apply.status,
        String::from_utf8_lossy(&apply.stdout),
        String::from_utf8_lossy(&apply.stderr)
    );
    let apply_text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&apply.stdout),
        String::from_utf8_lossy(&apply.stderr)
    );
    for v in &expected_keep {
        assert!(
            apply_text.contains(v),
            "apply output must include keep version {v}:\n{apply_text}"
        );
    }
    for v in &expected_delete {
        assert!(
            apply_text.contains(v),
            "apply output must include delete version {v}:\n{apply_text}"
        );
    }

    let (keep_list, delete_list) = parse_retain_output_lists(&apply_text);
    if !keep_list.is_empty() && !delete_list.is_empty() {
        let keep_set: BTreeSet<_> = keep_list.iter().cloned().collect();
        let delete_set: BTreeSet<_> = delete_list.iter().cloned().collect();
        assert_eq!(keep_set, expected_keep);
        assert_eq!(delete_set, expected_delete);

        let mut keep_sorted = keep_list.clone();
        keep_sorted.sort_by(|a, b| {
            let a = SemverVersion::parse(a).unwrap_or_else(|e| panic!("parse keep {a}: {e}"));
            let b = SemverVersion::parse(b).unwrap_or_else(|e| panic!("parse keep {b}: {e}"));
            a.cmp(&b)
        });
        assert_eq!(keep_list, keep_sorted, "keep list must be sorted ascending");

        let mut delete_sorted = delete_list.clone();
        delete_sorted.sort_by(|a, b| {
            let a = SemverVersion::parse(a).unwrap_or_else(|e| panic!("parse delete {a}: {e}"));
            let b = SemverVersion::parse(b).unwrap_or_else(|e| panic!("parse delete {b}: {e}"));
            a.cmp(&b)
        });
        assert_eq!(
            delete_list, delete_sorted,
            "delete list must be sorted ascending"
        );
    }

    for v in expected_keep.iter() {
        assert!(
            codex_dir.join("snapshots").join(v).exists(),
            "expected keep snapshots/{v}"
        );
        assert!(
            codex_dir.join("reports").join(v).exists(),
            "expected keep reports/{v}"
        );
    }
    for v in expected_delete.iter() {
        assert!(
            !codex_dir.join("snapshots").join(v).exists(),
            "expected delete snapshots/{v}"
        );
        assert!(
            !codex_dir.join("reports").join(v).exists(),
            "expected delete reports/{v}"
        );
    }

    for v in all_versions {
        assert!(
            codex_dir.join("raw_help").join(v).exists(),
            "retain must not delete raw_help/{v}"
        );
    }
}
