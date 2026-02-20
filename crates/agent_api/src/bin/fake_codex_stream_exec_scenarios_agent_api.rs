use std::{
    env,
    io::{self, Write},
};

fn write_line(out: &mut impl Write, line: &str) -> io::Result<()> {
    out.write_all(line.as_bytes())?;
    out.flush()?;
    Ok(())
}

fn has_flag(args: &[String], flag: &str) -> bool {
    args.iter().any(|arg| arg == flag)
}

fn flag_value<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    args.iter()
        .position(|arg| arg == flag)
        .and_then(|idx| args.get(idx + 1))
        .map(String::as_str)
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

fn require_flag_present(out: &mut impl Write, args: &[String], flag: &str) -> io::Result<bool> {
    if has_flag(args, flag) {
        return Ok(true);
    }
    emit_jsonl(
        out,
        &format!(r#"{{"type":"error","message":"missing required flag: {flag}"}}"#),
    )?;
    Ok(false)
}

fn assert_env_overrides(out: &mut impl Write) -> io::Result<bool> {
    for (key, expected) in env::vars() {
        let Some(target) = key.strip_prefix("FAKE_CODEX_ASSERT_ENV_") else {
            continue;
        };
        let got = env::var(target).ok();
        if got.as_deref() != Some(expected.as_str()) {
            let msg = format!("expected env {target}={expected:?}, got {got:?}");
            emit_jsonl(out, &format!(r#"{{"type":"error","message":"{msg}"}}"#))?;
            return Ok(false);
        }
    }
    Ok(true)
}

fn main() -> io::Result<()> {
    // Cross-platform test binary used by `agent_api` tests.
    //
    // Emulates: `codex exec --json ...` by printing small JSONL sequences that trigger:
    // - per-line parse errors
    // - per-line normalize errors
    // - non-zero exits with stderr content
    // - env override assertions
    //
    // Scenario is selected via `FAKE_CODEX_SCENARIO`.
    let args: Vec<String> = env::args().collect();
    let mut out = io::stdout().lock();

    if !args.get(1).is_some_and(|arg| arg == "exec") {
        emit_jsonl(
            &mut out,
            r#"{"type":"error","message":"expected argv[1] to be \"exec\""}"#,
        )?;
        std::process::exit(2);
    }

    if !require_flag_present(&mut out, &args, "--json")? {
        std::process::exit(1);
    }
    if !require_flag_present(&mut out, &args, "--skip-git-repo-check")? {
        std::process::exit(1);
    }

    // Optional argv validation used by exec-policy tests.
    if let Ok(expected_sandbox) = env::var("FAKE_CODEX_EXPECT_SANDBOX") {
        let sandbox = flag_value(&args, "--sandbox");
        if !require_eq(
            &mut out,
            "--sandbox",
            sandbox,
            Some(expected_sandbox.as_str()),
        )? {
            std::process::exit(1);
        }
    }
    if let Ok(expected_approval) = env::var("FAKE_CODEX_EXPECT_APPROVAL") {
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
    }

    let scenario = env::var("FAKE_CODEX_SCENARIO").unwrap_or_else(|_| "ok".to_string());
    match scenario.as_str() {
        "env_assert" => {
            if !assert_env_overrides(&mut out)? {
                std::process::exit(1);
            }
            emit_jsonl(
                &mut out,
                r#"{"type":"thread.started","thread_id":"thread-1"}"#,
            )?;
        }
        "parse_error_midstream" => {
            emit_jsonl(
                &mut out,
                r#"{"type":"thread.started","thread_id":"thread-1"}"#,
            )?;
            write_line(&mut out, "THIS IS NOT JSON RAW-LINE-SECRET-PARSE\n")?;
            emit_jsonl(
                &mut out,
                r#"{"type":"turn.started","thread_id":"thread-1","turn_id":"turn-1"}"#,
            )?;
        }
        "normalize_error_midstream" => {
            emit_jsonl(
                &mut out,
                r#"{"type":"thread.started","secret":"RAW-LINE-SECRET-NORM"}"#,
            )?;
            emit_jsonl(
                &mut out,
                r#"{"type":"thread.started","thread_id":"thread-1"}"#,
            )?;
        }
        "nonzero_exit" => {
            emit_jsonl(
                &mut out,
                r#"{"type":"thread.started","thread_id":"thread-1"}"#,
            )?;
            eprintln!("RAW-STDERR-SECRET");
            std::process::exit(3);
        }
        _ => {
            emit_jsonl(
                &mut out,
                r#"{"type":"thread.started","thread_id":"thread-1"}"#,
            )?;
        }
    }

    Ok(())
}
