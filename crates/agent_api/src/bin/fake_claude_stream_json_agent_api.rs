use std::{
    env,
    io::{self, Write},
    thread,
    time::Duration,
};

use serde_json::json;

const SYSTEM_INIT: &str =
    include_str!("../../../claude_code/tests/fixtures/stream_json/v1/system_init.jsonl");
const USER_MESSAGE: &str =
    include_str!("../../../claude_code/tests/fixtures/stream_json/v1/user_message.jsonl");
const RESULT_ERROR: &str =
    include_str!("../../../claude_code/tests/fixtures/stream_json/v1/result_error.jsonl");
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

fn main() -> io::Result<()> {
    // Cross-platform test binary used by `agent_api` tests.
    //
    // Emulates: `claude --print --output-format stream-json ...`
    // Scenario is selected via env var so tests can validate incrementality + gating.
    //
    // The universal agent wrapper contract defaults to non-interactive behavior; require that the
    // wrapper passes `--permission-mode bypassPermissions` so tests fail loudly if we regress.
    let args: Vec<String> = env::args().collect();
    let mut out = io::stdout().lock();
    if !has_flag_value(&args, "--permission-mode", "bypassPermissions") {
        fail(
            &mut out,
            "assertion failed: missing --permission-mode bypassPermissions",
        );
    }

    let scenario = env::var("FAKE_CLAUDE_SCENARIO").unwrap_or_else(|_| "two_events_delayed".into());

    let init = first_nonempty_line(SYSTEM_INIT);
    let user = first_nonempty_line(USER_MESSAGE);
    let result_error = first_nonempty_line(RESULT_ERROR);

    match scenario.as_str() {
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
            write_line(&mut out, &format!("{result_error}\n"))?;
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
            write_line(&mut out, &format!("{result_error}\n"))?;
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
        // Default: two events with a smaller delay (still long enough to demonstrate streaming).
        _ => {
            write_line(&mut out, &format!("{init}\n"))?;
            thread::sleep(Duration::from_millis(250));
            write_line(&mut out, &format!("{user}\n"))?;
        }
    }

    Ok(())
}
