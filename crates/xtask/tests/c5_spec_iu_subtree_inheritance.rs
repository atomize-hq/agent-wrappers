use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use jsonschema::{Draft, JSONSchema};
use serde_json::{json, Value};

const VERSION: &str = "0.61.0";
const TS: &str = "1970-01-01T00:00:00Z";

const TARGET_LINUX: &str = "x86_64-unknown-linux-musl";
const TARGET_MACOS: &str = "aarch64-apple-darwin";
const TARGET_WINDOWS: &str = "x86_64-pc-windows-msvc";
const TARGETS: [&str; 3] = [TARGET_LINUX, TARGET_MACOS, TARGET_WINDOWS];

const IU_NOTE: &str = "waived subtree for C0 IU inheritance test";

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
        { "path": ["waived"], "available_on": TARGETS },
        {
            "path": ["waived", "child-a"],
            "available_on": TARGETS,
            "flags": [
                { "key": "--zzz", "long": "--zzz", "takes_value": false, "available_on": TARGETS },
                { "key": "--aaa", "long": "--aaa", "takes_value": false, "available_on": TARGETS }
            ],
            "args": [
                { "name": "ARG_A", "available_on": TARGETS }
            ]
        },
        {
            "path": ["waived", "child-b"],
            "available_on": TARGETS,
            "flags": [
                { "key": "--bbb", "long": "--bbb", "takes_value": false, "available_on": TARGETS }
            ],
            "args": [
                { "name": "ARG_B", "available_on": TARGETS }
            ]
        },
        { "path": ["waived", "override"], "available_on": TARGETS }
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

fn write_wrapper_coverage(codex_dir: &Path) {
    let wrapper_coverage = json!({
        "schema_version": 1,
        "generated_at": TS,
        "wrapper_version": "0.0.0-test",
        "coverage": [
            {
                "path": ["waived"],
                "level": "intentionally_unsupported",
                "note": IU_NOTE
            },
            {
                "path": ["waived", "override"],
                "level": "explicit"
            }
        ]
    });
    write_json(&codex_dir.join("wrapper_coverage.json"), &wrapper_coverage);
}

fn materialize_minimal_codex_dir_for_report(codex_dir: &Path) {
    fs::create_dir_all(codex_dir).expect("mkdir codex dir");
    copy_from_repo(codex_dir, "SCHEMA.json");
    copy_from_repo(codex_dir, "RULES.json");
    copy_from_repo(codex_dir, "VERSION_METADATA_SCHEMA.json");

    write_union_snapshot(codex_dir);
    write_wrapper_coverage(codex_dir);
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

fn to_path_tokens(v: &Value) -> Vec<String> {
    v.as_array()
        .expect("path array")
        .iter()
        .map(|t| t.as_str().expect("path token string").to_string())
        .collect()
}

fn sort_key(v: &Value) -> (u8, Vec<String>, String) {
    let path = to_path_tokens(
        v.get("path")
            .unwrap_or_else(|| panic!("missing path in IU entry: {v}")),
    );

    if let Some(key) = v.get("key").and_then(|k| k.as_str()) {
        (1, path, key.to_string())
    } else if let Some(name) = v.get("name").and_then(|n| n.as_str()) {
        (2, path, name.to_string())
    } else {
        (0, path, String::new())
    }
}

#[test]
fn c5_iu_subtree_inheritance_classifies_descendants_and_sorts_deterministically() {
    let temp = make_temp_dir("ccm-c5-iu-subtree-inheritance");
    let codex_dir = temp.join("cli_manifests").join("codex");
    materialize_minimal_codex_dir_for_report(&codex_dir);

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

    let schema = compile_schema_with_file_id(&codex_dir.join("SCHEMA.json"));
    assert_schema_valid(&schema, &report);

    assert_eq!(
        report.get("generated_at").and_then(|v| v.as_str()),
        Some(TS),
        "generated_at must be deterministic with SOURCE_DATE_EPOCH=0"
    );

    let deltas = report
        .get("deltas")
        .and_then(|v| v.as_object())
        .expect("report.deltas object");

    assert_eq!(
        deltas
            .get("missing_commands")
            .and_then(|v| v.as_array())
            .expect("missing_commands array")
            .len(),
        0,
        "IU descendants must not appear under missing_commands"
    );
    assert_eq!(
        deltas
            .get("missing_flags")
            .and_then(|v| v.as_array())
            .expect("missing_flags array")
            .len(),
        0,
        "IU descendants must not appear under missing_flags"
    );
    assert_eq!(
        deltas
            .get("missing_args")
            .and_then(|v| v.as_array())
            .expect("missing_args array")
            .len(),
        0,
        "IU descendants must not appear under missing_args"
    );

    let iu = deltas
        .get("intentionally_unsupported")
        .and_then(|v| v.as_array())
        .expect("deltas.intentionally_unsupported array");

    let repr = iu
        .iter()
        .map(|v| {
            let (kind, path, key_or_name) = sort_key(v);
            (kind, path, key_or_name)
        })
        .collect::<Vec<_>>();

    let expected = vec![
        (0, vec!["waived".to_string()], "".to_string()),
        (
            0,
            vec!["waived".to_string(), "child-a".to_string()],
            "".to_string(),
        ),
        (
            0,
            vec!["waived".to_string(), "child-b".to_string()],
            "".to_string(),
        ),
        (
            1,
            vec!["waived".to_string(), "child-a".to_string()],
            "--aaa".to_string(),
        ),
        (
            1,
            vec!["waived".to_string(), "child-a".to_string()],
            "--zzz".to_string(),
        ),
        (
            1,
            vec!["waived".to_string(), "child-b".to_string()],
            "--bbb".to_string(),
        ),
        (
            2,
            vec!["waived".to_string(), "child-a".to_string()],
            "ARG_A".to_string(),
        ),
        (
            2,
            vec!["waived".to_string(), "child-b".to_string()],
            "ARG_B".to_string(),
        ),
    ];

    assert_eq!(
        repr, expected,
        "IU delta list must match ADR 0004 ordering (kind, path, key/name)"
    );

    for entry in iu {
        assert_eq!(
            entry.get("wrapper_level").and_then(|v| v.as_str()),
            Some("intentionally_unsupported"),
            "IU entries must set wrapper_level=intentionally_unsupported"
        );
        assert_eq!(
            entry.get("note").and_then(|v| v.as_str()),
            Some(IU_NOTE),
            "IU entries must include inherited note"
        );
    }

    assert!(
        !iu.iter().any(|v| {
            to_path_tokens(v.get("path").expect("IU entry path")) == vec!["waived", "override"]
        }),
        "explicitly-declared descendant must not be IU by inheritance"
    );
}
