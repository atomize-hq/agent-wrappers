use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};

const VERSION: &str = "0.61.0";
const TS: &str = "1970-01-01T00:00:00Z";

const TARGET_LINUX: &str = "x86_64-unknown-linux-musl";
const TARGET_MACOS: &str = "aarch64-apple-darwin";
const TARGET_WINDOWS: &str = "x86_64-pc-windows-msvc";
const TARGETS: [&str; 3] = [TARGET_LINUX, TARGET_MACOS, TARGET_WINDOWS];

const NOTE_COMPLETION: &str = "Shell completion generation is out of scope for the wrapper.";

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

fn write_union_snapshot(codex_dir: &Path) {
    let inputs = TARGETS
        .iter()
        .map(|target| {
            json!({
                "target_triple": target,
                "collected_at": TS,
                "binary": minimal_binary_for_target(target),
            })
        })
        .collect::<Vec<_>>();

    let commands = json!([
        { "path": ["completion"], "available_on": TARGETS },
        {
            "path": ["completion", "gen"],
            "available_on": TARGETS,
            "flags": [
                { "key": "--shell", "long": "--shell", "takes_value": true, "available_on": TARGETS }
            ],
            "args": [
                { "name": "SHELL", "available_on": TARGETS }
            ]
        }
    ]);

    let union = json!({
        "snapshot_schema_version": 2,
        "tool": "codex-cli",
        "mode": "union",
        "collected_at": TS,
        "expected_targets": TARGETS,
        "complete": true,
        "inputs": inputs,
        "commands": commands,
    });

    let union_path = codex_dir.join("snapshots").join(VERSION).join("union.json");
    write_json(&union_path, &union);
}

fn materialize_minimal_codex_dir_for_report(codex_dir: &Path) {
    fs::create_dir_all(codex_dir).expect("mkdir codex dir");
    copy_from_repo(codex_dir, "SCHEMA.json");
    copy_from_repo(codex_dir, "RULES.json");
    copy_from_repo(codex_dir, "VERSION_METADATA_SCHEMA.json");
    write_union_snapshot(codex_dir);
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
        "xtask is missing `{subcommand}`.\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
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

fn run_xtask_wrapper_coverage(out: &Path, rules: &Path) -> std::process::Output {
    let xtask_bin = PathBuf::from(env!("CARGO_BIN_EXE_xtask"));
    Command::new(xtask_bin)
        .arg("codex-wrapper-coverage")
        .arg("--out")
        .arg(out)
        .arg("--rules")
        .arg(rules)
        .env("SOURCE_DATE_EPOCH", "0")
        .output()
        .expect("spawn xtask codex-wrapper-coverage")
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

fn path_matches(entry: &Value, expected: &[&str]) -> bool {
    entry
        .get("path")
        .and_then(Value::as_array)
        .is_some_and(|tokens| {
            tokens.len() == expected.len()
                && tokens
                    .iter()
                    .zip(expected.iter())
                    .all(|(t, exp)| t.as_str() == Some(*exp))
        })
}

fn assert_wrapper_coverage_contains_iuroot(wrapper_coverage: &Value, path: &[&str], note: &str) {
    let coverage = wrapper_coverage
        .get("coverage")
        .and_then(Value::as_array)
        .expect("wrapper_coverage.coverage array");

    let entry = coverage
        .iter()
        .find(|e| path_matches(e, path))
        .unwrap_or_else(|| panic!("expected IU root entry for path={path:?} in wrapper coverage"));

    assert_eq!(
        entry.get("level").and_then(Value::as_str),
        Some("intentionally_unsupported"),
        "IU root entry must have level=intentionally_unsupported for path={path:?}"
    );
    assert_eq!(
        entry.get("note").and_then(Value::as_str),
        Some(note),
        "IU root entry must have the exact note string for path={path:?}"
    );
}

fn has_path0(entries: &[Value], root: &str) -> bool {
    entries.iter().any(|e| {
        e.get("path")
            .and_then(Value::as_array)
            .and_then(|p| p.first())
            .and_then(Value::as_str)
            == Some(root)
    })
}

#[test]
fn c7_iu_roots_are_generated_and_reports_waive_descendants() {
    let temp = make_temp_dir("ccm-c7-iu-roots-adoption");
    let codex_dir = temp.join("cli_manifests").join("codex");
    materialize_minimal_codex_dir_for_report(&codex_dir);

    let rules = codex_dir.join("RULES.json");
    let wrapper_coverage_out = codex_dir.join("wrapper_coverage.json");
    let output = run_xtask_wrapper_coverage(&wrapper_coverage_out, &rules);
    assert!(
        output.status.success(),
        "expected codex-wrapper-coverage success:\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let wrapper_coverage = read_json(&wrapper_coverage_out);
    assert_eq!(
        wrapper_coverage.get("generated_at").and_then(Value::as_str),
        Some(TS),
        "wrapper coverage generated_at must be deterministic with SOURCE_DATE_EPOCH=0"
    );
    assert_wrapper_coverage_contains_iuroot(&wrapper_coverage, &["completion"], NOTE_COMPLETION);

    let output = run_xtask_codex_report(&codex_dir);
    assert!(
        output.status.success(),
        "expected codex-report success:\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let report_path = codex_dir
        .join("reports")
        .join(VERSION)
        .join("coverage.any.json");
    let report = read_json(&report_path);

    let deltas = report
        .get("deltas")
        .and_then(Value::as_object)
        .expect("report.deltas object");

    let missing_commands = deltas
        .get("missing_commands")
        .and_then(Value::as_array)
        .expect("missing_commands array");
    let missing_flags = deltas
        .get("missing_flags")
        .and_then(Value::as_array)
        .expect("missing_flags array");
    let missing_args = deltas
        .get("missing_args")
        .and_then(Value::as_array)
        .expect("missing_args array");
    let iu = deltas
        .get("intentionally_unsupported")
        .and_then(Value::as_array)
        .expect("deltas.intentionally_unsupported array");

    {
        let root = "completion";
        assert!(
            !has_path0(missing_commands, root),
            "missing_commands must not contain IU descendants under {root}"
        );
        assert!(
            !has_path0(missing_flags, root),
            "missing_flags must not contain IU descendants under {root}"
        );
        assert!(
            !has_path0(missing_args, root),
            "missing_args must not contain IU descendants under {root}"
        );
        assert!(
            has_path0(iu, root),
            "deltas.intentionally_unsupported must include audit-visible IU entries under {root}"
        );
    }
}
