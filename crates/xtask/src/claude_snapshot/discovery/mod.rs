use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    time::Duration,
};

use std::time::Instant;

use super::{layout, supplements, CommandSnapshot, Error};

mod parse;
mod run_help;
mod usage_infer;

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
    const MAX_PATH_DEPTH: usize = 12;

    let mut out = BTreeMap::<Vec<String>, CommandSnapshot>::new();
    let mut visited = BTreeSet::<Vec<String>>::new();
    let mut known_omissions: Vec<String> = Vec::new();
    let help_timeout = Duration::from_millis(help_timeout_ms);
    let started_at = Instant::now();
    let mut last_progress_at = Instant::now();

    let root_help = run_help::run_claude_help_strict(claude_binary, &[], help_timeout)?;
    let root_parsed = parse::parse_help(&root_help);

    if capture_raw_help {
        if let Some(dir) = raw_help_dir {
            layout::write_raw_help(dir, &[], &root_help)?;
        }
    }

    let mut root_args = root_parsed.args;
    if let Some(usage) = root_parsed.usage.as_deref() {
        usage_infer::merge_inferred_args(
            &mut root_args,
            usage_infer::infer_args_from_usage(usage, &[], !root_parsed.subcommands.is_empty()),
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

    // Use an explicit stack to avoid deep recursion (which can stack-overflow on Windows if the
    // CLI presents a deeply nested or accidentally self-referential command tree).
    let mut stack: Vec<Vec<String>> = root_parsed
        .subcommands
        .into_iter()
        .map(|t| vec![t])
        .collect();

    let mut probed = 0usize;
    let mut omission_count = 0usize;

    while let Some(path) = stack.pop() {
        if !visited.insert(path.clone()) {
            continue;
        }

        probed += 1;
        if probed == 1 || probed % 25 == 0 || last_progress_at.elapsed() >= Duration::from_secs(5)
        {
            last_progress_at = Instant::now();
            let elapsed = started_at.elapsed();
            eprintln!(
                "[claude-snapshot] probed={} visited={} queue={} elapsed={}s last={}",
                probed,
                visited.len(),
                stack.len(),
                elapsed.as_secs(),
                if path.is_empty() {
                    "<root>".to_string()
                } else {
                    path.join(" ")
                }
            );
        }

        match run_help::run_claude_help_tolerant(claude_binary, &path, help_timeout) {
            Ok(help) => {
                let parsed = parse::parse_help(&help);

                if capture_raw_help {
                    if let Some(dir) = raw_help_dir {
                        layout::write_raw_help(dir, &path, &help)?;
                    }
                }

                let mut args = parsed.args;
                if let Some(usage) = parsed.usage.as_deref() {
                    usage_infer::merge_inferred_args(
                        &mut args,
                        usage_infer::infer_args_from_usage(usage, &path, !parsed.subcommands.is_empty()),
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
                    if path.len() + 1 > MAX_PATH_DEPTH {
                        known_omissions.push(format!(
                            "max command depth exceeded (>{MAX_PATH_DEPTH}): {} {}",
                            if path.is_empty() {
                                "<root>".to_string()
                            } else {
                                path.join(" ")
                            },
                            sub
                        ));
                        continue;
                    }

                    // Guard against accidental self-recursive help trees. We’ve observed cases where
                    // a command’s help lists itself as a subcommand (e.g. `plugin manifest` listing
                    // `manifest` again), which would otherwise explode into an infinite unique-path
                    // traversal (`... manifest manifest manifest ...`).
                    if path.last().is_some_and(|last| last == &sub) {
                        known_omissions.push(format!(
                            "skipped recursive subcommand token: {} {}",
                            if path.is_empty() {
                                "<root>".to_string()
                            } else {
                                path.join(" ")
                            },
                            sub
                        ));
                        continue;
                    }

                    // Guard against multi-token cycles like:
                    // `plugin manifest marketplace` -> `manifest` -> `marketplace` -> ...
                    // where the CLI's help output presents a cyclic tree. Without this, snapshot
                    // generation produces a large number of synthetic command paths that aren't
                    // actionable wrapper gaps.
                    if path.iter().any(|t| t == &sub) {
                        known_omissions.push(format!(
                            "skipped cyclic subcommand token: {} {}",
                            if path.is_empty() {
                                "<root>".to_string()
                            } else {
                                path.join(" ")
                            },
                            sub
                        ));
                        continue;
                    }

                    let mut next = path.clone();
                    next.push(sub);
                    stack.push(next);
                }
            }
            Err(note) => {
                // Keep the snapshot deterministic and progressing even if a specific `--help`
                // probe fails (e.g., requires auth, tries to do first-run setup, or exits 1).
                omission_count += 1;
                if omission_count <= 25 {
                    eprintln!("[claude-snapshot] omission: {note}");
                } else if omission_count == 26 {
                    eprintln!("[claude-snapshot] omission: (further omissions suppressed)");
                }
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
