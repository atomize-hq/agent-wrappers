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
fn c8_spec_capability_matrix_matches_checked_in_file() {
    let xtask_bin = PathBuf::from(env!("CARGO_BIN_EXE_xtask"));
    let workdir = repo_root().join("crates").join("xtask");
    let temp_dir = make_temp_dir("capability-matrix-staleness");
    let generated = temp_dir.join("matrix.md");
    let checked_in = repo_root()
        .join("docs")
        .join("specs")
        .join("universal-agent-api")
        .join("capability-matrix.md");

    let output = Command::new(&xtask_bin)
        .arg("capability-matrix")
        .arg("--out")
        .arg(&generated)
        .current_dir(&workdir)
        .output()
        .expect("spawn xtask capability-matrix");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "capability-matrix generation failed\nstdout:\n{}\nstderr:\n{}",
        stdout,
        stderr
    );

    let generated_bytes = fs::read(&generated).expect("read generated capability matrix");
    let checked_in_bytes = fs::read(&checked_in).expect("read checked-in capability matrix");

    assert_eq!(
        generated_bytes,
        checked_in_bytes,
        "checked-in capability matrix is stale; regenerate with `cargo run -p xtask -- capability-matrix`"
    );
}
