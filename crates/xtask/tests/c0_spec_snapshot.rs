#[cfg(unix)]
mod unix {
    use std::cmp::Ordering;
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

    fn run_xtask_snapshot(codex_bin: &Path, out_dir: &Path, supplement: &Path) -> Value {
        let xtask_bin = PathBuf::from(env!("CARGO_BIN_EXE_xtask"));

        let output = Command::new(xtask_bin)
            .arg("codex-snapshot")
            .arg("--codex-binary")
            .arg(codex_bin)
            .arg("--out-dir")
            .arg(out_dir)
            .arg("--capture-raw-help")
            .arg("--supplement")
            .arg(supplement)
            .output()
            .expect("spawn xtask codex-snapshot");

        if !output.status.success() {
            panic!(
                "xtask codex-snapshot failed:\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
                output.status,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }

        let snapshot_path = out_dir.join("current.json");
        let snapshot_text = fs::read_to_string(&snapshot_path).expect("read current.json");
        serde_json::from_str(&snapshot_text).expect("parse current.json")
    }

    fn path_cmp(a: &[String], b: &[String]) -> Ordering {
        for (a_token, b_token) in a.iter().zip(b.iter()) {
            match a_token.cmp(b_token) {
                Ordering::Equal => {}
                non_eq => return non_eq,
            }
        }
        a.len().cmp(&b.len())
    }

    fn flag_key(flag: &Value) -> (Option<String>, Option<String>) {
        let long = flag
            .get("long")
            .and_then(Value::as_str)
            .map(|s| s.to_string());
        let short = flag
            .get("short")
            .and_then(Value::as_str)
            .map(|s| s.to_string());
        (long, short)
    }

    fn flag_cmp(a: &Value, b: &Value) -> Ordering {
        let (a_long, a_short) = flag_key(a);
        let (b_long, b_short) = flag_key(b);

        match (a_long, b_long) {
            (Some(a), Some(b)) => match a.cmp(&b) {
                Ordering::Equal => {}
                non_eq => return non_eq,
            },
            (Some(_), None) => return Ordering::Less,
            (None, Some(_)) => return Ordering::Greater,
            (None, None) => {}
        }

        match (a_short, b_short) {
            (Some(a), Some(b)) => a.cmp(&b),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        }
    }

    fn assert_commands_sorted(snapshot: &Value) {
        let commands = snapshot
            .get("commands")
            .and_then(Value::as_array)
            .expect("snapshot.commands is array");

        let mut last: Option<Vec<String>> = None;
        for cmd in commands {
            let path = cmd
                .get("path")
                .and_then(Value::as_array)
                .expect("commands[].path is array")
                .iter()
                .map(|v| {
                    v.as_str()
                        .expect("commands[].path token is string")
                        .to_string()
                })
                .collect::<Vec<_>>();

            if let Some(prev) = &last {
                assert!(
                    path_cmp(prev, &path) != Ordering::Greater,
                    "commands not sorted: {:?} > {:?}",
                    prev,
                    path
                );
            }
            last = Some(path);
        }
    }

    fn assert_command_flags_sorted(snapshot: &Value, command_path: &[&str]) {
        let commands = snapshot
            .get("commands")
            .and_then(Value::as_array)
            .expect("snapshot.commands is array");

        let cmd = commands
            .iter()
            .find(|c| {
                c.get("path").and_then(Value::as_array).is_some_and(|p| {
                    p.iter()
                        .filter_map(Value::as_str)
                        .eq(command_path.iter().copied())
                })
            })
            .unwrap_or_else(|| panic!("missing command path {:?}", command_path));

        let flags = cmd
            .get("flags")
            .and_then(Value::as_array)
            .expect("commands[].flags is array");

        for pair in flags.windows(2) {
            let a = &pair[0];
            let b = &pair[1];
            assert!(
                flag_cmp(a, b) != Ordering::Greater,
                "flags not sorted for {:?}: {:?} > {:?}",
                command_path,
                flag_key(a),
                flag_key(b)
            );
        }
    }

    fn get_command<'a>(snapshot: &'a Value, command_path: &[&str]) -> &'a Value {
        let commands = snapshot
            .get("commands")
            .and_then(Value::as_array)
            .expect("snapshot.commands is array");

        commands
            .iter()
            .find(|c| {
                c.get("path").and_then(Value::as_array).is_some_and(|p| {
                    p.iter()
                        .filter_map(Value::as_str)
                        .eq(command_path.iter().copied())
                })
            })
            .unwrap_or_else(|| panic!("missing command path {:?}", command_path))
    }

    #[test]
    fn c0_snapshot_applies_supplement_and_sorts_commands_and_flags() {
        let temp = make_temp_dir("ccp-c0-test");

        let codex_bin = copy_executable_fixture("fake_codex.sh", &temp);
        let supplement = copy_fixture("supplement_commands.json", &temp);

        let out_dir = temp.join("out");
        fs::create_dir_all(&out_dir).expect("create out dir");

        let snapshot = run_xtask_snapshot(&codex_bin, &out_dir, &supplement);

        assert_eq!(
            snapshot
                .get("snapshot_schema_version")
                .and_then(Value::as_i64),
            Some(1),
            "snapshot_schema_version must be 1"
        );
        assert_eq!(
            snapshot.get("tool").and_then(Value::as_str),
            Some("codex-cli"),
            "tool must be codex-cli"
        );

        assert_commands_sorted(&snapshot);
        assert_command_flags_sorted(&snapshot, &["exec", "start"]);

        let commands = snapshot
            .get("commands")
            .and_then(Value::as_array)
            .expect("snapshot.commands is array");
        let sandbox = commands
            .iter()
            .find(|c| {
                c.get("path")
                    .and_then(Value::as_array)
                    .is_some_and(|p| p.iter().filter_map(Value::as_str).eq(["sandbox"]))
            })
            .expect("supplemented command path [\"sandbox\"] exists in snapshot.commands");

        assert_eq!(
            sandbox
                .get("platforms")
                .and_then(Value::as_array)
                .map(|p| p.iter().filter_map(Value::as_str).collect::<Vec<_>>()),
            Some(vec!["linux", "macos"]),
            "supplemented command platforms applied"
        );

        let omissions = snapshot
            .get("known_omissions")
            .and_then(Value::as_array)
            .expect("snapshot.known_omissions is array");
        assert!(
            omissions
                .iter()
                .any(|o| o.as_str() == Some("supplement/commands.json:v1:sandbox")),
            "known_omissions records applied supplement entry"
        );

        assert!(
            out_dir
                .join("raw_help")
                .join("0.77.0")
                .join("help.txt")
                .is_file(),
            "captures root help at raw_help/<semantic_version>/help.txt"
        );
    }

    #[test]
    fn c0_snapshot_is_deterministic_except_for_collected_at() {
        let temp = make_temp_dir("ccp-c0-test-determinism");

        let codex_bin = copy_executable_fixture("fake_codex.sh", &temp);
        let supplement = copy_fixture("supplement_commands.json", &temp);

        let out_a = temp.join("out_a");
        let out_b = temp.join("out_b");
        fs::create_dir_all(&out_a).expect("create out_a");
        fs::create_dir_all(&out_b).expect("create out_b");

        let mut a = run_xtask_snapshot(&codex_bin, &out_a, &supplement);
        let mut b = run_xtask_snapshot(&codex_bin, &out_b, &supplement);

        a.as_object_mut().expect("snapshot is object").insert(
            "collected_at".to_string(),
            Value::String("1970-01-01T00:00:00Z".to_string()),
        );
        b.as_object_mut().expect("snapshot is object").insert(
            "collected_at".to_string(),
            Value::String("1970-01-01T00:00:00Z".to_string()),
        );

        assert_eq!(
            a, b,
            "snapshot output differs beyond collected_at when inputs are identical"
        );
    }

    #[test]
    fn c0_snapshot_infers_usage_args_and_parses_flags_across_blank_lines() {
        let temp = make_temp_dir("ccp-c0-test-usage-args");

        let codex_bin = copy_executable_fixture("fake_codex.sh", &temp);
        let supplement = copy_fixture("supplement_commands.json", &temp);

        let out_dir = temp.join("out");
        fs::create_dir_all(&out_dir).expect("create out dir");

        let snapshot = run_xtask_snapshot(&codex_bin, &out_dir, &supplement);

        let exec = get_command(&snapshot, &["exec"]);
        let exec_args = exec
            .get("args")
            .and_then(Value::as_array)
            .expect("exec.args is array");

        let arg_names = exec_args
            .iter()
            .filter_map(|a| a.get("name").and_then(Value::as_str))
            .collect::<Vec<_>>();
        assert!(
            arg_names.contains(&"PROMPT"),
            "exec args include PROMPT inferred from usage"
        );
        assert!(
            !arg_names.contains(&"COMMAND"),
            "exec args must not include COMMAND pseudo-arg when Commands: section is present"
        );

        let exec_flags = exec
            .get("flags")
            .and_then(Value::as_array)
            .expect("exec.flags is array");

        let beta = exec_flags
            .iter()
            .find(|f| f.get("long").and_then(Value::as_str) == Some("--beta"));
        assert!(
            beta.is_some(),
            "exec.flags includes --beta (after blank line)"
        );
        assert_eq!(
            beta.and_then(|f| f.get("short")).and_then(Value::as_str),
            None,
            "--beta is long-only and must not be mis-parsed as a short flag"
        );
        assert!(
            exec_flags
                .iter()
                .any(|f| f.get("long").and_then(Value::as_str) == Some("--alpha")),
            "exec.flags includes --alpha"
        );
    }

    #[test]
    fn c0_snapshot_records_feature_probe_and_feature_gated_commands() {
        let temp = make_temp_dir("ccp-c0-test-features");

        let codex_bin = copy_executable_fixture("fake_codex.sh", &temp);
        let supplement = copy_fixture("supplement_commands.json", &temp);

        let out_dir = temp.join("out");
        fs::create_dir_all(&out_dir).expect("create out dir");

        let snapshot = run_xtask_snapshot(&codex_bin, &out_dir, &supplement);

        let features = snapshot
            .get("features")
            .expect("snapshot.features exists when feature probe succeeds");
        let enabled = features
            .get("enabled_for_snapshot")
            .and_then(Value::as_array)
            .expect("features.enabled_for_snapshot is array");
        assert!(
            enabled.iter().any(|v| v.as_str() == Some("extra_feature")),
            "feature list includes extra_feature and snapshot enables it for discovery"
        );

        let added = features
            .get("commands_added_when_all_enabled")
            .and_then(Value::as_array)
            .expect("features.commands_added_when_all_enabled is array");
        assert!(
            added.iter().any(|p| {
                p.as_array()
                    .is_some_and(|a| a.iter().filter_map(Value::as_str).eq(["extra"]))
            }),
            "feature-gated command path [\"extra\"] is recorded as added when enabling features"
        );

        let commands = snapshot
            .get("commands")
            .and_then(Value::as_array)
            .expect("snapshot.commands is array");
        assert!(
            commands.iter().any(|c| {
                c.get("path")
                    .and_then(Value::as_array)
                    .is_some_and(|p| p.iter().filter_map(Value::as_str).eq(["extra"]))
            }),
            "feature-gated command [\"extra\"] appears in merged command list"
        );
    }
}
