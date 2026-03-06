use std::{
    env,
    fs::{self, OpenOptions},
    io::{self, Write},
    thread,
    time::Duration,
};

use serde_json::json;

const SYSTEM_INIT: &str =
    include_str!("../../../claude_code/tests/fixtures/stream_json/v1/system_init.jsonl");
const USER_MESSAGE: &str =
    include_str!("../../../claude_code/tests/fixtures/stream_json/v1/user_message.jsonl");
const STREAM_EVENT_TOOL_USE_START: &str = include_str!(
    "../../../claude_code/tests/fixtures/stream_json/v1/stream_event_tool_use_start.jsonl"
);
const STREAM_EVENT_INPUT_JSON_DELTA: &str = include_str!(
    "../../../claude_code/tests/fixtures/stream_json/v1/stream_event_input_json_delta.jsonl"
);
const STREAM_EVENT_TOOL_RESULT_START: &str = include_str!(
    "../../../claude_code/tests/fixtures/stream_json/v1/stream_event_tool_result_start.jsonl"
);
const ASSISTANT_MESSAGE_TEXT: &str =
    include_str!("../../../claude_code/tests/fixtures/stream_json/v1/assistant_message_text.jsonl");

fn first_nonempty_line(text: &str) -> &str {
    text.lines()
        .find(|line| !line.chars().all(|ch| ch.is_whitespace()))
        .expect("fixture contains a non-empty line")
}

fn write_line(out: &mut impl Write, line: &str) -> io::Result<()> {
    out.write_all(line.as_bytes())?;
    out.flush()?;
    Ok(())
}

fn write_assistant_text(out: &mut impl Write, text: &str) -> io::Result<()> {
    let payload = json!({
        "type": "assistant",
        "session_id": "sess-1",
        "message": {
            "content": [{
                "type": "text",
                "text": text,
            }],
        },
    });
    write_line(out, &payload.to_string())?;
    write_line(out, "\n")?;
    Ok(())
}

fn write_result_error_with_message(out: &mut impl Write, message: &str) -> io::Result<()> {
    let payload = json!({
        "type": "result",
        "subtype": "error",
        "session_id": "sess-1",
        "is_error": true,
        "message": message,
    });
    write_line(out, &payload.to_string())?;
    write_line(out, "\n")?;
    Ok(())
}

fn has_flag_value(args: &[String], flag: &str, expected: &str) -> bool {
    args.iter()
        .position(|arg| arg == flag)
        .and_then(|idx| args.get(idx + 1))
        .is_some_and(|value| value == expected)
}

fn has_flag(args: &[String], flag: &str) -> bool {
    args.iter().any(|arg| arg == flag)
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

fn require(env_key: &str) -> String {
    env::var(env_key).unwrap_or_else(|_| panic!("missing required env var {env_key}"))
}

fn fail(mut out: impl Write, message: &str) -> ! {
    let _ = write_assistant_text(&mut out, message);
    std::process::exit(2);
}

fn env_is_true(key: &str) -> bool {
    matches!(env::var(key).as_deref(), Ok("1") | Ok("true") | Ok("TRUE"))
}

fn maybe_log_invocation(kind: &str) {
    let Ok(path) = env::var("FAKE_CLAUDE_INVOCATION_LOG") else {
        return;
    };

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .expect("open FAKE_CLAUDE_INVOCATION_LOG");
    writeln!(file, "{kind}").expect("append invocation log");
}

fn maybe_assert_flag_presence(args: &[String], env_key: &str, flag: &str, out: &mut dyn Write) {
    let Ok(raw) = env::var(env_key) else {
        return;
    };
    let expect_present = match raw.as_str() {
        "1" => true,
        "0" => false,
        other => panic!("{env_key} must be \"1\" or \"0\" (got {other:?})"),
    };

    let present = has_flag(args, flag);
    if expect_present != present {
        let expectation = if expect_present { "present" } else { "absent" };
        fail(
            out,
            &format!("assertion failed: expected {flag} to be {expectation}"),
        );
    }
}

fn maybe_write_env_snapshot() {
    let Ok(path) = env::var("FAKE_CLAUDE_ENV_SNAPSHOT_PATH") else {
        return;
    };

    let mut snapshot = String::new();
    for key in [
        "CLAUDE_HOME",
        "HOME",
        "XDG_CONFIG_HOME",
        "XDG_DATA_HOME",
        "XDG_CACHE_HOME",
    ] {
        let value = env::var(key).unwrap_or_default();
        snapshot.push_str(key);
        snapshot.push('=');
        snapshot.push_str(&value);
        snapshot.push('\n');
    }

    fs::write(path, snapshot).expect("write FAKE_CLAUDE_ENV_SNAPSHOT_PATH");
}

fn main() -> io::Result<()> {
    // Cross-platform test binary used by `agent_api` tests.
    //
    // Emulates: `claude --print --output-format stream-json ...`
    // Scenario is selected via env var so tests can validate incrementality + gating.
    //
    // The universal agent wrapper contract defaults to non-interactive behavior; require that the
    // wrapper passes `--permission-mode bypassPermissions` so tests fail loudly if we regress.
    let args: Vec<String> = env::args().collect();

    if has_flag(&args, "--help") {
        maybe_log_invocation("help");

        if env_is_true("FAKE_CLAUDE_HELP_FAIL") {
            let secret = env::var("FAKE_CLAUDE_HELP_FAIL_SECRET").unwrap_or_else(|_| {
                panic!("missing required env var FAKE_CLAUDE_HELP_FAIL_SECRET")
            });
            eprintln!("{secret}");
            std::process::exit(1);
        }

        let supports_allow_flag = env_is_true("FAKE_CLAUDE_HELP_SUPPORTS_ALLOW_FLAG");
        print!("Usage: claude [options]\n");
        if supports_allow_flag {
            print!("  --allow-dangerously-skip-permissions\n");
        }
        return Ok(());
    }

    let mut out = io::stdout().lock();
    if has_flag(&args, "--print") {
        maybe_log_invocation("print");

        if env_is_true("FAKE_CLAUDE_PRINT_SHOULD_NOT_RUN") {
            fail(&mut out, "assertion failed: --print should not be spawned");
        }

        if !has_flag_value(&args, "--permission-mode", "bypassPermissions") {
            fail(
                &mut out,
                "assertion failed: missing --permission-mode bypassPermissions",
            );
        }

        maybe_assert_flag_presence(
            &args,
            "FAKE_CLAUDE_EXPECT_DANGEROUS_SKIP_PERMISSIONS",
            "--dangerously-skip-permissions",
            &mut out,
        );
        maybe_assert_flag_presence(
            &args,
            "FAKE_CLAUDE_EXPECT_ALLOW_DANGEROUSLY_SKIP_PERMISSIONS",
            "--allow-dangerously-skip-permissions",
            &mut out,
        );
    }

    let scenario = env::var("FAKE_CLAUDE_SCENARIO").unwrap_or_else(|_| "two_events_delayed".into());

    let init = first_nonempty_line(SYSTEM_INIT);
    let user = first_nonempty_line(USER_MESSAGE);

    match scenario.as_str() {
        "fork_last_assert" => {
            let expected_prompt = require("FAKE_CLAUDE_EXPECT_PROMPT");
            if args.last() != Some(&expected_prompt) {
                fail(
                    &mut out,
                    "assertion failed: prompt must be final argv token",
                );
            }
            if !has_flag(&args, "--verbose") {
                fail(&mut out, "assertion failed: missing --verbose");
            }

            let ok = contains_ordered_subsequence(
                &args,
                &[
                    "--print",
                    "--output-format",
                    "stream-json",
                    "--permission-mode",
                    "bypassPermissions",
                    "--continue",
                    "--fork-session",
                    "--verbose",
                    &expected_prompt,
                ],
            );
            if !ok {
                fail(
                    &mut out,
                    "assertion failed: missing fork(last) argv subsequence",
                );
            }

            write_line(&mut out, &format!("{init}\n"))?;
        }
        "fork_id_assert" => {
            let expected_prompt = require("FAKE_CLAUDE_EXPECT_PROMPT");
            let expected_id = require("FAKE_CLAUDE_EXPECT_RESUME_ID");
            if args.last() != Some(&expected_prompt) {
                fail(
                    &mut out,
                    "assertion failed: prompt must be final argv token",
                );
            }
            if !has_flag(&args, "--verbose") {
                fail(&mut out, "assertion failed: missing --verbose");
            }

            let ok = contains_ordered_subsequence(
                &args,
                &[
                    "--print",
                    "--output-format",
                    "stream-json",
                    "--permission-mode",
                    "bypassPermissions",
                    "--fork-session",
                    "--resume",
                    &expected_id,
                    "--verbose",
                    &expected_prompt,
                ],
            );
            if !ok {
                fail(
                    &mut out,
                    "assertion failed: missing fork(id) argv subsequence",
                );
            }

            write_line(&mut out, &format!("{init}\n"))?;
        }
        "resume_last_assert" => {
            let expected_prompt = require("FAKE_CLAUDE_EXPECT_PROMPT");
            if args.last() != Some(&expected_prompt) {
                fail(
                    &mut out,
                    "assertion failed: prompt must be final argv token",
                );
            }
            if !has_flag(&args, "--verbose") {
                fail(&mut out, "assertion failed: missing --verbose");
            }

            let ok = contains_ordered_subsequence(
                &args,
                &[
                    "--print",
                    "--output-format",
                    "stream-json",
                    "--permission-mode",
                    "bypassPermissions",
                    "--continue",
                    "--verbose",
                    &expected_prompt,
                ],
            );
            if !ok {
                fail(
                    &mut out,
                    "assertion failed: missing resume(last) argv subsequence",
                );
            }

            write_line(&mut out, &format!("{init}\n"))?;
        }
        "resume_id_assert" => {
            let expected_prompt = require("FAKE_CLAUDE_EXPECT_PROMPT");
            let expected_id = require("FAKE_CLAUDE_EXPECT_RESUME_ID");
            if args.last() != Some(&expected_prompt) {
                fail(
                    &mut out,
                    "assertion failed: prompt must be final argv token",
                );
            }
            if !has_flag(&args, "--verbose") {
                fail(&mut out, "assertion failed: missing --verbose");
            }

            let ok = contains_ordered_subsequence(
                &args,
                &[
                    "--print",
                    "--output-format",
                    "stream-json",
                    "--permission-mode",
                    "bypassPermissions",
                    "--resume",
                    &expected_id,
                    "--verbose",
                    &expected_prompt,
                ],
            );
            if !ok {
                fail(
                    &mut out,
                    "assertion failed: missing resume(id) argv subsequence",
                );
            }

            write_line(&mut out, &format!("{init}\n"))?;
        }
        "fork_last_not_found" => {
            let expected_prompt = require("FAKE_CLAUDE_EXPECT_PROMPT");
            if args.last() != Some(&expected_prompt) {
                fail(
                    &mut out,
                    "assertion failed: prompt must be final argv token",
                );
            }
            if !has_flag(&args, "--verbose") {
                fail(&mut out, "assertion failed: missing --verbose");
            }
            if !contains_ordered_subsequence(&args, &["--continue", "--fork-session"]) {
                fail(
                    &mut out,
                    "assertion failed: missing --continue --fork-session",
                );
            }

            write_line(&mut out, &format!("{init}\n"))?;
            write_result_error_with_message(&mut out, "no session found")?;
            std::process::exit(1);
        }
        "fork_id_not_found" => {
            let expected_prompt = require("FAKE_CLAUDE_EXPECT_PROMPT");
            let expected_id = require("FAKE_CLAUDE_EXPECT_RESUME_ID");
            if args.last() != Some(&expected_prompt) {
                fail(
                    &mut out,
                    "assertion failed: prompt must be final argv token",
                );
            }
            if !has_flag(&args, "--verbose") {
                fail(&mut out, "assertion failed: missing --verbose");
            }
            if !contains_ordered_subsequence(&args, &["--fork-session", "--resume", &expected_id]) {
                fail(
                    &mut out,
                    "assertion failed: missing --fork-session --resume <ID>",
                );
            }

            write_line(&mut out, &format!("{init}\n"))?;
            write_result_error_with_message(&mut out, "session not found")?;
            std::process::exit(1);
        }
        "fork_last_generic_error" => {
            let expected_prompt = require("FAKE_CLAUDE_EXPECT_PROMPT");
            if args.last() != Some(&expected_prompt) {
                fail(
                    &mut out,
                    "assertion failed: prompt must be final argv token",
                );
            }
            if !has_flag(&args, "--verbose") {
                fail(&mut out, "assertion failed: missing --verbose");
            }
            if !contains_ordered_subsequence(&args, &["--continue", "--fork-session"]) {
                fail(
                    &mut out,
                    "assertion failed: missing --continue --fork-session",
                );
            }

            write_line(&mut out, &format!("{init}\n"))?;
            write_result_error_with_message(&mut out, "permission denied")?;
            std::process::exit(1);
        }
        "fork_id_generic_error" => {
            let expected_prompt = require("FAKE_CLAUDE_EXPECT_PROMPT");
            let expected_id = require("FAKE_CLAUDE_EXPECT_RESUME_ID");
            if args.last() != Some(&expected_prompt) {
                fail(
                    &mut out,
                    "assertion failed: prompt must be final argv token",
                );
            }
            if !has_flag(&args, "--verbose") {
                fail(&mut out, "assertion failed: missing --verbose");
            }
            if !contains_ordered_subsequence(&args, &["--fork-session", "--resume", &expected_id]) {
                fail(
                    &mut out,
                    "assertion failed: missing --fork-session --resume <ID>",
                );
            }

            write_line(&mut out, &format!("{init}\n"))?;
            write_result_error_with_message(&mut out, "permission denied")?;
            std::process::exit(1);
        }
        "resume_last_not_found" => {
            let expected_prompt = require("FAKE_CLAUDE_EXPECT_PROMPT");
            if args.last() != Some(&expected_prompt) {
                fail(
                    &mut out,
                    "assertion failed: prompt must be final argv token",
                );
            }
            if !has_flag(&args, "--verbose") {
                fail(&mut out, "assertion failed: missing --verbose");
            }
            if !contains_ordered_subsequence(&args, &["--continue"]) {
                fail(&mut out, "assertion failed: missing --continue");
            }

            write_line(&mut out, &format!("{init}\n"))?;
            write_result_error_with_message(&mut out, "no session found")?;
            std::process::exit(1);
        }
        "resume_id_not_found" => {
            let expected_prompt = require("FAKE_CLAUDE_EXPECT_PROMPT");
            let expected_id = require("FAKE_CLAUDE_EXPECT_RESUME_ID");
            if args.last() != Some(&expected_prompt) {
                fail(
                    &mut out,
                    "assertion failed: prompt must be final argv token",
                );
            }
            if !has_flag(&args, "--verbose") {
                fail(&mut out, "assertion failed: missing --verbose");
            }
            if !contains_ordered_subsequence(&args, &["--resume", &expected_id]) {
                fail(&mut out, "assertion failed: missing --resume <ID>");
            }

            write_line(&mut out, &format!("{init}\n"))?;
            write_result_error_with_message(&mut out, "session not found")?;
            std::process::exit(1);
        }
        "resume_last_generic_error" => {
            let expected_prompt = require("FAKE_CLAUDE_EXPECT_PROMPT");
            if args.last() != Some(&expected_prompt) {
                fail(
                    &mut out,
                    "assertion failed: prompt must be final argv token",
                );
            }
            if !has_flag(&args, "--verbose") {
                fail(&mut out, "assertion failed: missing --verbose");
            }
            if !contains_ordered_subsequence(&args, &["--continue"]) {
                fail(&mut out, "assertion failed: missing --continue");
            }

            write_line(&mut out, &format!("{init}\n"))?;
            write_result_error_with_message(&mut out, "permission denied")?;
            std::process::exit(1);
        }
        "resume_id_generic_error" => {
            let expected_prompt = require("FAKE_CLAUDE_EXPECT_PROMPT");
            let expected_id = require("FAKE_CLAUDE_EXPECT_RESUME_ID");
            if args.last() != Some(&expected_prompt) {
                fail(
                    &mut out,
                    "assertion failed: prompt must be final argv token",
                );
            }
            if !has_flag(&args, "--verbose") {
                fail(&mut out, "assertion failed: missing --verbose");
            }
            if !contains_ordered_subsequence(&args, &["--resume", &expected_id]) {
                fail(&mut out, "assertion failed: missing --resume <ID>");
            }

            write_line(&mut out, &format!("{init}\n"))?;
            write_result_error_with_message(&mut out, "permission denied")?;
            std::process::exit(1);
        }
        "resume_id_file_not_found_trap" => {
            let expected_prompt = require("FAKE_CLAUDE_EXPECT_PROMPT");
            let expected_id = require("FAKE_CLAUDE_EXPECT_RESUME_ID");
            if args.last() != Some(&expected_prompt) {
                fail(
                    &mut out,
                    "assertion failed: prompt must be final argv token",
                );
            }
            if !has_flag(&args, "--verbose") {
                fail(&mut out, "assertion failed: missing --verbose");
            }
            if !contains_ordered_subsequence(&args, &["--resume", &expected_id]) {
                fail(&mut out, "assertion failed: missing --resume <ID>");
            }

            write_line(&mut out, &format!("{init}\n"))?;
            write_result_error_with_message(&mut out, "file not found")?;
            std::process::exit(1);
        }
        "block_until_killed" => {
            write_line(&mut out, &format!("{init}\n"))?;
            loop {
                thread::sleep(Duration::from_secs(1));
            }
        }
        "single_event_then_exit" => {
            write_line(&mut out, &format!("{init}\n"))?;
        }
        // Prove "event observed before process exit" by delaying process exit well after the first
        // line is written and flushed.
        "two_events_long_delay" => {
            write_line(&mut out, &format!("{init}\n"))?;
            thread::sleep(Duration::from_millis(2000));
            write_line(&mut out, &format!("{user}\n"))?;
        }
        "final_text_and_tools" => {
            let tool_use_start = first_nonempty_line(STREAM_EVENT_TOOL_USE_START);
            let input_json_delta = first_nonempty_line(STREAM_EVENT_INPUT_JSON_DELTA);
            let tool_result_start = first_nonempty_line(STREAM_EVENT_TOOL_RESULT_START);
            let assistant_text = first_nonempty_line(ASSISTANT_MESSAGE_TEXT);

            write_line(&mut out, &format!("{init}\n"))?;
            write_line(&mut out, &format!("{tool_use_start}\n"))?;
            write_line(&mut out, &format!("{input_json_delta}\n"))?;
            write_line(&mut out, &format!("{tool_result_start}\n"))?;
            write_line(&mut out, &format!("{assistant_text}\n"))?;
        }
        "claude_home_env_snapshot" => {
            maybe_write_env_snapshot();
            let assistant_text = first_nonempty_line(ASSISTANT_MESSAGE_TEXT);

            write_line(&mut out, &format!("{init}\n"))?;
            write_line(&mut out, &format!("{assistant_text}\n"))?;
        }
        // Default: two events with a smaller delay (still long enough to demonstrate streaming).
        _ => {
            write_line(&mut out, &format!("{init}\n"))?;
            thread::sleep(Duration::from_millis(250));
            write_line(&mut out, &format!("{user}\n"))?;
        }
    }

    Ok(())
}
