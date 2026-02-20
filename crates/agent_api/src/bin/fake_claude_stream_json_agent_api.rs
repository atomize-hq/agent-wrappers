use std::{
    env,
    io::{self, Write},
    thread,
    time::Duration,
};

const SYSTEM_INIT: &str =
    include_str!("../../../claude_code/tests/fixtures/stream_json/v1/system_init.jsonl");
const USER_MESSAGE: &str =
    include_str!("../../../claude_code/tests/fixtures/stream_json/v1/user_message.jsonl");

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

fn has_flag_value(args: &[String], flag: &str, expected: &str) -> bool {
    args.iter()
        .position(|arg| arg == flag)
        .and_then(|idx| args.get(idx + 1))
        .is_some_and(|value| value == expected)
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
    if !has_flag_value(&args, "--permission-mode", "bypassPermissions") {
        let mut out = io::stdout().lock();
        write_line(
            &mut out,
            r#"{"type":"result","subtype":"error","error":{"type":"invalid_request_error","message":"missing --permission-mode bypassPermissions"}}"#,
        )?;
        write_line(&mut out, "\n")?;
        std::process::exit(1);
    }

    let scenario = env::var("FAKE_CLAUDE_SCENARIO").unwrap_or_else(|_| "two_events_delayed".into());

    let init = first_nonempty_line(SYSTEM_INIT);
    let user = first_nonempty_line(USER_MESSAGE);

    let mut out = io::stdout().lock();

    match scenario.as_str() {
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
        // Default: two events with a smaller delay (still long enough to demonstrate streaming).
        _ => {
            write_line(&mut out, &format!("{init}\n"))?;
            thread::sleep(Duration::from_millis(250));
            write_line(&mut out, &format!("{user}\n"))?;
        }
    }

    Ok(())
}
