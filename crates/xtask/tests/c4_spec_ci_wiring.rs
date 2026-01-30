use std::fs;
use std::path::PathBuf;

use regex::Regex;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("CARGO_MANIFEST_DIR has crates/<crate> parent structure")
        .to_path_buf()
}

fn read_repo_file(relative_path: &str) -> String {
    let path = repo_root().join(relative_path);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path:?}: {e}"))
}

#[test]
fn c4_spec_update_snapshot_workflow_runs_full_pipeline_and_uploads_artifacts() {
    let yml = read_repo_file(".github/workflows/codex-cli-update-snapshot.yml");

    // C4-spec: acquire pinned upstream binaries using artifacts.lock + RULES.json expected targets.
    assert!(
        yml.contains("cli_manifests/codex/artifacts.lock.json"),
        "workflow must reference cli_manifests/codex/artifacts.lock.json to acquire pinned binaries"
    );
    assert!(
        yml.contains("cli_manifests/codex/RULES.json"),
        "workflow must reference cli_manifests/codex/RULES.json (for union.expected_targets contract)"
    );
    assert!(
        yml.contains("expected_targets"),
        "workflow must reference RULES.json union.expected_targets (expected_targets)"
    );

    // C4-spec: per-target snapshots should run on Linux/macOS/Windows.
    assert!(
        yml.contains("ubuntu-"),
        "workflow must include at least one ubuntu runs-on job (Linux snapshots + union stage)"
    );
    assert!(
        yml.contains("macos-"),
        "workflow must include at least one macos runs-on job (macOS snapshots)"
    );
    assert!(
        yml.contains("windows-"),
        "workflow must include at least one windows runs-on job (Windows snapshots)"
    );

    // C4-spec: generate per-target snapshots + raw help captures and upload raw help as CI artifacts.
    assert!(
        yml.contains("codex-snapshot"),
        "workflow must run xtask codex-snapshot"
    );
    assert!(
        yml.contains("cli_manifests/codex/raw_help/"),
        "workflow must capture/upload raw help under cli_manifests/codex/raw_help/<version>/<target_triple>/"
    );
    assert!(
        yml.contains("actions/upload-artifact@v4"),
        "workflow must upload raw help and artifact bundles via actions/upload-artifact@v4"
    );

    // C4-spec: on Linux, run union → wrapper-coverage → report → version-metadata → validate.
    for required in [
        "codex-union",
        "codex-wrapper-coverage",
        "codex-report",
        "codex-version-metadata",
        "codex-validate",
    ] {
        assert!(
            yml.contains(required),
            "workflow must run xtask {required} as part of the end-to-end pipeline"
        );
    }

    // C4-spec: upload artifact bundle containing snapshots/reports/versions + wrapper coverage.
    for required_path in [
        "cli_manifests/codex/snapshots/",
        "cli_manifests/codex/reports/",
        "cli_manifests/codex/versions/",
        "cli_manifests/codex/wrapper_coverage.json",
    ] {
        assert!(
            yml.contains(required_path),
            "workflow must upload committed-artifact bundle including {required_path}"
        );
    }
}

#[test]
fn c4_spec_ci_workflow_has_conditional_codex_validate_gate() {
    let yml = read_repo_file(".github/workflows/ci.yml");

    // C4-spec (normative): gate runs only when committed artifacts regime is active.
    //
    // Two supported implementations:
    // - job-level hashFiles gate
    // - a first step that detects committed versions and gates subsequent steps via outputs
    let has_hashfiles_gate =
        yml.contains("hashFiles('cli_manifests/codex/versions/*.json') != ''");
    let has_step_gate = yml.contains("Detect Codex committed artifacts")
        && yml.contains("has_versions")
        && yml.contains("steps.codex-artifacts.outputs.has_versions");
    assert!(
        has_hashfiles_gate || has_step_gate,
        "ci.yml must gate codex-validate behind either hashFiles('cli_manifests/codex/versions/*.json') != '' or a Detect Codex committed artifacts step gate"
    );

    // Ensure the job actually runs codex-validate (not just mentions it).
    let validate_invocation =
        Regex::new(r"cargo\s+run\s+-p\s+xtask\s+--[\s\\]*\n?[\s\\]*codex-validate")
            .expect("valid regex");
    assert!(
        validate_invocation.is_match(&yml),
        "ci.yml must invoke: cargo run -p xtask -- codex-validate"
    );
}
