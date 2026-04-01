use std::{
    env,
    io::{self, Read, Write},
    sync::mpsc,
    time::Duration,
};

const ADD_DIRS_RUNTIME_REJECTION_MESSAGE: &str = "add_dirs rejected by runtime";
const ADD_DIR_RAW_PATH_SECRET: &str = "ADD_DIR_RAW_PATH_SECRET";
const ADD_DIR_STDOUT_SECRET: &str = "ADD_DIR_STDOUT_SECRET";
const ADD_DIR_STDERR_SECRET: &str = "ADD_DIR_STDERR_SECRET";

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

fn assert_current_dir(out: &mut impl Write) -> io::Result<bool> {
    let Ok(expected_cwd) = env::var("FAKE_CODEX_EXPECT_CWD") else {
        return Ok(true);
    };
    let got = env::current_dir()?;
    let expected = std::fs::canonicalize(&expected_cwd)
        .unwrap_or_else(|_| std::path::PathBuf::from(&expected_cwd));
    let got_canonical = std::fs::canonicalize(&got).unwrap_or(got.clone());
    if got_canonical == expected {
        return Ok(true);
    }

    let msg = format!(
        "expected cwd={}, got {}",
        expected.display(),
        got_canonical.display()
    );
    emit_jsonl(out, &format!(r#"{{"type":"error","message":"{msg}"}}"#))?;
    Ok(false)
}

fn assert_add_dirs(out: &mut impl Write, args: &[String]) -> io::Result<bool> {
    let Ok(expected_count_raw) = env::var("FAKE_CODEX_EXPECT_ADD_DIR_COUNT") else {
        return Ok(true);
    };
    let expected_count = match expected_count_raw.parse::<usize>() {
        Ok(value) => value,
        Err(err) => {
            emit_jsonl(
                out,
                &format!(
                    r#"{{"type":"error","message":"invalid FAKE_CODEX_EXPECT_ADD_DIR_COUNT: {err}"}}"#
                ),
            )?;
            return Ok(false);
        }
    };

    let mut actual = Vec::new();
    let mut idx = 0usize;
    while idx < args.len() {
        if args[idx] == "--add-dir" {
            let Some(value) = args.get(idx + 1) else {
                emit_jsonl(
                    out,
                    r#"{"type":"error","message":"--add-dir missing required value"}"#,
                )?;
                return Ok(false);
            };
            actual.push(value.clone());
            idx += 2;
            continue;
        }
        idx += 1;
    }

    if actual.len() != expected_count {
        emit_jsonl(
            out,
            &format!(
                r#"{{"type":"error","message":"expected {expected_count} --add-dir values, got {}"}}"#,
                actual.len()
            ),
        )?;
        return Ok(false);
    }

    for (index, got) in actual.iter().enumerate() {
        let key = format!("FAKE_CODEX_EXPECT_ADD_DIR_{index}");
        let expected = require_env_var(out, &key)?;
        if got != &expected {
            emit_jsonl(
                out,
                &format!(
                    r#"{{"type":"error","message":"expected add-dir[{index}]={expected:?}, got {got:?}"}}"#
                ),
            )?;
            return Ok(false);
        }
    }

    Ok(true)
}

fn assert_model(out: &mut impl Write, args: &[String]) -> io::Result<bool> {
    let Ok(expected_model) = env::var("FAKE_CODEX_EXPECT_MODEL") else {
        return Ok(true);
    };

    let positions: Vec<_> = args
        .iter()
        .enumerate()
        .filter_map(|(index, arg)| (arg == "--model").then_some(index))
        .collect();

    if expected_model == "<absent>" {
        if positions.is_empty() {
            return Ok(true);
        }
        emit_jsonl(
            out,
            &format!(
                r#"{{"type":"error","message":"did not expect --model flag, got {}"}}"#,
                positions.len()
            ),
        )?;
        return Ok(false);
    }

    if positions.len() != 1 {
        emit_jsonl(
            out,
            &format!(
                r#"{{"type":"error","message":"expected exactly one --model flag, got {}"}}"#,
                positions.len()
            ),
        )?;
        return Ok(false);
    }

    let model_index = positions[0];
    let Some(got_model) = args.get(model_index + 1) else {
        emit_jsonl(
            out,
            r#"{"type":"error","message":"--model missing required value"}"#,
        )?;
        return Ok(false);
    };

    if got_model != &expected_model {
        emit_jsonl(
            out,
            &format!(
                r#"{{"type":"error","message":"expected --model={expected_model:?}, got {got_model:?}"}}"#
            ),
        )?;
        return Ok(false);
    }

    if let Some(add_dir_index) = args.iter().position(|arg| arg == "--add-dir") {
        if model_index > add_dir_index {
            emit_jsonl(
                out,
                r#"{"type":"error","message":"expected --model before first --add-dir"}"#,
            )?;
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

fn emit_add_dirs_runtime_rejection(out: &mut impl Write) -> io::Result<()> {
    emit_jsonl(
        out,
        &format!(
            r#"{{"type":"error","message":"{ADD_DIRS_RUNTIME_REJECTION_MESSAGE}","code":"add_dirs_runtime_rejection","details":{{"raw_path":"{ADD_DIR_RAW_PATH_SECRET}","stdout":"{ADD_DIR_STDOUT_SECRET}","stderr":"{ADD_DIR_STDERR_SECRET}"}}}}"#
        ),
    )?;
    {
        let mut err = io::stderr().lock();
        writeln!(err, "{ADD_DIR_STDERR_SECRET}")?;
        err.flush()?;
    }
    Ok(())
}

fn runtime_rejection_exit_code() -> io::Result<i32> {
    match env::var("FAKE_CODEX_RUNTIME_REJECTION_EXIT_CODE") {
        Ok(raw) => raw.parse::<i32>().map_err(|err| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("invalid FAKE_CODEX_RUNTIME_REJECTION_EXIT_CODE: {err}"),
            )
        }),
        Err(env::VarError::NotPresent) => Ok(1),
        Err(err) => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("failed to read FAKE_CODEX_RUNTIME_REJECTION_EXIT_CODE: {err}"),
        )),
    }
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

    if args.get(1).is_some_and(|arg| arg == "--version") {
        write_line(&mut out, "codex 1.2.3\n")?;
        return Ok(());
    }

    if args.len() >= 3 && args[1] == "features" && args[2] == "list" {
        if args.get(3).is_some_and(|arg| arg == "--json") {
            write_line(&mut out, r#"{"features":["add_dir"]}"#)?;
            write_line(&mut out, "\n")?;
        } else {
            write_line(&mut out, "add_dir\n")?;
        }
        return Ok(());
    }

    if args.get(1).is_some_and(|arg| arg == "--help") {
        write_line(&mut out, "Usage: codex --add-dir\n")?;
        return Ok(());
    }

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
    if !assert_current_dir(&mut out)? {
        std::process::exit(1);
    }
    if !assert_add_dirs(&mut out, &args)? {
        std::process::exit(1);
    }
    if !assert_model(&mut out, &args)? {
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
        "add_dirs_runtime_rejection_resume_last" => {
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
            emit_add_dirs_runtime_rejection(&mut out)?;
            std::process::exit(1);
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
        "add_dirs_runtime_rejection_resume_id" => {
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
            emit_add_dirs_runtime_rejection(&mut out)?;
            std::process::exit(1);
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
        "add_dirs_runtime_rejection_exec" => {
            emit_jsonl(
                &mut out,
                r#"{"type":"thread.started","thread_id":"thread-1"}"#,
            )?;
            emit_add_dirs_runtime_rejection(&mut out)?;
            std::process::exit(runtime_rejection_exit_code()?);
        }
        "model_runtime_rejection_after_thread_started" => {
            let secret =
                require_env_var(&mut out, "FAKE_CODEX_MODEL_RUNTIME_REJECTION_SECRET")?;
            let model = flag_value(&args, "--model").unwrap_or("<missing>");

            emit_jsonl(
                &mut out,
                r#"{"type":"thread.started","thread_id":"thread-1"}"#,
            )?;
            emit_jsonl(
                &mut out,
                &format!(
                    r#"{{"type":"error","message":"unknown model: {model} ({secret})","code":"model_runtime_rejection"}}"#
                ),
            )?;
            std::process::exit(runtime_rejection_exit_code()?);
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
