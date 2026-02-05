use super::*;

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
