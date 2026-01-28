#[cfg(unix)]
mod unix {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    use serde_json::Value;

    fn fixtures_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
    }

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .expect("CARGO_MANIFEST_DIR has 2 parents (crates/xtask)")
            .to_path_buf()
    }

    fn write_rules_json(codex_root: &Path) {
        fs::create_dir_all(codex_root).expect("create codex root dir");
        let src = repo_root()
            .join("cli_manifests")
            .join("codex")
            .join("RULES.json");
        let dst = codex_root.join("RULES.json");
        fs::copy(src, dst).expect("copy RULES.json");
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

    fn copy_executable_fixture(fixture_name: &str, dst_dir: &Path) -> PathBuf {
        let src = fixtures_dir().join(fixture_name);
        let dst = dst_dir.join(fixture_name);
        fs::copy(&src, &dst).expect("copy executable fixture");

        let mut perms = fs::metadata(&dst).expect("stat fixture").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&dst, perms).expect("chmod fixture");

        dst
    }

    fn copy_fixture(fixture_name: &str, dst_dir: &Path) -> PathBuf {
        let src = fixtures_dir().join(fixture_name);
        let dst = dst_dir.join(fixture_name);
        fs::copy(&src, &dst).expect("copy fixture");
        dst
    }

    #[test]
    fn c1_snapshot_writes_per_target_out_file_and_raw_help_under_codex_root() {
        let temp = make_temp_dir("ccm-c1-snapshot-per-target");

        let codex_bin = copy_executable_fixture("fake_codex.sh", &temp);
        let supplement = copy_fixture("supplement_commands.json", &temp);

        let codex_root = temp.join("cli_manifests").join("codex");
        write_rules_json(&codex_root);
        let version = "0.77.0";
        let target = "x86_64-unknown-linux-musl";

        let out_file = codex_root
            .join("snapshots")
            .join(version)
            .join(format!("{target}.json"));
        fs::create_dir_all(out_file.parent().expect("out_file parent"))
            .expect("create snapshots/<version> dir");

        let xtask_bin = PathBuf::from(env!("CARGO_BIN_EXE_xtask"));
        let output = Command::new(xtask_bin)
            .arg("codex-snapshot")
            .arg("--codex-binary")
            .arg(&codex_bin)
            .arg("--out-file")
            .arg(&out_file)
            .arg("--capture-raw-help")
            .arg("--raw-help-target")
            .arg(target)
            .arg("--supplement")
            .arg(&supplement)
            .env("SOURCE_DATE_EPOCH", "0")
            .output()
            .expect("spawn xtask codex-snapshot");

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("unexpected argument '--out-file' found") {
                panic!("xtask codex-snapshot is missing per-target mode flags (needs `--out-file` + `--raw-help-target` per C1-spec)");
            }
            panic!(
                "xtask codex-snapshot failed:\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
                output.status,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }

        let snapshot_text = fs::read_to_string(&out_file).expect("read per-target snapshot file");
        let snapshot: Value = serde_json::from_str(&snapshot_text).expect("parse snapshot JSON");
        assert_eq!(
            snapshot
                .get("binary")
                .and_then(|b| b.get("target_triple"))
                .and_then(Value::as_str),
            Some(target),
            "binary.target_triple reflects --raw-help-target"
        );

        let raw_help = codex_root
            .join("raw_help")
            .join(version)
            .join(target)
            .join("help.txt");
        assert!(
            raw_help.is_file(),
            "raw help is captured under raw_help/<version>/<target>/help.txt"
        );
    }

    #[test]
    fn c1_snapshot_rejects_out_dir_when_out_file_is_set() {
        let temp = make_temp_dir("ccm-c1-snapshot-out-file-xor-out-dir");

        let codex_bin = copy_executable_fixture("fake_codex.sh", &temp);
        let supplement = copy_fixture("supplement_commands.json", &temp);

        let codex_root = temp.join("cli_manifests").join("codex");
        write_rules_json(&codex_root);
        let version = "0.77.0";
        let target = "x86_64-unknown-linux-musl";
        let out_file = codex_root
            .join("snapshots")
            .join(version)
            .join(format!("{target}.json"));
        fs::create_dir_all(out_file.parent().expect("out_file parent"))
            .expect("create snapshots/<version> dir");

        let out_dir = temp.join("out_dir");
        fs::create_dir_all(&out_dir).expect("create out_dir");

        let xtask_bin = PathBuf::from(env!("CARGO_BIN_EXE_xtask"));
        let output = Command::new(xtask_bin)
            .arg("codex-snapshot")
            .arg("--codex-binary")
            .arg(&codex_bin)
            .arg("--out-file")
            .arg(&out_file)
            .arg("--out-dir")
            .arg(&out_dir)
            .arg("--capture-raw-help")
            .arg("--raw-help-target")
            .arg(target)
            .arg("--supplement")
            .arg(&supplement)
            .output()
            .expect("spawn xtask codex-snapshot");

        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("unexpected argument '--out-file' found") {
            panic!("xtask codex-snapshot is missing per-target mode flags (needs `--out-file` + `--raw-help-target` per C1-spec)");
        }

        assert!(
            !output.status.success(),
            "expected failure when --out-file is combined with --out-dir"
        );
        assert!(
            stderr.contains("--out-file") && stderr.contains("--out-dir"),
            "stderr should mention both conflicting flags; got:\n{stderr}"
        );
    }
}
