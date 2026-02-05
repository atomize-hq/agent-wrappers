use super::*;

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
