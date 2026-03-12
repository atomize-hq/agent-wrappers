use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

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

#[test]
fn c8_spec_capability_matrix_succeeds_outside_repo_root() {
    let xtask_bin = PathBuf::from(env!("CARGO_BIN_EXE_xtask"));
    let workdir = repo_root().join("crates").join("xtask");
    let temp_dir = make_temp_dir("capability-matrix");
    let out = temp_dir.join("matrix.md");

    let output = Command::new(&xtask_bin)
        .arg("capability-matrix")
        .arg("--out")
        .arg(&out)
        .current_dir(&workdir)
        .output()
        .expect("spawn xtask capability-matrix");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "capability-matrix from {:?} failed\nstdout:\n{}\nstderr:\n{}",
        workdir,
        stdout,
        stderr
    );

    let markdown = fs::read_to_string(&out).expect("read generated capability matrix");
    assert!(
        markdown.contains("# Capability matrix"),
        "generated markdown missing title:\n{markdown}"
    );
}

#[test]
fn c8_spec_capability_matrix_audit_succeeds_outside_repo_root() {
    let xtask_bin = PathBuf::from(env!("CARGO_BIN_EXE_xtask"));
    let workdir = repo_root().join("crates").join("xtask");

    let output = Command::new(&xtask_bin)
        .arg("capability-matrix-audit")
        .current_dir(&workdir)
        .output()
        .expect("spawn xtask capability-matrix-audit");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "capability-matrix-audit from {:?} failed\nstdout:\n{}\nstderr:\n{}",
        workdir,
        stdout,
        stderr
    );
}
