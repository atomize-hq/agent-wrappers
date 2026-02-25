#[cfg(unix)]
mod unix {
    use std::{fs, process::Command, time::Duration};

    use claude_code::{ClaudeClient, ClaudeCodeError, ClaudeSetupTokenRequest};
    use tempfile::TempDir;
    use tokio::time;

    fn pid_exists(pid: i32) -> bool {
        Command::new("kill")
            .arg("-0")
            .arg(pid.to_string())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }

    async fn assert_pid_gone(pid: i32) {
        let deadline = time::Instant::now() + Duration::from_millis(500);
        loop {
            if !pid_exists(pid) {
                return;
            }

            if time::Instant::now() >= deadline {
                panic!("expected pid {pid} to be gone, but it still exists");
            }

            time::sleep(Duration::from_millis(25)).await;
        }
    }

    #[tokio::test]
    async fn setup_token_timeout_reaps_child() {
        use std::os::unix::fs::PermissionsExt;

        let dir = TempDir::new().expect("temp dir");
        let pid_file = dir.path().join("pid.txt");
        let script_path = dir.path().join("fake-claude");

        let script = r#"#!/bin/sh
set -eu
if [ "${1:-}" != "setup-token" ]; then
  echo "expected 'setup-token' arg, got: ${1:-<none>}" >&2
  exit 10
fi
: "${PID_FILE:?missing PID_FILE}"
echo $$ > "$PID_FILE"
exec sleep 1000000
"#;

        fs::write(&script_path, script).expect("write script");
        let mut perms = fs::metadata(&script_path).expect("metadata").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script_path, perms).expect("chmod");

        let client = ClaudeClient::builder()
            .binary(&script_path)
            .env("PID_FILE", pid_file.to_string_lossy().to_string())
            .build();

        let session = client
            .setup_token_start_with(
                ClaudeSetupTokenRequest::new().timeout(Some(Duration::from_millis(150))),
            )
            .await
            .expect("start");

        let err = session.wait().await.unwrap_err();
        assert!(
            matches!(err, ClaudeCodeError::Timeout { .. }),
            "expected timeout, got: {err:?}"
        );

        let pid: i32 = fs::read_to_string(&pid_file)
            .expect("pid file")
            .trim()
            .parse()
            .expect("pid parse");
        assert_pid_gone(pid).await;
    }
}
