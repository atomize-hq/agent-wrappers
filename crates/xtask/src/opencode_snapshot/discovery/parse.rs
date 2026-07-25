//! Pure parsing of yargs help text.
//!
//! Split out from the command walk so the row/section grammar can be unit-tested against captured
//! fixtures without executing a binary.

use std::collections::BTreeMap;

use super::super::{ArgSnapshot, FlagSnapshot};

/// One `Commands:` row: the child's path plus the usage text that declares its positionals.
pub(super) struct ChildSpec {
    pub(super) path: Vec<String>,
    pub(super) usage: String,
    pub(super) about: Option<String>,
}

// ---------------------------------------------------------------------------------------------
// Section splitting
// ---------------------------------------------------------------------------------------------

#[derive(Default)]
pub(super) struct Sections {
    pub(super) commands: Vec<Entry>,
    pub(super) positionals: Vec<Entry>,
    pub(super) options: Vec<Entry>,
}

/// One logical row, with yargs metadata separated out.
///
/// `head` keeps the first line's *original* internal spacing, because the boundary between a
/// row's declaration and its description is a run of two or more spaces — collapsing whitespace
/// here would erase the only thing that separates `opencode agent create` from `create a new
/// agent`. Continuation lines are description-only, so they are collapsed and kept apart.
pub(super) struct Entry {
    pub(super) head: String,
    pub(super) continuation: String,
    pub(super) annotations: Vec<String>,
}

pub(super) fn parse_sections(help: &str) -> Sections {
    let mut sections = Sections::default();
    let mut current: Option<&'static str> = None;
    let mut pending: Vec<String> = Vec::new();

    fn flush(current: Option<&'static str>, pending: &mut Vec<String>, sections: &mut Sections) {
        if pending.is_empty() {
            return;
        }
        let entry = build_entry(pending);
        pending.clear();
        match current {
            Some("commands") => sections.commands.push(entry),
            Some("positionals") => sections.positionals.push(entry),
            Some("options") => sections.options.push(entry),
            _ => {}
        }
    }

    for raw_line in help.lines() {
        let trimmed = raw_line.trim_end();
        let header = match trimmed.trim() {
            "Commands:" => Some("commands"),
            "Positionals:" => Some("positionals"),
            "Options:" => Some("options"),
            _ => None,
        };
        if let Some(header) = header {
            flush(current, &mut pending, &mut sections);
            current = Some(header);
            continue;
        }

        if current.is_none() {
            continue;
        }
        if trimmed.trim().is_empty() {
            flush(current, &mut pending, &mut sections);
            continue;
        }

        if pending.is_empty() || starts_row(current, trimmed) {
            flush(current, &mut pending, &mut sections);
        }
        pending.push(trimmed.to_string());
    }
    flush(current, &mut pending, &mut sections);

    sections
}

/// Deepest indent at which a new row can begin.
///
/// yargs left-aligns rows in a shallow column (2 for `Commands:`/`Positionals:`, 2 or 6 for
/// `Options:` depending on whether the flag has a short form) and wraps continuations out at the
/// description column, which is far deeper. Anything past this is continuation text.
const MAX_ROW_INDENT: usize = 8;

fn starts_row(section: Option<&'static str>, line: &str) -> bool {
    let indent = line.len() - line.trim_start().len();
    if indent > MAX_ROW_INDENT {
        return false;
    }
    // An `Options:` row always begins with its flag, which lets `      --format` (aligned past a
    // short-form column) be recognized as a row rather than folded into the previous one.
    if section == Some("options") {
        return line.trim_start().starts_with('-');
    }
    true
}

fn build_entry(lines: &[String]) -> Entry {
    let mut annotations = Vec::new();
    let mut head = String::new();
    let mut continuation_parts: Vec<String> = Vec::new();

    for (idx, line) in lines.iter().enumerate() {
        let (text, mut anns) = split_annotations(line);
        annotations.append(&mut anns);
        if idx == 0 {
            head = text.trim_end().to_string();
        } else {
            continuation_parts.push(text);
        }
    }

    let continuation = continuation_parts
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    Entry {
        head,
        continuation,
        annotations,
    }
}

/// Peel trailing balanced `[...]` groups off a help line.
///
/// A hand-rolled scan rather than a regex because yargs nests brackets inside its own metadata
/// (`[array] [default: []]`) and quotes strings inside it (`[choices: "DEBUG", "INFO"]`).
pub(super) fn split_annotations(line: &str) -> (String, Vec<String>) {
    let chars: Vec<char> = line.chars().collect();
    let mut end = chars.len();
    let mut groups: Vec<String> = Vec::new();

    loop {
        let mut cursor = end;
        while cursor > 0 && chars[cursor - 1].is_whitespace() {
            cursor -= 1;
        }
        if cursor == 0 || chars[cursor - 1] != ']' {
            break;
        }

        let mut depth = 0usize;
        let mut idx = cursor;
        let start = loop {
            if idx == 0 {
                break None;
            }
            idx -= 1;
            match chars[idx] {
                ']' => depth += 1,
                '[' => {
                    depth -= 1;
                    if depth == 0 {
                        break Some(idx);
                    }
                }
                _ => {}
            }
        };

        let Some(start) = start else { break };
        groups.push(chars[start..cursor].iter().collect());
        end = start;
    }

    groups.reverse();
    let text: String = chars[..end].iter().collect();
    (text.trim_end().to_string(), groups)
}

// ---------------------------------------------------------------------------------------------
// Row parsing
// ---------------------------------------------------------------------------------------------

/// Split a row into its leading declaration and its description, at the first 2+ space run.
fn split_declaration(entry: &Entry) -> (String, Option<String>) {
    let trimmed = entry.head.trim();
    let (decl, head_desc) = match find_gap(trimmed) {
        Some(idx) => {
            let (decl, rest) = trimmed.split_at(idx);
            (decl.trim().to_string(), rest.trim().to_string())
        }
        None => (trimmed.to_string(), String::new()),
    };

    let mut description = head_desc;
    if !entry.continuation.is_empty() {
        if !description.is_empty() {
            description.push(' ');
        }
        description.push_str(&entry.continuation);
    }
    let description = description.split_whitespace().collect::<Vec<_>>().join(" ");

    (
        decl,
        if description.is_empty() {
            None
        } else {
            Some(description)
        },
    )
}

fn find_gap(text: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut run = 0usize;
    for (idx, byte) in bytes.iter().enumerate() {
        if *byte == b' ' {
            run += 1;
        } else {
            if run >= 2 {
                return Some(idx - run);
            }
            run = 0;
        }
    }
    None
}

pub(super) fn parse_children(sections: &Sections, parent: &[String]) -> Vec<ChildSpec> {
    let mut out = Vec::new();
    for entry in &sections.commands {
        let (usage, about) = split_declaration(entry);
        let Some(path) = command_path(&usage) else {
            continue;
        };
        // The `[default]` row restates the parent itself; it is identity, not a child.
        if path.len() <= parent.len() {
            continue;
        }
        out.push(ChildSpec { path, usage, about });
    }
    out
}

/// Command path = the usage tokens after the program name, minus positional placeholders.
fn command_path(usage: &str) -> Option<Vec<String>> {
    let mut tokens = usage.split_whitespace();
    tokens.next()?; // the program name
    Some(
        tokens
            .filter(|t| !t.starts_with('<') && !t.starts_with('['))
            .map(|t| t.to_string())
            .collect(),
    )
}

/// The root's own usage/about come from its `[default]` row in the `Commands:` table.
pub(super) fn root_identity(sections: &Sections) -> (Option<String>, Option<String>) {
    for entry in &sections.commands {
        if !entry.annotations.iter().any(|a| a == "[default]") {
            continue;
        }
        let (usage, about) = split_declaration(entry);
        if command_path(&usage).map(|p| p.is_empty()).unwrap_or(false) {
            return (Some(usage), about);
        }
    }
    (None, None)
}

pub(super) fn parse_positionals(sections: &Sections, usage: &str) -> Vec<ArgSnapshot> {
    let placeholders = usage_placeholders(usage);
    let mut out = Vec::new();
    for entry in &sections.positionals {
        let (name, _desc) = split_declaration(entry);
        let name = name.trim().to_string();
        if name.is_empty() {
            continue;
        }
        let (required, variadic) = placeholders
            .get(&name)
            .copied()
            .unwrap_or((false, is_variadic_annotation(&entry.annotations)));
        out.push(ArgSnapshot {
            name,
            required,
            variadic,
            note: None,
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// Required/variadic are declared by the usage line's brackets, not by the positional row.
fn usage_placeholders(usage: &str) -> BTreeMap<String, (bool, bool)> {
    let mut out = BTreeMap::new();
    for token in usage.split_whitespace() {
        let (required, inner) = if let Some(inner) = token.strip_prefix('<') {
            (true, inner.trim_end_matches('>'))
        } else if let Some(inner) = token.strip_prefix('[') {
            (false, inner.trim_end_matches(']'))
        } else {
            continue;
        };
        let variadic = inner.ends_with("..");
        let name = inner.trim_end_matches('.').to_string();
        if !name.is_empty() {
            out.insert(name, (required, variadic));
        }
    }
    out
}

fn is_variadic_annotation(annotations: &[String]) -> bool {
    annotations.iter().any(|a| a.starts_with("[array]"))
}

pub(super) fn parse_options(sections: &Sections) -> Vec<FlagSnapshot> {
    let mut out: Vec<FlagSnapshot> = Vec::new();
    for entry in &sections.options {
        let (decl, _desc) = split_declaration(entry);
        let mut short = None;
        let mut long = None;
        for token in decl.split(',').map(str::trim) {
            let token = token.split_whitespace().next().unwrap_or(token);
            if let Some(rest) = token.strip_prefix("--") {
                if !rest.is_empty() {
                    long = Some(token.to_string());
                }
            } else if token.starts_with('-') && token.len() > 1 {
                short = Some(token.to_string());
            }
        }
        if short.is_none() && long.is_none() {
            continue;
        }

        let value_type = annotation_type(&entry.annotations);
        out.push(FlagSnapshot {
            long,
            short,
            takes_value: matches!(value_type, Some("string" | "number" | "array" | "count")),
            value_name: None,
            repeatable: Some(matches!(value_type, Some("array"))),
            stability: None,
            platforms: None,
        });
    }

    out.sort_by(|a, b| {
        flag_key(a)
            .cmp(&flag_key(b))
            .then_with(|| a.long.cmp(&b.long))
            .then_with(|| a.short.cmp(&b.short))
    });
    out.dedup_by(|a, b| flag_key(a) == flag_key(b));
    out
}

fn flag_key(flag: &FlagSnapshot) -> String {
    flag.long
        .clone()
        .or_else(|| flag.short.clone())
        .unwrap_or_default()
}

fn annotation_type(annotations: &[String]) -> Option<&'static str> {
    for annotation in annotations {
        match annotation.as_str() {
            "[boolean]" => return Some("boolean"),
            "[string]" => return Some("string"),
            "[number]" => return Some("number"),
            "[array]" => return Some("array"),
            "[count]" => return Some("count"),
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Captured verbatim from `opencode --help` (1.18.4), trimmed to the interesting rows.
    const ROOT_HELP: &str = r#"
Commands:
  opencode mcp                 manage MCP (Model Context Protocol) servers
  opencode [project]           start opencode tui                                          [default]
  opencode attach <url>        attach to a running opencode server
  opencode run [message..]     run opencode with a message
  opencode providers           manage AI providers and credentials                   [aliases: auth]

Positionals:
  project  path to start opencode in                                                        [string]

Options:
  -h, --help          show help                                                            [boolean]
      --log-level     log level                 [string] [choices: "DEBUG", "INFO", "WARN", "ERROR"]
      --mdns          enable mDNS service discovery (defaults hostname to 0.0.0.0)
                                                                          [boolean] [default: false]
      --cors          additional domains to allow for CORS                     [array] [default: []]
  -m, --model         model to use in the format of provider/model                          [string]
      --port          port to listen on                                        [number] [default: 0]
"#;

    fn flag<'a>(flags: &'a [FlagSnapshot], long: &str) -> &'a FlagSnapshot {
        flags
            .iter()
            .find(|f| f.long.as_deref() == Some(long))
            .unwrap_or_else(|| panic!("missing flag {long}"))
    }

    #[test]
    fn commands_section_yields_child_paths_without_positional_placeholders() {
        let sections = parse_sections(ROOT_HELP);
        let children = parse_children(&sections, &[]);
        let paths: Vec<Vec<String>> = children.iter().map(|c| c.path.clone()).collect();

        assert!(paths.contains(&vec!["mcp".to_string()]));
        assert!(paths.contains(&vec!["attach".to_string()]));
        assert!(paths.contains(&vec!["run".to_string()]));
        assert!(paths.contains(&vec!["providers".to_string()]));
        // The `[default]` row restates the root itself and must not become a child.
        assert!(!paths.iter().any(|p| p.is_empty()));
        // Description words must never leak into a command path.
        assert!(
            !paths.iter().any(|p| p.iter().any(|t| t == "manage")),
            "description text leaked into a command path: {paths:?}"
        );
    }

    #[test]
    fn command_about_drops_trailing_yargs_metadata() {
        let sections = parse_sections(ROOT_HELP);
        let children = parse_children(&sections, &[]);
        let providers = children
            .iter()
            .find(|c| c.path == vec!["providers".to_string()])
            .expect("providers row");
        assert_eq!(
            providers.about.as_deref(),
            Some("manage AI providers and credentials"),
            "`[aliases: auth]` is metadata, not description"
        );
    }

    #[test]
    fn root_identity_comes_from_the_default_row() {
        let sections = parse_sections(ROOT_HELP);
        let (usage, about) = root_identity(&sections);
        assert_eq!(usage.as_deref(), Some("opencode [project]"));
        assert_eq!(about.as_deref(), Some("start opencode tui"));
    }

    #[test]
    fn positional_arity_is_read_from_the_usage_line_not_the_positionals_row() {
        let sections = parse_sections(ROOT_HELP);
        let args = parse_positionals(&sections, "opencode [project]");
        assert_eq!(args.len(), 1);
        assert_eq!(args[0].name, "project");
        assert!(!args[0].required, "[project] is optional");
        assert!(!args[0].variadic);

        // `<url>` is required, `[message..]` is optional and variadic.
        let required = usage_placeholders("opencode attach <url>");
        assert_eq!(required.get("url").copied(), Some((true, false)));
        let variadic = usage_placeholders("opencode run [message..]");
        assert_eq!(variadic.get("message").copied(), Some((false, true)));
    }

    #[test]
    fn option_value_semantics_come_from_the_yargs_type_annotation() {
        let sections = parse_sections(ROOT_HELP);
        let flags = parse_options(&sections);

        assert!(
            !flag(&flags, "--help").takes_value,
            "[boolean] takes no value"
        );
        assert!(
            flag(&flags, "--model").takes_value,
            "[string] takes a value"
        );
        assert!(flag(&flags, "--port").takes_value, "[number] takes a value");

        let cors = flag(&flags, "--cors");
        assert!(cors.takes_value, "[array] takes a value");
        assert_eq!(cors.repeatable, Some(true), "[array] is repeatable");

        assert_eq!(flag(&flags, "--help").short.as_deref(), Some("-h"));
        assert_eq!(flag(&flags, "--model").short.as_deref(), Some("-m"));
        assert!(flag(&flags, "--log-level").short.is_none());
    }

    #[test]
    fn a_flag_aligned_past_the_short_form_column_is_its_own_row() {
        // `      --log-level` is indented to align past `  -h, `. Treating indent alone as a
        // continuation signal would swallow every short-form-less flag into the previous row.
        let sections = parse_sections(ROOT_HELP);
        let flags = parse_options(&sections);
        for long in ["--log-level", "--mdns", "--cors", "--port"] {
            assert!(
                flags.iter().any(|f| f.long.as_deref() == Some(long)),
                "{long} must be parsed as its own option row"
            );
        }
    }

    #[test]
    fn a_wrapped_annotation_still_types_its_flag() {
        // `--mdns` carries `[boolean] [default: false]` on a wrapped continuation line.
        let sections = parse_sections(ROOT_HELP);
        let flags = parse_options(&sections);
        assert!(
            !flag(&flags, "--mdns").takes_value,
            "the wrapped [boolean] annotation must still be attached to --mdns"
        );
    }

    #[test]
    fn nested_command_rows_are_discovered_from_a_subcommand_help() {
        const MCP_HELP: &str = r#"
opencode mcp

manage MCP (Model Context Protocol) servers

Commands:
  opencode mcp add [name]     add an MCP server
  opencode mcp list           list MCP servers and their status                        [aliases: ls]

Options:
  -h, --help        show help                                                              [boolean]
"#;
        let sections = parse_sections(MCP_HELP);
        let children = parse_children(&sections, &["mcp".to_string()]);
        let paths: Vec<Vec<String>> = children.iter().map(|c| c.path.clone()).collect();
        assert!(paths.contains(&vec!["mcp".to_string(), "add".to_string()]));
        assert!(paths.contains(&vec!["mcp".to_string(), "list".to_string()]));

        let add = children
            .iter()
            .find(|c| c.path == vec!["mcp".to_string(), "add".to_string()])
            .expect("mcp add");
        assert_eq!(add.usage, "opencode mcp add [name]");
        assert_eq!(add.about.as_deref(), Some("add an MCP server"));
    }

    #[test]
    fn annotations_peel_off_even_when_they_nest_or_quote_brackets() {
        let (text, anns) = split_annotations(
            r#"      --cors  additional domains to allow for CORS   [array] [default: []]"#,
        );
        assert!(text.trim_end().ends_with("for CORS"));
        assert_eq!(
            anns,
            vec!["[array]".to_string(), "[default: []]".to_string()]
        );

        let (text, anns) = split_annotations(
            r#"      --log-level  log level   [string] [choices: "DEBUG", "INFO"]"#,
        );
        assert!(text.trim_end().ends_with("log level"));
        assert_eq!(
            anns,
            vec![
                "[string]".to_string(),
                r#"[choices: "DEBUG", "INFO"]"#.to_string()
            ]
        );
    }
}
