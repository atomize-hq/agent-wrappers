use super::{CoverageLevel, WrapperArgCoverageV1, WrapperCommandCoverageV1, WrapperFlagCoverageV1};

const NOTE: &str = "Forwarded verbatim by NonTuiCommandRequest.";

fn flag(key: &str) -> WrapperFlagCoverageV1 {
    WrapperFlagCoverageV1 {
        key: key.to_string(),
        level: CoverageLevel::Passthrough,
        note: Some(NOTE.to_string()),
        scope: None,
    }
}

fn arg(name: &str) -> WrapperArgCoverageV1 {
    WrapperArgCoverageV1 {
        name: name.to_string(),
        level: CoverageLevel::Passthrough,
        note: Some(NOTE.to_string()),
        scope: None,
    }
}

fn command(path: &[&str], flags: &[&str], args: &[&str]) -> WrapperCommandCoverageV1 {
    WrapperCommandCoverageV1 {
        path: path.iter().map(ToString::to_string).collect(),
        level: CoverageLevel::Passthrough,
        note: Some("Forwarded by NonTuiCommandRequest.".to_string()),
        scope: None,
        flags: (!flags.is_empty()).then(|| flags.iter().map(|key| flag(key)).collect()),
        args: (!args.is_empty()).then(|| args.iter().map(|name| arg(name)).collect()),
    }
}

/// Coverage declarations for the bounded 0.144.6 packet compatibility API.
pub(super) fn packet_non_tui_coverage() -> Vec<WrapperCommandCoverageV1> {
    vec![
        command(&["app-server", "daemon"], &[], &[]),
        command(
            &["app-server", "daemon", "bootstrap"],
            &["--remote-control"],
            &[],
        ),
        command(
            &["app-server", "daemon", "disable-remote-control"],
            &[],
            &[],
        ),
        command(&["app-server", "daemon", "enable-remote-control"], &[], &[]),
        command(&["app-server", "daemon", "help"], &[], &["COMMAND"]),
        command(&["app-server", "daemon", "restart"], &[], &[]),
        command(&["app-server", "daemon", "start"], &[], &[]),
        command(&["app-server", "daemon", "stop"], &[], &[]),
        command(&["app-server", "daemon", "version"], &[], &[]),
        command(&["archive"], &[], &["SESSION"]),
        command(&["delete"], &["--force"], &["SESSION"]),
        command(
            &["doctor"],
            &["--all", "--ascii", "--json", "--no-color", "--summary"],
            &[],
        ),
        command(
            &["plugin", "add"],
            &["--json", "--marketplace"],
            &["PLUGIN[@MARKETPLACE]"],
        ),
        command(
            &["plugin", "list"],
            &["--available", "--json", "--marketplace"],
            &[],
        ),
        command(&["plugin", "marketplace", "list"], &["--json"], &[]),
        command(
            &["plugin", "remove"],
            &["--json", "--marketplace"],
            &["PLUGIN[@MARKETPLACE]"],
        ),
        command(&["remote-control"], &["--json"], &[]),
        command(&["remote-control", "help"], &[], &["COMMAND"]),
        command(&["remote-control", "pair"], &["--json"], &[]),
        command(&["remote-control", "start"], &["--json"], &[]),
        command(&["remote-control", "stop"], &["--json"], &[]),
        command(&["unarchive"], &[], &["SESSION"]),
    ]
}

fn explicit_entry(
    path: &[&str],
    flags: &[(&str, CoverageLevel)],
    args: &[(&str, CoverageLevel)],
) -> WrapperCommandCoverageV1 {
    let note = |level| (level == CoverageLevel::Passthrough).then(|| NOTE.to_string());
    WrapperCommandCoverageV1 {
        path: path.iter().map(ToString::to_string).collect(),
        level: CoverageLevel::Explicit,
        note: None,
        scope: None,
        flags: (!flags.is_empty()).then(|| {
            flags
                .iter()
                .map(|(key, level)| WrapperFlagCoverageV1 {
                    key: (*key).to_string(),
                    level: *level,
                    note: note(*level),
                    scope: None,
                })
                .collect()
        }),
        args: (!args.is_empty()).then(|| {
            args.iter()
                .map(|(name, level)| WrapperArgCoverageV1 {
                    name: (*name).to_string(),
                    level: *level,
                    note: note(*level),
                    scope: None,
                })
                .collect()
        }),
    }
}

/// Existing plugin support plus the explicitly deferred shell-completion debt.
pub(super) fn plugin_and_debt_coverage() -> Vec<WrapperCommandCoverageV1> {
    use CoverageLevel::{Explicit, IntentionallyUnsupported, Passthrough};

    let mut coverage = vec![
        explicit_entry(&["plugin"], &[], &[]),
        explicit_entry(&["plugin", "help"], &[], &[("COMMAND", Explicit)]),
        explicit_entry(&["plugin", "marketplace"], &[], &[]),
        explicit_entry(
            &["plugin", "marketplace", "add"],
            &[
                ("--ref", Explicit),
                ("--sparse", Explicit),
                ("--json", Passthrough),
            ],
            &[("SOURCE", Explicit)],
        ),
        explicit_entry(
            &["plugin", "marketplace", "help"],
            &[],
            &[("COMMAND", Explicit)],
        ),
        explicit_entry(
            &["plugin", "marketplace", "remove"],
            &[("--json", Passthrough)],
            &[("MARKETPLACE_NAME", Explicit)],
        ),
        explicit_entry(
            &["plugin", "marketplace", "upgrade"],
            &[("--json", Passthrough)],
            &[("MARKETPLACE_NAME", Explicit)],
        ),
    ];
    coverage.push(WrapperCommandCoverageV1 {
        path: vec!["completion".to_string()],
        level: IntentionallyUnsupported,
        note: Some("Shell completion generation is out of scope for the wrapper.".to_string()),
        scope: None,
        flags: None,
        args: Some(vec![WrapperArgCoverageV1 {
            name: "SHELL".to_string(),
            level: IntentionallyUnsupported,
            note: Some("Shell completion generation is out of scope for the wrapper.".to_string()),
            scope: None,
        }]),
    });
    coverage
}
