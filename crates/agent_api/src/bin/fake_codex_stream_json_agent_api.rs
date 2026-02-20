use std::{
    env,
    io::{self, Write},
};

const STREAMING_JSONL: &str = include_str!("../../../codex/examples/fixtures/versioned/0.61.0/streaming.jsonl");

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

    let expected_sandbox = env::var("FAKE_CODEX_EXPECT_SANDBOX").unwrap_or_else(|_| "workspace-write".to_string());
    let expected_approval = env::var("FAKE_CODEX_EXPECT_APPROVAL").unwrap_or_else(|_| "never".to_string());

    let sandbox = flag_value(&args, "--sandbox");
    if !require_eq(&mut out, "--sandbox", sandbox, Some(expected_sandbox.as_str()))? {
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

    for line in STREAMING_JSONL.lines() {
        if line.trim().is_empty() {
            continue;
        }
        write_line(&mut out, line)?;
        write_line(&mut out, "\n")?;
    }

    Ok(())
}
