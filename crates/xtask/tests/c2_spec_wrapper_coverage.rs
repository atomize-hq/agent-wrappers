use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};

const VERSION: &str = "0.61.0";
const TS: &str = "1970-01-01T00:00:00Z";

const REQUIRED_TARGET: &str = "x86_64-unknown-linux-musl";
const TARGETS: [&str; 3] = [
    "x86_64-unknown-linux-musl",
    "aarch64-apple-darwin",
    "x86_64-pc-windows-msvc",
];

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

fn copy_from_repo(codex_dir: &Path, filename: &str) {
    let src = repo_root()
        .join("cli_manifests")
        .join("codex")
        .join(filename);
    let dst = codex_dir.join(filename);
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent).expect("mkdir dst parent");
    }
    fs::copy(&src, &dst).unwrap_or_else(|e| panic!("copy {:?} -> {:?}: {}", src, dst, e));
}

fn materialize_minimal_valid_codex_dir(codex_dir: &Path, wrapper_coverage: &Value) {
    fs::create_dir_all(codex_dir).expect("mkdir codex dir");

    copy_from_repo(codex_dir, "SCHEMA.json");
    copy_from_repo(codex_dir, "RULES.json");
    copy_from_repo(codex_dir, "VERSION_METADATA_SCHEMA.json");

    write_text(
        &codex_dir.join("min_supported.txt"),
        &format!("{VERSION}\n"),
    );
    write_text(
        &codex_dir.join("latest_validated.txt"),
        &format!("{VERSION}\n"),
    );

    for target in TARGETS {
        let supported = codex_dir
            .join("pointers")
            .join("latest_supported")
            .join(format!("{target}.txt"));
        let validated = codex_dir
            .join("pointers")
            .join("latest_validated")
            .join(format!("{target}.txt"));

        if target == REQUIRED_TARGET {
            write_text(&supported, &format!("{VERSION}\n"));
            write_text(&validated, &format!("{VERSION}\n"));
        } else {
            write_text(&supported, "none\n");
            write_text(&validated, "none\n");
        }
    }

    let inputs = json!([{
        "target_triple": REQUIRED_TARGET,
        "collected_at": TS,
        "binary": {
            "sha256": "00",
            "size_bytes": 0,
            "platform": { "os": "linux", "arch": "x86_64" },
            "target_triple": REQUIRED_TARGET,
            "version_output": format!("codex-cli {VERSION}"),
            "semantic_version": VERSION,
            "channel": "stable",
        }
    }]);

    let union = json!({
        "snapshot_schema_version": 2,
        "tool": "codex-cli",
        "mode": "union",
        "collected_at": TS,
        "expected_targets": TARGETS,
        "complete": false,
        "missing_targets": [TARGETS[1], TARGETS[2]],
        "inputs": inputs,
        "commands": [],
    });

    let union_path = codex_dir.join("snapshots").join(VERSION).join("union.json");
    write_json(&union_path, &union);

    let per_target = json!({
        "snapshot_schema_version": 1,
        "tool": "codex-cli",
        "collected_at": TS,
        "binary": {
            "sha256": "00",
            "size_bytes": 0,
            "platform": { "os": "linux", "arch": "x86_64" },
            "target_triple": REQUIRED_TARGET,
            "version_output": format!("codex-cli {VERSION}"),
            "semantic_version": VERSION,
            "channel": "stable",
        },
        "commands": [],
    });
    write_json(
        &codex_dir
            .join("snapshots")
            .join(VERSION)
            .join(format!("{REQUIRED_TARGET}.json")),
        &per_target,
    );

    let union_text = fs::read_to_string(&union_path).expect("read union.json text");
    write_text(&codex_dir.join("current.json"), &union_text);

    let version_metadata = json!({
        "schema_version": 1,
        "semantic_version": VERSION,
        "status": "snapshotted",
        "updated_at": TS,
        "coverage": {
            "supported_targets": [REQUIRED_TARGET],
            "supported_required_target": true
        },
        "validation": {
            "passed_targets": [REQUIRED_TARGET],
            "failed_targets": [],
            "skipped_targets": []
        }
    });
    write_json(
        &codex_dir.join("versions").join(format!("{VERSION}.json")),
        &version_metadata,
    );

    write_json(&codex_dir.join("wrapper_coverage.json"), wrapper_coverage);
}

fn run_xtask_validate(codex_dir: &Path) -> std::process::Output {
    let xtask_bin = PathBuf::from(env!("CARGO_BIN_EXE_xtask"));
    let fixture_root = codex_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("codex_dir is <fixture_root>/cli_manifests/codex");

    let help = Command::new(&xtask_bin)
        .arg("codex-validate")
        .arg("--help")
        .current_dir(fixture_root)
        .output()
        .expect("spawn xtask codex-validate --help");
    assert!(
        help.status.success(),
        "xtask codex-validate --help failed:\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
        help.status,
        String::from_utf8_lossy(&help.stdout),
        String::from_utf8_lossy(&help.stderr)
    );
    let help_text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&help.stdout),
        String::from_utf8_lossy(&help.stderr)
    );

    let mut cmd = Command::new(xtask_bin);
    cmd.arg("codex-validate");
    cmd.current_dir(fixture_root);
    if help_text.contains("--root") {
        cmd.arg("--root").arg(codex_dir);
    } else if help_text.contains("--codex-dir") {
        cmd.arg("--codex-dir").arg(codex_dir);
    } else {
        panic!("codex-validate help did not contain --root or --codex-dir:\n{help_text}");
    }

    cmd.output().expect("spawn xtask codex-validate")
}

fn read_codex_wrapper_version() -> String {
    let cargo_toml = repo_root().join("crates").join("codex").join("Cargo.toml");
    let text = fs::read_to_string(&cargo_toml)
        .unwrap_or_else(|e| panic!("read {}: {e}", cargo_toml.display()));

    let mut in_package = false;
    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            in_package = line == "[package]";
            continue;
        }
        if !in_package {
            continue;
        }
        let Some(rest) = line.strip_prefix("version") else {
            continue;
        };
        let Some(rest) = rest.trim_start().strip_prefix('=') else {
            continue;
        };
        let val = rest.trim();
        if let Some(stripped) = val.strip_prefix('"').and_then(|v| v.strip_suffix('"')) {
            return stripped.to_string();
        }
    }

    panic!(
        "failed to locate [package].version in {}",
        cargo_toml.display()
    );
}

fn run_xtask_wrapper_coverage(out: &Path, rules: &Path) -> std::process::Output {
    let xtask_bin = PathBuf::from(env!("CARGO_BIN_EXE_xtask"));
    let output = Command::new(xtask_bin)
        .arg("codex-wrapper-coverage")
        .arg("--out")
        .arg(out)
        .arg("--rules")
        .arg(rules)
        .env("SOURCE_DATE_EPOCH", "0")
        .output()
        .expect("spawn xtask codex-wrapper-coverage");

    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.contains("unrecognized subcommand 'codex-wrapper-coverage'") {
        panic!("xtask is missing `codex-wrapper-coverage` (C2-code must add the subcommand)");
    }

    output
}

#[test]
fn c2_wrapper_coverage_generation_is_deterministic_and_includes_wrapper_version() {
    let temp = make_temp_dir("ccm-c2-wrapper-coverage-determinism");
    let out_a = temp.join("wrapper_coverage.a.json");
    let out_b = temp.join("wrapper_coverage.b.json");
    let rules = repo_root()
        .join("cli_manifests")
        .join("codex")
        .join("RULES.json");

    let output_a = run_xtask_wrapper_coverage(&out_a, &rules);
    if !output_a.status.success() {
        panic!(
            "xtask codex-wrapper-coverage failed:\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
            output_a.status,
            String::from_utf8_lossy(&output_a.stdout),
            String::from_utf8_lossy(&output_a.stderr),
        );
    }
    let output_b = run_xtask_wrapper_coverage(&out_b, &rules);
    if !output_b.status.success() {
        panic!(
            "xtask codex-wrapper-coverage failed:\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
            output_b.status,
            String::from_utf8_lossy(&output_b.stdout),
            String::from_utf8_lossy(&output_b.stderr),
        );
    }

    let bytes_a = fs::read(&out_a).expect("read wrapper_coverage.a.json");
    let bytes_b = fs::read(&out_b).expect("read wrapper_coverage.b.json");
    assert_eq!(bytes_a, bytes_b, "output must be byte-identical");
    assert!(
        bytes_a.last().is_some_and(|b| *b == b'\n'),
        "output must end with a trailing newline"
    );

    let parsed: Value = serde_json::from_slice(&bytes_a).expect("parse wrapper_coverage JSON");
    assert_eq!(
        parsed.get("schema_version").and_then(Value::as_i64),
        Some(1),
        "schema_version must be 1"
    );
    assert!(
        parsed.get("generated_at").and_then(Value::as_str).is_some(),
        "generated_at must be present"
    );

    let expected_version = read_codex_wrapper_version();
    assert_eq!(
        parsed
            .get("wrapper_version")
            .and_then(Value::as_str)
            .map(|s| s.to_string()),
        Some(expected_version),
        "wrapper_version must match crates/codex/Cargo.toml [package].version"
    );
}

#[test]
fn c2_validate_rejects_intentionally_unsupported_without_note() {
    let temp = make_temp_dir("ccm-c2-wrapper-iu-note");
    let codex_dir = temp.join("cli_manifests").join("codex");

    let wrapper_coverage = json!({
        "schema_version": 1,
        "generated_at": TS,
        "wrapper_version": "0.0.0-test",
        "coverage": [{
            "path": ["codex"],
            "level": "intentionally_unsupported"
        }]
    });
    materialize_minimal_valid_codex_dir(&codex_dir, &wrapper_coverage);

    let output = run_xtask_validate(&codex_dir);
    assert!(
        !output.status.success(),
        "expected validation failure:\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("IU_NOTE_MISSING") && stderr.contains("wrapper_coverage.json"),
        "expected IU_NOTE_MISSING for wrapper_coverage.json, got:\n{stderr}"
    );
}

#[test]
fn c2_validate_rejects_passthrough_without_note() {
    let temp = make_temp_dir("ccm-c2-wrapper-passthrough-note");
    let codex_dir = temp.join("cli_manifests").join("codex");

    let wrapper_coverage = json!({
        "schema_version": 1,
        "generated_at": TS,
        "wrapper_version": "0.0.0-test",
        "coverage": [{
            "path": ["codex"],
            "level": "passthrough"
        }]
    });
    materialize_minimal_valid_codex_dir(&codex_dir, &wrapper_coverage);

    let output = run_xtask_validate(&codex_dir);
    assert!(
        !output.status.success(),
        "expected validation failure:\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("PASSTHROUGH_NOTE_MISSING") && stderr.contains("wrapper_coverage.json"),
        "expected PASSTHROUGH_NOTE_MISSING for wrapper_coverage.json, got:\n{stderr}"
    );
}

#[test]
fn c2_validate_rejects_overlapping_wrapper_scopes_no_scope_means_all_expected_targets() {
    let temp = make_temp_dir("ccm-c2-wrapper-overlap-no-scope");
    let codex_dir = temp.join("cli_manifests").join("codex");

    let wrapper_coverage = json!({
        "schema_version": 1,
        "generated_at": TS,
        "wrapper_version": "0.0.0-test",
        "coverage": [
            { "path": ["codex"], "level": "passthrough" },
            {
                "path": ["codex"],
                "level": "explicit",
                "scope": { "target_triples": [REQUIRED_TARGET] }
            }
        ]
    });
    materialize_minimal_valid_codex_dir(&codex_dir, &wrapper_coverage);

    let output = run_xtask_validate(&codex_dir);
    assert!(
        !output.status.success(),
        "expected validation failure:\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("WRAPPER_SCOPE_OVERLAP")
            && stderr.contains("target_triple=")
            && stderr.contains("matching_entry_indexes=")
            && stderr.contains("wrapper_coverage.json"),
        "expected overlap violation details for wrapper_coverage.json, got:\n{stderr}"
    );
}

#[test]
fn c2_validate_rejects_overlapping_wrapper_scopes_platforms_expand_via_union_platform_mapping() {
    let temp = make_temp_dir("ccm-c2-wrapper-overlap-platforms");
    let codex_dir = temp.join("cli_manifests").join("codex");

    let wrapper_coverage = json!({
        "schema_version": 1,
        "generated_at": TS,
        "wrapper_version": "0.0.0-test",
        "coverage": [
            {
                "path": ["codex"],
                "level": "passthrough",
                "scope": { "platforms": ["linux"] }
            },
            {
                "path": ["codex"],
                "level": "explicit",
                "scope": { "target_triples": [REQUIRED_TARGET] }
            }
        ]
    });
    materialize_minimal_valid_codex_dir(&codex_dir, &wrapper_coverage);

    let output = run_xtask_validate(&codex_dir);
    assert!(
        !output.status.success(),
        "expected validation failure:\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("WRAPPER_SCOPE_OVERLAP")
            && stderr.contains("target_triple=x86_64-unknown-linux-musl")
            && stderr.contains("wrapper_coverage.json"),
        "expected overlap for linux target triple via platforms expansion, got:\n{stderr}"
    );
}
