use super::*;

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
