//! Help-surface discovery for the OpenCode CLI.
//!
//! OpenCode's help is produced by **yargs**, which is a different shape from clap (codex) and
//! from claude's format, so this parser is genuinely per-CLI — that is exactly what a snapshot
//! adapter is for. Everything downstream of it (union, report, validate, promotion) is shared.
//!
//! The format is three optional sections per command:
//!
//! ```text
//! Commands:
//!   opencode mcp add [name]     add an MCP server
//!   opencode providers          manage AI providers            [aliases: auth]
//!
//! Positionals:
//!   message  message to send                          [array] [default: []]
//!
//! Options:
//!   -h, --help       show help                                     [boolean]
//!       --log-level  log level    [string] [choices: "DEBUG", "INFO", "WARN"]
//! ```
//!
//! Trailing `[...]` groups are yargs metadata, not description text, and both descriptions and
//! their metadata can wrap onto indented continuation lines.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

use wait_timeout::ChildExt;

use super::layout;
use super::util;
use super::{CommandSnapshot, Error};

mod parse;
use parse::{
    parse_children, parse_options, parse_positionals, parse_sections, root_identity, ChildSpec,
};

/// Depth ceiling for command recursion.
///
/// yargs will happily print a parent's help for an unrecognized subcommand, so an unbounded walk
/// could otherwise loop; the visited-set below is the real guard and this is a backstop.
const MAX_DEPTH: usize = 6;

pub(super) struct Discovery {
    pub(super) commands: BTreeMap<Vec<String>, CommandSnapshot>,
    pub(super) known_omissions: Vec<String>,
}

pub(super) fn discover_commands(
    binary: &Path,
    raw_help_dir: Option<&Path>,
    capture_raw_help: bool,
    help_timeout_ms: u64,
) -> Result<Discovery, Error> {
    let mut commands: BTreeMap<Vec<String>, CommandSnapshot> = BTreeMap::new();
    let mut known_omissions = Vec::new();
    let mut visited: BTreeSet<Vec<String>> = BTreeSet::new();

    // (path, usage/about inherited from the parent's `Commands:` row)
    let mut queue: Vec<ChildSpec> = vec![ChildSpec {
        path: Vec::new(),
        usage: String::new(),
        about: None,
    }];

    while let Some(spec) = queue.pop() {
        if !visited.insert(spec.path.clone()) {
            continue;
        }
        if spec.path.len() > MAX_DEPTH {
            known_omissions.push(format!(
                "command depth limit reached; skipped `{}`",
                spec.path.join(" ")
            ));
            continue;
        }

        let help = match run_help(binary, &spec.path, help_timeout_ms) {
            Ok(text) => text,
            Err(err) => {
                known_omissions.push(format!(
                    "could not capture help for `{}`: {err}",
                    render_path(&spec.path)
                ));
                continue;
            }
        };

        if capture_raw_help {
            if let Some(dir) = raw_help_dir {
                layout::write_raw_help(dir, &spec.path, &help)?;
            }
        }

        let sections = parse_sections(&help);

        // The root command's own usage/about live in its `Commands:` table as the `[default]`
        // entry; every other command states them in its own help header.
        let (usage, about) = if spec.path.is_empty() {
            root_identity(&sections)
        } else {
            let header_usage = header_usage(&help).unwrap_or_else(|| spec.usage.clone());
            (
                Some(header_usage),
                header_about(&help).or(spec.about.clone()),
            )
        };

        let usage_for_args = usage.clone().unwrap_or_else(|| spec.usage.clone());
        let args = parse_positionals(&sections, &usage_for_args);
        let flags = parse_options(&sections);

        commands.insert(
            spec.path.clone(),
            CommandSnapshot {
                path: spec.path.clone(),
                about,
                usage,
                stability: None,
                platforms: None,
                args: if args.is_empty() { None } else { Some(args) },
                flags: if flags.is_empty() { None } else { Some(flags) },
            },
        );

        for child in parse_children(&sections, &spec.path) {
            if child.path != spec.path {
                queue.push(child);
            }
        }
    }

    Ok(Discovery {
        commands,
        known_omissions,
    })
}

fn render_path(path: &[String]) -> String {
    if path.is_empty() {
        "<root>".to_string()
    } else {
        path.join(" ")
    }
}

fn run_help(binary: &Path, path: &[String], help_timeout_ms: u64) -> Result<String, String> {
    let mut cmd = Command::new(binary);
    cmd.args(path);
    cmd.arg("--help");
    cmd.env("NO_COLOR", "1");
    cmd.env("CLICOLOR", "0");
    cmd.env("TERM", "dumb");
    cmd.env("CI", "1");
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| e.to_string())?;

    // Never let one `--help` invocation hang the whole snapshot.
    let timeout = Duration::from_millis(help_timeout_ms);
    if child
        .wait_timeout(timeout)
        .map_err(|e| e.to_string())?
        .is_none()
    {
        let _ = child.kill();
        let _ = child.wait();
        return Err(format!(
            "timeout after {}ms: {}",
            timeout.as_millis(),
            util::command_string(&cmd)
        ));
    }

    let output = child.wait_with_output().map_err(|e| e.to_string())?;
    let text = util::normalize_text(&output.stdout, &output.stderr);

    // yargs exits 0 for `--help`, but a non-zero exit that still printed usable help is worth
    // keeping: dropping the command entirely would silently shrink the parity surface.
    if output.status.success() || looks_like_help(&text) {
        if text.trim().is_empty() {
            return Err(util::command_failed_message(&cmd, &output));
        }
        return Ok(text);
    }
    Err(util::command_failed_message(&cmd, &output))
}

fn looks_like_help(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("commands:") || lower.contains("options:") || lower.contains("positionals:")
}

/// A subcommand states its usage on the first non-empty line of its own help.
fn header_usage(help: &str) -> Option<String> {
    help.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(|line| line.to_string())
}

/// …and its description on the next non-empty line, before any section header.
fn header_about(help: &str) -> Option<String> {
    let mut non_empty = help.lines().map(str::trim).filter(|line| !line.is_empty());
    non_empty.next()?;
    let candidate = non_empty.next()?;
    if candidate.ends_with(':') {
        return None;
    }
    Some(candidate.to_string())
}
