use std::{
    env,
    io::{self, Read, Write},
    sync::mpsc,
    time::Duration,
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

fn contains_ordered_subsequence(args: &[String], subseq: &[&str]) -> bool {
    if subseq.is_empty() {
        return true;
    }

    let mut idx = 0usize;
    for arg in args {
        if arg == subseq[idx] {
            idx += 1;
            if idx == subseq.len() {
                return true;
            }
        }
    }
    false
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

fn require_env_var(out: &mut impl Write, key: &str) -> io::Result<String> {
    match env::var(key) {
        Ok(value) if !value.trim().is_empty() => Ok(value),
        _ => {
            emit_jsonl(
                out,
                &format!(r#"{{"type":"error","message":"missing required env var: {key}"}}"#),
            )?;
            std::process::exit(2);
        }
    }
}

fn read_stdin_to_end_with_timeout(timeout: Duration) -> io::Result<Vec<u8>> {
    let (tx, rx) = mpsc::channel::<io::Result<Vec<u8>>>();
    std::thread::spawn(move || {
        let mut buf = Vec::new();
        let result = io::stdin().read_to_end(&mut buf).map(|_| buf);
        let _ = tx.send(result);
    });

    match rx.recv_timeout(timeout) {
        Ok(result) => result,
        Err(mpsc::RecvTimeoutError::Timeout) => Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "stdin read timed out (stdin may not have been closed)",
        )),
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            Err(io::Error::other("stdin reader thread disconnected"))
        }
    }
}

fn assert_stdin_prompt(
    out: &mut impl Write,
    expected_prompt: &str,
    timeout: Duration,
) -> io::Result<bool> {
    let stdin = match read_stdin_to_end_with_timeout(timeout) {
        Ok(stdin) => stdin,
        Err(err) => {
            emit_jsonl(
                out,
                &format!(r#"{{"type":"error","message":"failed to read stdin: {err}"}}"#),
            )?;
            return Ok(false);
        }
    };

    let mut expected = expected_prompt.as_bytes().to_vec();
    expected.push(b'\n');

    if stdin == expected {
        return Ok(true);
    }

    emit_jsonl(
        out,
        &format!(
            r#"{{"type":"error","message":"stdin prompt mismatch: expected {} bytes, got {} bytes"}}"#,
            expected.len(),
            stdin.len()
        ),
    )?;
    Ok(false)
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

    // Optional argv validation used by external-sandbox exec-policy conformance tests.
    if let Ok(expect_bypass) = env::var("FAKE_CODEX_EXPECT_DANGEROUS_BYPASS") {
        if !expect_bypass.trim().is_empty() {
            const BYPASS_FLAG: &str = "--dangerously-bypass-approvals-and-sandbox";
            let bypass_count = args.iter().filter(|arg| *arg == BYPASS_FLAG).count();
            if bypass_count != 1 {
                emit_jsonl(
                    &mut out,
                    &format!(
                        r#"{{"type":"error","message":"expected {BYPASS_FLAG} exactly once, got {bypass_count}"}}"#
                    ),
                )?;
                std::process::exit(1);
            }

            for forbidden in ["--full-auto", "--ask-for-approval", "--sandbox"] {
                if has_flag(&args, forbidden) {
                    emit_jsonl(
                        &mut out,
                        &format!(
                            r#"{{"type":"error","message":"did not expect forbidden flag: {forbidden}"}}"#
                        ),
                    )?;
                    std::process::exit(1);
                }
            }
        }
    }

    let scenario = env::var("FAKE_CODEX_SCENARIO").unwrap_or_else(|_| "ok".to_string());
    match scenario.as_str() {
        "resume_last_assert" => {
            let expected_prompt = require_env_var(&mut out, "FAKE_CODEX_EXPECT_PROMPT")?;

            let ok =
                contains_ordered_subsequence(&args, &["exec", "--json", "resume", "--last", "-"]);
            if !ok {
                emit_jsonl(
                    &mut out,
                    r#"{"type":"error","message":"missing argv subsequence: exec --json resume --last -"}"#,
                )?;
                std::process::exit(1);
            }

            if !assert_stdin_prompt(&mut out, &expected_prompt, Duration::from_secs(1))? {
                std::process::exit(1);
            }

            emit_jsonl(
                &mut out,
                r#"{"type":"thread.resumed","thread_id":"thread-1"}"#,
            )?;
        }
        "resume_id_assert" => {
            let expected_prompt = require_env_var(&mut out, "FAKE_CODEX_EXPECT_PROMPT")?;
            let expected_id = require_env_var(&mut out, "FAKE_CODEX_EXPECT_RESUME_ID")?;

            let ok = contains_ordered_subsequence(
                &args,
                &["exec", "--json", "resume", expected_id.as_str(), "-"],
            );
            if !ok {
                emit_jsonl(
                    &mut out,
                    r#"{"type":"error","message":"missing argv subsequence: exec --json resume <ID> -"}"#,
                )?;
                std::process::exit(1);
            }

            if !assert_stdin_prompt(&mut out, &expected_prompt, Duration::from_secs(1))? {
                std::process::exit(1);
            }

            emit_jsonl(
                &mut out,
                r#"{"type":"thread.resumed","thread_id":"thread-1"}"#,
            )?;
        }
        "resume_last_not_found" => {
            let expected_prompt = require_env_var(&mut out, "FAKE_CODEX_EXPECT_PROMPT")?;

            let ok =
                contains_ordered_subsequence(&args, &["exec", "--json", "resume", "--last", "-"]);
            if !ok {
                emit_jsonl(
                    &mut out,
                    r#"{"type":"error","message":"missing argv subsequence: exec --json resume --last -"}"#,
                )?;
                std::process::exit(1);
            }

            if !assert_stdin_prompt(&mut out, &expected_prompt, Duration::from_secs(1))? {
                std::process::exit(1);
            }

            eprintln!("no session found");
            std::process::exit(1);
        }
        "resume_id_not_found" => {
            let expected_prompt = require_env_var(&mut out, "FAKE_CODEX_EXPECT_PROMPT")?;
            let expected_id = require_env_var(&mut out, "FAKE_CODEX_EXPECT_RESUME_ID")?;

            let ok = contains_ordered_subsequence(
                &args,
                &["exec", "--json", "resume", expected_id.as_str(), "-"],
            );
            if !ok {
                emit_jsonl(
                    &mut out,
                    r#"{"type":"error","message":"missing argv subsequence: exec --json resume <ID> -"}"#,
                )?;
                std::process::exit(1);
            }

            if !assert_stdin_prompt(&mut out, &expected_prompt, Duration::from_secs(1))? {
                std::process::exit(1);
            }

            eprintln!("session not found");
            std::process::exit(1);
        }
        // Stable scenario name used by SEAM-4 explicit cancellation integration tests.
        "block_until_killed" => {
            emit_jsonl(
                &mut out,
                r#"{"type":"thread.started","thread_id":"thread-1"}"#,
            )?;
            {
                let mut err = io::stderr().lock();
                writeln!(err, "RAW-STDERR-SECRET-CANCEL")?;
                err.flush()?;
            }

            loop {
                std::thread::sleep(Duration::from_secs(1));
            }
        }
        // Stable scenario name used by SEAM-4 drop receiver regression integration tests.
        "many_events_then_exit" => {
            const MANY_EVENTS_N: usize = 200;
            let padding = "x".repeat(1024);

            emit_jsonl(
                &mut out,
                r#"{"type":"thread.started","thread_id":"thread-1"}"#,
            )?;
            for idx in 0..MANY_EVENTS_N {
                emit_jsonl(
                    &mut out,
                    &format!(
                        r#"{{"type":"turn.started","thread_id":"thread-1","turn_id":"turn-{idx}","padding":"{padding}"}}"#
                    ),
                )?;
            }
        }
        "env_assert" => {
            if !assert_env_overrides(&mut out)? {
                std::process::exit(1);
            }
            emit_jsonl(
                &mut out,
                r#"{"type":"thread.started","thread_id":"thread-1"}"#,
            )?;
        }
        "tool_lifecycle_ok" => {
            emit_jsonl(
                &mut out,
                r#"{"type":"thread.started","thread_id":"thread-1"}"#,
            )?;
            emit_jsonl(
                &mut out,
                r#"{"type":"turn.started","thread_id":"thread-1","turn_id":"turn-1"}"#,
            )?;
            emit_jsonl(
                &mut out,
                r#"{"type":"item.started","thread_id":"thread-1","turn_id":"turn-1","item_id":"item-tool-1","item_type":"command_execution","content":{"command":"echo hi"}}"#,
            )?;
            // Sentinels appear only in tool output fields (stdout/stderr) so leak assertions are meaningful.
            emit_jsonl(
                &mut out,
                r#"{"type":"item.delta","thread_id":"thread-1","turn_id":"turn-1","item_id":"item-tool-1","item_type":"command_execution","delta":{"stdout":"STDOUT-SENTINEL-DO-NOT-LEAK","stderr":"STDERR-SENTINEL-DO-NOT-LEAK"}}"#,
            )?;
            emit_jsonl(
                &mut out,
                r#"{"type":"item.completed","thread_id":"thread-1","turn_id":"turn-1","item_id":"item-tool-1","item_type":"command_execution","content":{"command":"echo hi","stdout":"ok","stderr":"warn","exit_code":0}}"#,
            )?;
            emit_jsonl(
                &mut out,
                r#"{"type":"turn.completed","thread_id":"thread-1","turn_id":"turn-1"}"#,
            )?;
        }
        "tool_lifecycle_fail_unknown_type" => {
            emit_jsonl(
                &mut out,
                r#"{"type":"thread.started","thread_id":"thread-1"}"#,
            )?;
            emit_jsonl(
                &mut out,
                r#"{"type":"turn.started","thread_id":"thread-1","turn_id":"turn-1"}"#,
            )?;
            // No top-level item_type, so attribution is not deterministic and should map to Error.
            emit_jsonl(
                &mut out,
                r#"{"type":"item.failed","thread_id":"thread-1","turn_id":"turn-1","item_id":"item-tool-1","error":{"message":"benign failure"}}"#,
            )?;
            emit_jsonl(
                &mut out,
                r#"{"type":"turn.completed","thread_id":"thread-1","turn_id":"turn-1"}"#,
            )?;
        }
        "tool_lifecycle_fail_known_type" => {
            emit_jsonl(
                &mut out,
                r#"{"type":"thread.started","thread_id":"thread-1"}"#,
            )?;
            emit_jsonl(
                &mut out,
                r#"{"type":"turn.started","thread_id":"thread-1","turn_id":"turn-1"}"#,
            )?;
            // IMPORTANT: item_type must be top-level (not nested under an "extra" object) so it
            // lands in ItemFailure.extra["item_type"].
            emit_jsonl(
                &mut out,
                r#"{"type":"item.failed","thread_id":"thread-1","turn_id":"turn-1","item_id":"item-tool-1","item_type":"command_execution","error":{"message":"benign failure"}}"#,
            )?;
            emit_jsonl(
                &mut out,
                r#"{"type":"turn.completed","thread_id":"thread-1","turn_id":"turn-1"}"#,
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
