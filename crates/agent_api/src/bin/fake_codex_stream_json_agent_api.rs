use std::{
    env, fs,
    io::{self, Write},
    path::{Path, PathBuf},
    time::Duration,
};

const STREAMING_JSONL: &str =
    include_str!("../../../codex/examples/fixtures/versioned/0.61.0/streaming.jsonl");

fn write_line(out: &mut impl Write, line: &str) -> io::Result<()> {
    out.write_all(line.as_bytes())?;
    out.flush()?;
    Ok(())
}

fn flag_value<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    args.iter()
        .position(|arg| arg == flag)
        .and_then(|idx| args.get(idx + 1))
        .map(String::as_str)
}

fn has_flag(args: &[String], flag: &str) -> bool {
    args.iter().any(|arg| arg == flag)
}

fn require_eq(
    out: &mut impl Write,
    name: &str,
    got: Option<&str>,
    expected: Option<&str>,
) -> io::Result<bool> {
    if got == expected {
        return Ok(true);
    }
    let msg = format!("expected {name}={expected:?}, got {got:?}");
    write_line(out, &format!(r#"{{"type":"error","message":"{msg}"}}"#))?;
    write_line(out, "\n")?;
    Ok(false)
}

fn emit_jsonl(out: &mut impl Write, line: &str) -> io::Result<()> {
    write_line(out, line)?;
    write_line(out, "\n")?;
    Ok(())
}

fn create_parent_dirs(path: &Path) -> io::Result<()> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    if parent.as_os_str().is_empty() {
        return Ok(());
    }
    fs::create_dir_all(parent)
}

fn main() -> io::Result<()> {
    // Cross-platform test binary used by `agent_api` tests.
    //
    // Emulates: `codex exec --json ...` by printing a small JSONL fixture set.
    //
    // The wrapper validates argv deterministically via env vars so tests can assert that the
    // universal backend pins non-interactive behavior and sandbox mode.
    let args: Vec<String> = env::args().collect();
    let mut out = io::stdout().lock();

    if !args.get(1).is_some_and(|arg| arg == "exec") {
        write_line(
            &mut out,
            r#"{"type":"error","message":"expected argv[1] to be \"exec\""}"#,
        )?;
        write_line(&mut out, "\n")?;
        std::process::exit(2);
    }

    let expected_sandbox =
        env::var("FAKE_CODEX_EXPECT_SANDBOX").unwrap_or_else(|_| "workspace-write".to_string());
    let expected_approval =
        env::var("FAKE_CODEX_EXPECT_APPROVAL").unwrap_or_else(|_| "never".to_string());

    let sandbox = flag_value(&args, "--sandbox");
    if !require_eq(
        &mut out,
        "--sandbox",
        sandbox,
        Some(expected_sandbox.as_str()),
    )? {
        std::process::exit(1);
    }

    if expected_approval == "<absent>" {
        if has_flag(&args, "--ask-for-approval") {
            write_line(
                &mut out,
                r#"{"type":"error","message":"did not expect --ask-for-approval"}"#,
            )?;
            write_line(&mut out, "\n")?;
            std::process::exit(1);
        }
    } else {
        let approval = flag_value(&args, "--ask-for-approval");
        if !require_eq(
            &mut out,
            "--ask-for-approval",
            approval,
            Some(expected_approval.as_str()),
        )? {
            std::process::exit(1);
        }
    }

    let scenario = env::var("FAKE_CODEX_SCENARIO").ok();
    match scenario.as_deref() {
        Some("live_two_events_long_delay") => {
            emit_jsonl(
                &mut out,
                r#"{"type":"thread.started","thread_id":"thread-1"}"#,
            )?;
            emit_jsonl(
                &mut out,
                r#"{"type":"turn.started","thread_id":"thread-1","turn_id":"turn-1"}"#,
            )?;
            std::thread::sleep(Duration::from_millis(750));
        }
        Some("emit_normalize_error_with_rawline_secret") => {
            emit_jsonl(
                &mut out,
                r#"{"type":"thread.started","secret":"RAWLINE_SECRET_DO_NOT_LEAK"}"#,
            )?;
            emit_jsonl(
                &mut out,
                r#"{"type":"thread.started","thread_id":"thread-1"}"#,
            )?;
        }
        Some("dump_env_then_exit") => {
            let dump_env = env::var("CODEX_WRAPPER_TEST_DUMP_ENV").map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "missing CODEX_WRAPPER_TEST_DUMP_ENV",
                )
            })?;
            let dump_path = PathBuf::from(dump_env);
            create_parent_dirs(&dump_path)?;

            let mut entries = env::vars()
                .filter(|(k, _)| k.starts_with("C2_"))
                .collect::<Vec<_>>();
            entries.sort_by(|(a, _), (b, _)| a.cmp(b));

            let mut content = String::new();
            for (key, value) in entries {
                content.push_str(&key);
                content.push('=');
                content.push_str(&value);
                content.push('\n');
            }

            fs::write(&dump_path, content.as_bytes())?;
            emit_jsonl(
                &mut out,
                r#"{"type":"thread.started","thread_id":"thread-1"}"#,
            )?;
        }
        Some(_) | None => {
            for line in STREAMING_JSONL.lines() {
                if line.trim().is_empty() {
                    continue;
                }
                emit_jsonl(&mut out, line)?;
            }
        }
    }

    Ok(())
}
