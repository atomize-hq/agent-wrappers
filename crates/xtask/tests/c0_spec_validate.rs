use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::json;

const VERSION: &str = "0.61.0";
const TS: &str = "1970-01-01T00:00:00Z";

const REQUIRED_TARGET: &str = "x86_64-unknown-linux-musl";
const TARGETS: [&str; 3] = [
    "x86_64-unknown-linux-musl",
    "aarch64-apple-darwin",
    "x86_64-pc-windows-msvc",
];

fn workspace_root() -> PathBuf {
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

fn write_json(path: &Path, value: &serde_json::Value) {
    let text = serde_json::to_string_pretty(value).expect("serialize json");
    write_text(path, &format!("{text}\n"));
}

fn copy_from_repo(codex_dir: &Path, filename: &str) {
    let src = workspace_root()
        .join("cli_manifests")
        .join("codex")
        .join(filename);
    let dst = codex_dir.join(filename);
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent).expect("mkdir dst parent");
    }
    fs::copy(&src, &dst).unwrap_or_else(|e| panic!("copy {:?} -> {:?}: {}", src, dst, e));
}

fn materialize_minimal_valid_codex_dir(
    codex_dir: &Path,
    version_status: &str,
    union_complete: bool,
) {
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

    let union_inputs = if union_complete {
        TARGETS.to_vec()
    } else {
        vec![REQUIRED_TARGET]
    };

    let inputs = union_inputs
        .iter()
        .map(|target| {
            let (os, arch) = match *target {
                "x86_64-unknown-linux-musl" => ("linux", "x86_64"),
                "aarch64-apple-darwin" => ("macos", "aarch64"),
                "x86_64-pc-windows-msvc" => ("windows", "x86_64"),
                _ => ("unknown", "unknown"),
            };

            json!({
                "target_triple": target,
                "collected_at": TS,
                "binary": {
                    "sha256": "00",
                    "size_bytes": 0,
                    "platform": { "os": os, "arch": arch },
                    "target_triple": target,
                    "version_output": format!("codex-cli {VERSION}"),
                    "semantic_version": VERSION,
                    "channel": "stable",
                }
            })
        })
        .collect::<Vec<_>>();

    let union = if union_complete {
        json!({
            "snapshot_schema_version": 2,
            "tool": "codex-cli",
            "mode": "union",
            "collected_at": TS,
            "expected_targets": TARGETS,
            "complete": true,
            "inputs": inputs,
            "commands": [],
        })
    } else {
        json!({
            "snapshot_schema_version": 2,
            "tool": "codex-cli",
            "mode": "union",
            "collected_at": TS,
            "expected_targets": TARGETS,
            "complete": false,
            "missing_targets": [TARGETS[1], TARGETS[2]],
            "inputs": inputs,
            "commands": [],
        })
    };

    let union_path = codex_dir.join("snapshots").join(VERSION).join("union.json");
    write_json(&union_path, &union);

    for target in &union_inputs {
        let (os, arch) = match *target {
            "x86_64-unknown-linux-musl" => ("linux", "x86_64"),
            "aarch64-apple-darwin" => ("macos", "aarch64"),
            "x86_64-pc-windows-msvc" => ("windows", "x86_64"),
            _ => ("unknown", "unknown"),
        };

        let per_target = json!({
            "snapshot_schema_version": 1,
            "tool": "codex-cli",
            "collected_at": TS,
            "binary": {
                "sha256": "00",
                "size_bytes": 0,
                "platform": { "os": os, "arch": arch },
                "target_triple": target,
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
                .join(format!("{target}.json")),
            &per_target,
        );
    }

    let union_text = fs::read_to_string(&union_path).expect("read union.json text");
    write_text(&codex_dir.join("current.json"), &union_text);

    let version_metadata = json!({
        "schema_version": 1,
        "semantic_version": VERSION,
        "status": version_status,
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

    let wrapper_coverage = json!({
        "schema_version": 1,
        "generated_at": TS,
        "wrapper_version": "0.0.0-test",
        "coverage": []
    });
    write_json(&codex_dir.join("wrapper_coverage.json"), &wrapper_coverage);
}

fn write_minimal_report_files(codex_dir: &Path, input_targets: &[&str], include_all: bool) {
    let report = json!({
        "schema_version": 1,
        "generated_at": TS,
        "inputs": {
            "upstream": {
                "semantic_version": VERSION,
                "mode": "union",
                "targets": input_targets
            },
            "wrapper": {
                "schema_version": 1,
                "wrapper_version": "0.0.0-test"
            },
            "rules": {
                "rules_schema_version": 1
            }
        },
        "platform_filter": {
            "mode": "any"
        },
        "deltas": {
            "missing_commands": [],
            "missing_flags": [],
            "missing_args": []
        }
    });

    let reports_dir = codex_dir.join("reports").join(VERSION);
    write_json(&reports_dir.join("coverage.any.json"), &report);
    for target in input_targets {
        write_json(
            &reports_dir.join(format!("coverage.{target}.json")),
            &report,
        );
    }
    if include_all {
        write_json(&reports_dir.join("coverage.all.json"), &report);
    }
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

#[test]
fn c0_validate_passes_on_minimal_valid_codex_dir() {
    let temp = make_temp_dir("ccm-c0-validate-pass");
    let codex_dir = temp.join("cli_manifests").join("codex");

    materialize_minimal_valid_codex_dir(&codex_dir, "snapshotted", false);

    let output = run_xtask_validate(&codex_dir);
    assert!(
        output.status.success(),
        "expected success:\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn c0_validate_requires_reports_when_version_status_reported() {
    let temp = make_temp_dir("ccm-c0-validate-reports");
    let codex_dir = temp.join("cli_manifests").join("codex");

    materialize_minimal_valid_codex_dir(&codex_dir, "reported", false);

    let output = run_xtask_validate(&codex_dir);
    assert!(
        !output.status.success(),
        "expected failure:\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("reports") && stderr.contains("coverage.any.json"),
        "expected report requirement violation, got:\n{stderr}"
    );

    write_minimal_report_files(&codex_dir, &[REQUIRED_TARGET], false);
    let output = run_xtask_validate(&codex_dir);
    assert!(
        output.status.success(),
        "expected success after adding required reports:\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn c0_validate_requires_coverage_all_only_when_union_complete() {
    let temp = make_temp_dir("ccm-c0-validate-coverage-all");
    let codex_dir = temp.join("cli_manifests").join("codex");

    materialize_minimal_valid_codex_dir(&codex_dir, "reported", true);
    write_minimal_report_files(&codex_dir, &TARGETS, false);

    let output = run_xtask_validate(&codex_dir);
    assert!(
        !output.status.success(),
        "expected failure (missing coverage.all.json):\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("coverage.all.json"),
        "expected missing coverage.all.json violation, got:\n{stderr}"
    );

    write_minimal_report_files(&codex_dir, &TARGETS, true);
    let output = run_xtask_validate(&codex_dir);
    assert!(
        output.status.success(),
        "expected success after adding coverage.all.json:\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn c0_validate_reports_wrapper_overlap_errors_with_required_fields_and_is_deterministic() {
    let temp = make_temp_dir("ccm-c0-validate-overlap");
    let codex_dir = temp.join("cli_manifests").join("codex");

    materialize_minimal_valid_codex_dir(&codex_dir, "snapshotted", false);

    let wrapper_coverage = json!({
        "schema_version": 1,
        "generated_at": TS,
        "wrapper_version": "0.0.0-test",
        "coverage": [
            {
                "path": ["exec"],
                "level": "explicit",
                "scope": { "platforms": ["linux"] }
            },
            {
                "path": ["exec"],
                "level": "explicit",
                "scope": { "target_triples": [REQUIRED_TARGET] }
            }
        ]
    });
    write_json(&codex_dir.join("wrapper_coverage.json"), &wrapper_coverage);

    let a = run_xtask_validate(&codex_dir);
    let b = run_xtask_validate(&codex_dir);
    assert!(
        !a.status.success(),
        "expected overlap failure:\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
        a.status,
        String::from_utf8_lossy(&a.stdout),
        String::from_utf8_lossy(&a.stderr)
    );
    assert_eq!(
        a.stderr, b.stderr,
        "validator output must be deterministic for identical inputs"
    );

    let stderr = String::from_utf8_lossy(&a.stderr);
    assert!(
        stderr.contains("wrapper_coverage.json"),
        "expected wrapper_coverage.json path in errors, got:\n{stderr}"
    );
    assert!(
        stderr.contains(REQUIRED_TARGET),
        "expected target triple in overlap errors, got:\n{stderr}"
    );
    assert!(
        stderr.contains("exec"),
        "expected unit key (command path) in overlap errors, got:\n{stderr}"
    );
    assert!(
        stderr.contains("0") && stderr.contains("1"),
        "expected matching entry indexes mentioned in overlap errors, got:\n{stderr}"
    );
}

#[test]
fn c0_validate_rejects_pointer_files_without_trailing_newline() {
    let temp = make_temp_dir("ccm-c0-validate-pointer-newline");
    let codex_dir = temp.join("cli_manifests").join("codex");

    materialize_minimal_valid_codex_dir(&codex_dir, "snapshotted", false);
    write_text(&codex_dir.join("latest_validated.txt"), VERSION);

    let output = run_xtask_validate(&codex_dir);
    assert!(
        !output.status.success(),
        "expected pointer format failure:\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("latest_validated.txt"),
        "expected latest_validated.txt referenced in errors, got:\n{stderr}"
    );
}
