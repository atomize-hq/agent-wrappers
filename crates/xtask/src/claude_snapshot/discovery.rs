use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    process::{Command, Stdio},
    time::Duration,
};

use super::{layout, supplements, util, ArgSnapshot, CommandSnapshot, Error, FlagSnapshot};

pub(super) struct DiscoveryOutput {
    pub(super) commands: BTreeMap<Vec<String>, CommandSnapshot>,
    pub(super) known_omissions: Vec<String>,
}

pub(super) fn discover_commands(
    claude_binary: &Path,
    raw_help_dir: Option<&Path>,
    capture_raw_help: bool,
    help_timeout_ms: u64,
) -> Result<DiscoveryOutput, Error> {
    let mut out = BTreeMap::<Vec<String>, CommandSnapshot>::new();
    let mut visited = BTreeSet::<Vec<String>>::new();
    let mut known_omissions: Vec<String> = Vec::new();
    let help_timeout = Duration::from_millis(help_timeout_ms);

    let root_help = run_claude_help_strict(claude_binary, &[], help_timeout)?;
    let root_parsed = parse_help(&root_help);

    if capture_raw_help {
        if let Some(dir) = raw_help_dir {
            layout::write_raw_help(dir, &[], &root_help)?;
        }
    }

    let mut root_args = root_parsed.args;
    if let Some(usage) = root_parsed.usage.as_deref() {
        merge_inferred_args(
            &mut root_args,
            infer_args_from_usage(usage, &[], !root_parsed.subcommands.is_empty()),
        );
    }
    if !root_args.is_empty() {
        root_args.sort_by(|a, b| a.name.cmp(&b.name));
    }

    let mut root_flags = root_parsed.flags;
    if !root_flags.is_empty() {
        root_flags.sort_by(supplements::flag_sort_key);
    }

    out.insert(
        Vec::new(),
        CommandSnapshot {
            path: Vec::new(),
            about: root_parsed.about,
            usage: root_parsed.usage,
            stability: None,
            platforms: None,
            args: if root_args.is_empty() {
                None
            } else {
                Some(root_args)
            },
            flags: if root_flags.is_empty() {
                None
            } else {
                Some(root_flags)
            },
        },
    );

    let ctx = HelpCtx {
        claude_binary,
        raw_help_dir,
        capture_raw_help,
        help_timeout,
    };

    // Use an explicit stack to avoid deep recursion (which can stack-overflow on Windows if the
    // CLI presents a deeply nested or accidentally self-referential command tree).
    let mut stack: Vec<Vec<String>> = root_parsed
        .subcommands
        .into_iter()
        .map(|t| vec![t])
        .collect();

    while let Some(path) = stack.pop() {
        if !visited.insert(path.clone()) {
            continue;
        }

        match run_claude_help_tolerant(ctx.claude_binary, &path, ctx.help_timeout) {
            Ok(help) => {
                let parsed = parse_help(&help);

                if ctx.capture_raw_help {
                    if let Some(dir) = ctx.raw_help_dir {
                        layout::write_raw_help(dir, &path, &help)?;
                    }
                }

                let mut args = parsed.args;
                if let Some(usage) = parsed.usage.as_deref() {
                    merge_inferred_args(
                        &mut args,
                        infer_args_from_usage(usage, &path, !parsed.subcommands.is_empty()),
                    );
                }
                if !args.is_empty() {
                    args.sort_by(|a, b| a.name.cmp(&b.name));
                }

                let mut flags = parsed.flags;
                if !flags.is_empty() {
                    flags.sort_by(supplements::flag_sort_key);
                }

                out.insert(
                    path.clone(),
                    CommandSnapshot {
                        path: path.clone(),
                        about: parsed.about,
                        usage: parsed.usage,
                        stability: None,
                        platforms: None,
                        args: if args.is_empty() { None } else { Some(args) },
                        flags: if flags.is_empty() { None } else { Some(flags) },
                    },
                );

                for sub in parsed.subcommands {
                    let mut next = path.clone();
                    next.push(sub);
                    stack.push(next);
                }
            }
            Err(note) => {
                // Keep the snapshot deterministic and progressing even if a specific `--help`
                // probe fails (e.g., requires auth, tries to do first-run setup, or exits 1).
                known_omissions.push(note);
                out.insert(
                    path.clone(),
                    CommandSnapshot {
                        path,
                        about: None,
                        usage: None,
                        stability: None,
                        platforms: None,
                        args: None,
                        flags: None,
                    },
                );
            }
        }
    }

    if !known_omissions.is_empty() {
        // Stable ordering for diff friendliness.
        known_omissions.sort();
    }

    Ok(DiscoveryOutput {
        commands: out,
        known_omissions,
    })
}

struct HelpCtx<'a> {
    claude_binary: &'a Path,
    raw_help_dir: Option<&'a Path>,
    capture_raw_help: bool,
    help_timeout: Duration,
}

fn run_claude_help_strict(
    claude_binary: &Path,
    path: &[String],
    timeout: Duration,
) -> Result<String, Error> {
    let help =
        run_claude_help_with_timeout(claude_binary, path, timeout).map_err(Error::CommandFailed)?;
    if !help.status.success() {
        return Err(Error::CommandFailed(help.failure_debug));
    }
    Ok(help.text)
}

fn run_claude_help_tolerant(
    claude_binary: &Path,
    path: &[String],
    timeout: Duration,
) -> Result<String, String> {
    let help = run_claude_help_with_timeout(claude_binary, path, timeout)?;

    if help.status.success() {
        return Ok(help.text);
    }

    // Some CLIs exit non-zero for `--help` while still printing a full usage block.
    // Prefer keeping snapshot generation moving, but record a stable omission note.
    if looks_like_help(&help.text) {
        return Ok(help.text);
    }

    Err(help.failure_note)
}

struct HelpRun {
    status: std::process::ExitStatus,
    text: String,
    failure_note: String,
    failure_debug: String,
}

fn run_claude_help_with_timeout(
    claude_binary: &Path,
    path: &[String],
    timeout: Duration,
) -> Result<HelpRun, String> {
    let mut cmd = Command::new(claude_binary);
    cmd.args(path);
    cmd.arg("--help");
    cmd.env("NO_COLOR", "1");
    cmd.env("CLICOLOR", "0");
    cmd.env("TERM", "dumb");
    cmd.env("DISABLE_AUTOUPDATER", "1");
    cmd.env("CI", "1");
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| e.to_string())?;

    // Avoid indefinite hangs on any single `--help` invocation.
    let _status = {
        use wait_timeout::ChildExt;
        match child.wait_timeout(timeout).map_err(|e| e.to_string())? {
            Some(status) => status,
            None => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "timeout after {}ms: {}",
                    timeout.as_millis(),
                    util::command_string(&cmd)
                ));
            }
        }
    };

    let output = child.wait_with_output().map_err(|e| e.to_string())?;
    let text = util::normalize_text(&output.stdout, &output.stderr);
    let cmd_string = util::command_string(&cmd);
    let exit_code = output.status.code().unwrap_or(-1);
    let failure_note = format!("help probe failed: {cmd_string} (exit {exit_code})");
    let failure_debug = util::command_failed_message(&cmd, &output);

    Ok(HelpRun {
        status: output.status,
        text,
        failure_note,
        failure_debug,
    })
}

#[derive(Debug)]
struct ParsedHelp {
    about: Option<String>,
    usage: Option<String>,
    subcommands: Vec<String>,
    flags: Vec<FlagSnapshot>,
    args: Vec<ArgSnapshot>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Section {
    Commands,
    Options,
    Args,
}

fn parse_help(help: &str) -> ParsedHelp {
    let lines: Vec<&str> = help.lines().collect();

    let mut usage: Option<String> = None;
    let mut usage_lines: Vec<String> = Vec::new();
    let mut usage_started = false;
    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.to_ascii_lowercase().starts_with("usage:") {
            usage_started = true;
            let rest = trimmed["usage:".len()..].trim();
            if !rest.is_empty() {
                usage_lines.push(rest.to_string());
            }
            for cont in lines.iter().skip(idx + 1) {
                if cont.trim().is_empty() {
                    break;
                }
                if cont.starts_with(' ') || cont.starts_with('\t') {
                    let t = cont.trim();
                    if t.ends_with(':') && is_section_header(t) {
                        break;
                    }
                    usage_lines.push(t.to_string());
                } else {
                    break;
                }
            }
            break;
        }
    }
    if usage_started && !usage_lines.is_empty() {
        usage = Some(usage_lines.join("\n"));
    }

    let about = {
        let mut nonempty_indices = lines
            .iter()
            .enumerate()
            .filter_map(|(i, l)| if l.trim().is_empty() { None } else { Some(i) })
            .collect::<Vec<_>>();
        if nonempty_indices.is_empty() {
            None
        } else {
            let title_idx = nonempty_indices.remove(0);
            let usage_idx = lines
                .iter()
                .position(|l| l.trim_start().to_ascii_lowercase().starts_with("usage:"))
                .unwrap_or(lines.len());
            let mut about_lines = Vec::new();
            for l in lines.iter().take(usage_idx).skip(title_idx + 1) {
                let t = l.trim();
                if t.is_empty() {
                    continue;
                }
                about_lines.push(t.to_string());
            }
            if about_lines.is_empty() {
                None
            } else {
                Some(about_lines.join("\n"))
            }
        }
    };

    let mut subcommands = Vec::new();
    let mut flags = Vec::new();
    let mut args = Vec::new();

    let mut section: Option<Section> = None;

    for line in lines {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }

        if let Some(s) = parse_section_header(t) {
            section = Some(s);
            continue;
        }

        match section {
            Some(Section::Commands) => {
                if let Some(token) = parse_command_token(line) {
                    subcommands.push(token);
                }
            }
            Some(Section::Options) => {
                if let Some(flag) = parse_flag_line(line) {
                    flags.push(flag);
                }
            }
            Some(Section::Args) => {
                if let Some(arg) = parse_arg_line(line) {
                    args.push(arg);
                } else if let Some(last) = args.last_mut() {
                    // Clap frequently wraps argument descriptions onto continuation lines with
                    // deeper indentation. Preserve these as part of the argument note so snapshots
                    // capture positional semantics with useful context.
                    let cont = line.trim();
                    if !cont.is_empty() {
                        match last.note.as_mut() {
                            Some(note) => {
                                note.push('\n');
                                note.push_str(cont);
                            }
                            None => last.note = Some(cont.to_string()),
                        }
                    }
                }
            }
            None => {}
        }
    }

    ParsedHelp {
        about,
        usage,
        subcommands,
        flags,
        args,
    }
}

fn looks_like_help(s: &str) -> bool {
    // Heuristic: accept common help markers even when exit status is non-zero.
    let lower = s.to_ascii_lowercase();
    lower.contains("usage:")
        || lower.contains("commands:")
        || lower.contains("subcommands:")
        || lower.contains("options:")
        || lower.contains("flags:")
}

fn is_section_header(t: &str) -> bool {
    matches!(
        t.trim_end_matches(':').to_ascii_lowercase().as_str(),
        "commands" | "subcommands" | "options" | "flags" | "arguments"
    )
}

fn parse_section_header(t: &str) -> Option<Section> {
    let header = t.trim_end_matches(':').to_ascii_lowercase();
    match header.as_str() {
        "commands" | "subcommands" => Some(Section::Commands),
        "options" | "flags" => Some(Section::Options),
        "arguments" => Some(Section::Args),
        _ => None,
    }
}

fn parse_command_token(line: &str) -> Option<String> {
    if !line.starts_with(' ') && !line.starts_with('\t') {
        return None;
    }
    let trimmed = line.trim_start();
    if trimmed.starts_with('-') {
        return None;
    }
    // Only treat "command list" entries as subcommands when the help output includes an
    // on-the-same-line description. Wrapped descriptions (continuation lines) should not be
    // interpreted as additional command tokens.
    let (head, desc) = split_tokens_and_desc(trimmed);
    // Clap's commands list format is typically: `<token>  <desc>`. If the "head" contains
    // whitespace, it's usually a usage/example line like `claude <cmd> --help`, not a subcommand.
    // Reject it to prevent runaway self-referential trees like `claude claude claude ...`.
    if head.split_whitespace().count() != 1 {
        return None;
    }
    let token = head;
    let token = token.split('|').next().unwrap_or(token);
    // Claude Code help often includes a meta `help` subcommand (e.g. `claude mcp help [command]`)
    // that does not behave like a real leaf command under `--help` crawling and may exit non-zero.
    // Skip it to keep snapshot generation deterministic.
    if token == "help" {
        return None;
    }
    if token == "claude" {
        return None;
    }
    if desc.is_empty() && head.trim() != token {
        return None;
    }
    if token
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        Some(token.to_string())
    } else {
        None
    }
}

fn parse_flag_line(line: &str) -> Option<FlagSnapshot> {
    if !line.starts_with(' ') && !line.starts_with('\t') {
        return None;
    }
    let trimmed = line.trim_start();
    if !trimmed.starts_with('-') {
        return None;
    }

    let (tokens_part, _) = split_tokens_and_desc(trimmed);
    let mut long: Option<String> = None;
    let mut short: Option<String> = None;
    let mut value_name: Option<String> = None;
    let mut repeatable: Option<bool> = None;

    for tok in tokens_part.split_whitespace() {
        let tok = tok.trim_end_matches(',').trim();
        if tok.is_empty() {
            continue;
        }

        if let Some(stripped) = tok.strip_prefix("--") {
            if long.is_none()
                && !stripped.is_empty()
                && stripped
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-')
            {
                long = Some(format!("--{stripped}"));
            }
            continue;
        }

        if tok.starts_with('-') && !tok.starts_with("--") {
            if short.is_none()
                && tok.len() == 2
                && tok
                    .chars()
                    .nth(1)
                    .is_some_and(|c| c.is_ascii_alphanumeric())
            {
                short = Some(tok.to_string());
            }
            continue;
        }

        if value_name.is_none() {
            // clap formats variadic value placeholders like `<FILE>...`
            if tok.starts_with('<') {
                if let Some(end) = tok.find('>') {
                    if end > 1 {
                        value_name = Some(tok[1..end].to_string());
                        if tok[end + 1..].contains("...") {
                            repeatable = Some(true);
                        }
                        continue;
                    }
                }
                continue;
            }

            if tok
                .chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_' || c == '-')
            {
                value_name = Some(tok.to_string());
                continue;
            }
        }
    }

    // Reject help text bullets/continuations like `- untrusted: ...` that are not real flags.
    if long.is_none() && short.is_none() {
        return None;
    }

    let takes_value = value_name.is_some();

    Some(FlagSnapshot {
        long,
        short,
        takes_value,
        value_name,
        repeatable,
        stability: None,
        platforms: None,
    })
}

fn split_tokens_and_desc(s: &str) -> (&str, &str) {
    let bytes = s.as_bytes();
    for i in 0..bytes.len().saturating_sub(1) {
        if bytes[i] == b' ' && bytes[i + 1] == b' ' {
            let tokens = s[..i].trim_end();
            let desc = s[i..].trim();
            return (tokens, desc);
        }
    }
    (s.trim_end(), "")
}

fn parse_arg_line(line: &str) -> Option<ArgSnapshot> {
    if !line.starts_with(' ') && !line.starts_with('\t') {
        return None;
    }
    let trimmed = line.trim_start();
    if trimmed.starts_with('-') {
        return None;
    }
    let (head, desc) = split_tokens_and_desc(trimmed);
    let token = head.split_whitespace().next()?;

    let (token, token_is_variadic) = token
        .strip_suffix("...")
        .map(|t| (t, true))
        .unwrap_or((token, false));

    let (required, mut name) = if token.starts_with('<') && token.ends_with('>') {
        (
            true,
            token
                .trim_start_matches('<')
                .trim_end_matches('>')
                .to_string(),
        )
    } else if token.starts_with('[') && token.ends_with(']') {
        (
            false,
            token
                .trim_start_matches('[')
                .trim_end_matches(']')
                .to_string(),
        )
    } else if token
        .chars()
        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_' || c == '-')
    {
        // Be conservative: treat bare ALLCAPS tokens as positional args (common in some help
        // formats) but avoid mis-parsing wrapped description lines as arguments.
        (false, token.to_string())
    } else {
        return None;
    };

    let mut variadic = token_is_variadic;
    if let Some(stripped) = name.strip_suffix("...") {
        variadic = true;
        name = stripped.to_string();
    }

    if name.is_empty() {
        return None;
    }

    Some(ArgSnapshot {
        name,
        required,
        variadic,
        note: if desc.is_empty() {
            None
        } else {
            Some(desc.to_string())
        },
    })
}

fn merge_inferred_args(args: &mut Vec<ArgSnapshot>, inferred: Vec<ArgSnapshot>) {
    for inf in inferred {
        if let Some(existing) = args.iter_mut().find(|a| a.name == inf.name) {
            existing.required |= inf.required;
            existing.variadic |= inf.variadic;
            if existing.note.is_none() {
                existing.note = inf.note;
            }
            continue;
        }
        args.push(inf);
    }
}

fn infer_args_from_usage(
    usage: &str,
    cmd_path: &[String],
    has_subcommands: bool,
) -> Vec<ArgSnapshot> {
    let mut out = Vec::new();

    for line in usage.lines() {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.is_empty() {
            continue;
        }

        // A clap "usage" line typically looks like: `codex <subcommands...> [OPTIONS] [ARGS...]`.
        // Only infer args when this usage line matches the command path we’re snapshotting.
        let mut idx = 0usize;
        if tokens.first().is_some_and(|t| *t == "codex") {
            idx += 1;
        }

        let mut matches = true;
        for p in cmd_path {
            if tokens.get(idx).is_some_and(|t| *t == p) {
                idx += 1;
            } else {
                matches = false;
                break;
            }
        }
        if !matches {
            continue;
        }

        let mut prev_was_flag = false;
        let mut after_double_dash = false;
        for tok in tokens.into_iter().skip(idx) {
            let tok = tok.trim_matches(|c| matches!(c, '(' | ')' | '|'));
            if tok.is_empty() {
                continue;
            }

            if !after_double_dash {
                if tok == "--" {
                    after_double_dash = true;
                    prev_was_flag = false;
                    continue;
                }

                // Some clap usage lines embed flags inside grouping tokens (e.g.
                // `<COMMAND|--url <URL>>`). Treat any token that contains a flag marker as a flag
                // so its value name is not mis-inferred as a positional argument.
                if tok.starts_with('-') || tok.contains("--") {
                    prev_was_flag = true;
                    continue;
                }

                if prev_was_flag {
                    // Likely a value name for the previous flag (e.g., `--out <DIR>`). Don’t treat it
                    // as a positional argument.
                    prev_was_flag = false;
                    continue;
                }
            }

            if tok.eq_ignore_ascii_case("[options]") || tok.eq_ignore_ascii_case("options") {
                continue;
            }

            let (name, required, variadic) = match parse_usage_arg_token(tok) {
                Some(v) => v,
                None => continue,
            };

            // Clap often represents subcommand dispatch using `[COMMAND]` and `[ARGS]` in usage lines
            // even when the help output includes an explicit `Commands:` section. Treat these as
            // implementation details, not stable positional args, and avoid inferring them.
            if has_subcommands && matches!(name.as_str(), "COMMAND" | "ARGS") {
                continue;
            }

            // Avoid duplicates across multiple usage variants.
            if out.iter().any(|a: &ArgSnapshot| a.name == name) {
                continue;
            }

            out.push(ArgSnapshot {
                name,
                required,
                variadic,
                note: Some("inferred from usage".to_string()),
            });
        }
    }

    out
}

fn parse_usage_arg_token(token: &str) -> Option<(String, bool, bool)> {
    let token = token.trim().trim_matches(',');
    if token.is_empty() {
        return None;
    }

    let (token, token_is_variadic) = token
        .strip_suffix("...")
        .map(|t| (t, true))
        .unwrap_or((token, false));

    if token == "[OPTIONS]" || token.eq_ignore_ascii_case("options") {
        return None;
    }

    if token.starts_with('<') && token.ends_with('>') {
        let name = token
            .trim_start_matches('<')
            .trim_end_matches('>')
            .to_string();
        if name.is_empty() {
            return None;
        }
        return Some((name, true, token_is_variadic));
    }

    if token.starts_with('[') && token.ends_with(']') {
        let name = token
            .trim_start_matches('[')
            .trim_end_matches(']')
            .to_string();
        if name.is_empty() {
            return None;
        }
        return Some((name, false, token_is_variadic));
    }

    None
}
