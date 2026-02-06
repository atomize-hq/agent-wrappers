use super::*;
use semver::Version as SemverVersion;
use std::collections::BTreeSet;

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
