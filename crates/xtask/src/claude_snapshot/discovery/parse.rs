use super::super::{ArgSnapshot, FlagSnapshot};

#[derive(Debug)]
pub(super) struct ParsedHelp {
    pub(super) about: Option<String>,
    pub(super) usage: Option<String>,
    pub(super) subcommands: Vec<String>,
    pub(super) flags: Vec<FlagSnapshot>,
    pub(super) args: Vec<ArgSnapshot>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Section {
    Commands,
    Options,
    Args,
}

pub(super) fn parse_help(help: &str) -> ParsedHelp {
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

fn is_clap_placeholder(tok: &str) -> bool {
    let t = tok.trim_matches(',').trim();
    if t.is_empty() {
        return true;
    }
    if t == "..." || t.ends_with("...") {
        return true;
    }
    if (t.starts_with('[') && t.ends_with(']')) || (t.starts_with('<') && t.ends_with('>')) {
        return true;
    }
    false
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
    // on-the-same-line description.
    let (head, desc) = split_tokens_and_desc(trimmed);
    if desc.is_empty() {
        return None;
    }

    let mut parts = head.split_whitespace();
    let first = parts.next()?;
    let token = first.split('|').next().unwrap_or(first);

    // Claude Code help often includes a meta `help` subcommand (e.g. `claude mcp help [command]`)
    // that does not behave like a real leaf command under `--help` crawling and may exit non-zero.
    // Skip it to keep snapshot generation deterministic.
    if token == "help" || token == "claude" {
        return None;
    }

    if !token
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return None;
    }

    // Clap may print command tokens with inline placeholders:
    // `add [options] <name> <commandOrUrl> [args...]  Add a server`
    // Accept these as long as the remainder are placeholder-like tokens.
    for rest in parts {
        if rest.starts_with('-') {
            return None;
        }
        if !is_clap_placeholder(rest) {
            return None;
        }
    }

    Some(token.to_string())
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

#[cfg(test)]
mod tests {
    use super::parse_help;

    #[test]
    fn parses_clap_command_lines_with_inline_placeholders() {
        let help = r#"
Usage: claude mcp [options] [command]

Configure and manage MCP servers

Commands:
  serve [options]                                Start the Claude Code MCP server
  add [options] <name> <commandOrUrl> [args...]  Add a server
  remove [options] <name>                        Remove a server
  list                                           List configured MCP servers
  help [command]                                 display help for command

Options:
  -h, --help                                     Display help for command
"#;

        let parsed = parse_help(help);
        assert!(parsed.subcommands.contains(&"serve".to_string()));
        assert!(parsed.subcommands.contains(&"add".to_string()));
        assert!(parsed.subcommands.contains(&"remove".to_string()));
        assert!(parsed.subcommands.contains(&"list".to_string()));
        assert!(!parsed.subcommands.contains(&"help".to_string()));
    }

    #[test]
    fn rejects_non_placeholder_tail_tokens() {
        let help = r#"
Usage: claude something [options] [command]

Commands:
  add --not-a-placeholder  Something
"#;
        let parsed = parse_help(help);
        assert!(!parsed.subcommands.contains(&"add".to_string()));
    }
}
