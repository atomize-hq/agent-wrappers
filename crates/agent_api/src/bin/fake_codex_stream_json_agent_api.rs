use std::{
    collections::BTreeMap,
    env, fs,
    io::{self, Write},
    path::Path,
    time::Duration,
};

const NEWLINE: &[u8] = b"\r\n";

fn write_bytes(out: &mut impl Write, bytes: &[u8]) -> io::Result<()> {
    out.write_all(bytes)?;
    out.flush()?;
    Ok(())
}

fn emit_jsonl(out: &mut impl Write, line: &str) -> io::Result<()> {
    write_bytes(out, line.as_bytes())?;
    write_bytes(out, NEWLINE)?;
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
    emit_jsonl(out, &format!(r#"{{"type":"error","message":"{msg}"}}"#))?;
    Ok(false)
}

fn dump_env_to_path(path: &Path) -> io::Result<()> {
    let mut vars: BTreeMap<String, String> = BTreeMap::new();
    for (k, v) in env::vars() {
        vars.insert(k, v);
    }

    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }

    let mut buf = String::new();
    for (k, v) in vars {
        buf.push_str(&k);
        buf.push('=');
        buf.push_str(&v);
        buf.push('\n');
    }

    fs::write(path, buf.as_bytes())
}

fn main() -> io::Result<()> {
    // Cross-platform test binary used by `agent_api` tests.
    //
    // Emulates: `codex exec --json ...` by printing JSONL event lines.
    //
    // Scenario is selected via `FAKE_CODEX_SCENARIO`:
    // - live_two_events_long_delay
    // - emit_normalize_error_with_rawline_secret
    // - dump_env_then_exit
    //
    // The wrapper validates argv deterministically via env vars so tests can assert that the
    // universal backend pins non-interactive behavior and sandbox mode.
    let args: Vec<String> = env::args().collect();
    let mut out = io::stdout().lock();

    if !args.get(1).is_some_and(|arg| arg == "exec") {
        emit_jsonl(
            &mut out,
            r#"{"type":"error","message":"expected argv[1] to be \"exec\""}"#,
        )?;
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
            emit_jsonl(
                &mut out,
                r#"{"type":"error","message":"did not expect --ask-for-approval"}"#,
            )?;
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

    let scenario = env::var("FAKE_CODEX_SCENARIO")
        .unwrap_or_else(|_| "live_two_events_long_delay".to_string());
    match scenario.as_str() {
        "live_two_events_long_delay" => {
            emit_jsonl(
                &mut out,
                r#"{"type":"thread.started","thread_id":"thread-1"}"#,
            )?;
            emit_jsonl(
                &mut out,
                r#"{"type":"turn.started","thread_id":"thread-1","turn_id":"turn-1"}"#,
            )?;
            std::thread::sleep(Duration::from_millis(300));
        }
        "emit_normalize_error_with_rawline_secret" => {
            // Parses as JSON, but is missing required context so `codex` emits
            // `ExecStreamError::Normalize { line, message }`.
            emit_jsonl(
                &mut out,
                r#"{"type":"thread.started","secret":"RAWLINE_SECRET_DO_NOT_LEAK"}"#,
            )?;
        }
        "dump_env_then_exit" => {
            let path = match env::var("CODEX_WRAPPER_TEST_DUMP_ENV") {
                Ok(path) if !path.trim().is_empty() => path,
                _ => {
                    emit_jsonl(
                        &mut out,
                        r#"{"type":"error","message":"CODEX_WRAPPER_TEST_DUMP_ENV must be set"}"#,
                    )?;
                    std::process::exit(2);
                }
            };

            dump_env_to_path(Path::new(&path))?;
            emit_jsonl(
                &mut out,
                r#"{"type":"thread.started","thread_id":"thread-1"}"#,
            )?;
        }
        other => {
            emit_jsonl(
                &mut out,
                &format!(r#"{{"type":"error","message":"unknown FAKE_CODEX_SCENARIO: {other}"}}"#),
            )?;
            std::process::exit(2);
        }
    }

    Ok(())
}
