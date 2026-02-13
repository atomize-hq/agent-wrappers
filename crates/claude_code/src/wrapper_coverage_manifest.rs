use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoverageLevel {
    Explicit,
    Passthrough,
    Unsupported,
    IntentionallyUnsupported,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WrapperSurfaceScopedTargets {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platforms: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_triples: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WrapperFlagCoverageV1 {
    pub key: String,
    pub level: CoverageLevel,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<WrapperSurfaceScopedTargets>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WrapperArgCoverageV1 {
    pub name: String,
    pub level: CoverageLevel,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<WrapperSurfaceScopedTargets>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WrapperCommandCoverageV1 {
    pub path: Vec<String>,
    pub level: CoverageLevel,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<WrapperSurfaceScopedTargets>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<Vec<WrapperFlagCoverageV1>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<WrapperArgCoverageV1>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WrapperCoverageManifestV1 {
    pub schema_version: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generated_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wrapper_version: Option<String>,
    pub coverage: Vec<WrapperCommandCoverageV1>,
}

pub fn wrapper_crate_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// The single source of truth for wrapper coverage declarations.
///
/// This value is consumed by `xtask claude-wrapper-coverage` to generate
/// `cli_manifests/claude_code/wrapper_coverage.json`.
pub fn wrapper_coverage_manifest() -> WrapperCoverageManifestV1 {
    fn flag(key: &str, level: CoverageLevel) -> WrapperFlagCoverageV1 {
        WrapperFlagCoverageV1 {
            key: key.to_string(),
            level,
            note: None,
            scope: None,
        }
    }

    fn command(
        path: &[&str],
        level: CoverageLevel,
        note: Option<&str>,
        flags: Vec<WrapperFlagCoverageV1>,
        args: Vec<WrapperArgCoverageV1>,
    ) -> WrapperCommandCoverageV1 {
        WrapperCommandCoverageV1 {
            path: path.iter().map(|s| s.to_string()).collect(),
            level,
            note: note.map(|s| s.to_string()),
            scope: None,
            flags: (!flags.is_empty()).then_some(flags),
            args: (!args.is_empty()).then_some(args),
        }
    }

    WrapperCoverageManifestV1 {
        schema_version: 1,
        generated_at: None,
        wrapper_version: None,
        coverage: vec![
            // Root `claude` in non-interactive print mode.
            command(
                &[],
                CoverageLevel::Explicit,
                None,
                vec![
                    flag("--help", CoverageLevel::Passthrough),
                    flag("--version", CoverageLevel::Passthrough),
                    flag("--print", CoverageLevel::Explicit),
                    flag("--output-format", CoverageLevel::Explicit),
                    flag("--input-format", CoverageLevel::Explicit),
                    flag("--json-schema", CoverageLevel::Explicit),
                    flag("--model", CoverageLevel::Explicit),
                    flag("--allowedTools", CoverageLevel::Explicit),
                    flag("--disallowedTools", CoverageLevel::Explicit),
                    flag("--permission-mode", CoverageLevel::Explicit),
                    flag("--dangerously-skip-permissions", CoverageLevel::Explicit),
                    flag("--add-dir", CoverageLevel::Explicit),
                    flag("--mcp-config", CoverageLevel::Explicit),
                    flag("--strict-mcp-config", CoverageLevel::Explicit),

                    // Remaining root flags are supported via `ClaudePrintRequest::extra_args`.
                    flag("--agent", CoverageLevel::Passthrough),
                    flag("--agents", CoverageLevel::Passthrough),
                    flag("--allow-dangerously-skip-permissions", CoverageLevel::Passthrough),
                    flag("--append-system-prompt", CoverageLevel::Passthrough),
                    flag("--betas", CoverageLevel::Passthrough),
                    flag("--chrome", CoverageLevel::Passthrough),
                    flag("--continue", CoverageLevel::Passthrough),
                    flag("--debug", CoverageLevel::Passthrough),
                    flag("--debug-file", CoverageLevel::Passthrough),
                    flag("--disable-slash-commands", CoverageLevel::Passthrough),
                    flag("--fallback-model", CoverageLevel::Passthrough),
                    flag("--file", CoverageLevel::Passthrough),
                    flag("--fork-session", CoverageLevel::Passthrough),
                    flag("--from-pr", CoverageLevel::Passthrough),
                    flag("--ide", CoverageLevel::Passthrough),
                    flag("--include-partial-messages", CoverageLevel::Passthrough),
                    flag("--max-budget-usd", CoverageLevel::Passthrough),
                    flag("--mcp-debug", CoverageLevel::Passthrough),
                    flag("--no-chrome", CoverageLevel::Passthrough),
                    flag("--no-session-persistence", CoverageLevel::Passthrough),
                    flag("--plugin-dir", CoverageLevel::Passthrough),
                    flag("--replay-user-messages", CoverageLevel::Passthrough),
                    flag("--resume", CoverageLevel::Passthrough),
                    flag("--session-id", CoverageLevel::Passthrough),
                    flag("--setting-sources", CoverageLevel::Passthrough),
                    flag("--settings", CoverageLevel::Passthrough),
                    flag("--system-prompt", CoverageLevel::Passthrough),
                    flag("--tools", CoverageLevel::Passthrough),
                    flag("--verbose", CoverageLevel::Passthrough),
                ],
                vec![],
            ),
            // IU subtree roots (policy): updater/diagnostics.
            command(
                &["update"],
                CoverageLevel::IntentionallyUnsupported,
                Some("Claude Code auto-update is out of scope for this wrapper."),
                vec![],
                vec![],
            ),
            command(
                &["doctor"],
                CoverageLevel::IntentionallyUnsupported,
                Some("Claude Code updater diagnostics are out of scope for this wrapper."),
                vec![],
                vec![],
            ),
            command(
                &["setup-token"],
                CoverageLevel::IntentionallyUnsupported,
                Some("Token setup is out of scope for this wrapper (secrets-handling policy)."),
                vec![],
                vec![],
            ),

            // MCP management is supported via explicit typed requests.
            command(
                &["mcp"],
                CoverageLevel::Explicit,
                None,
                vec![flag("--help", CoverageLevel::Passthrough)],
                vec![],
            ),
            command(&["mcp", "list"], CoverageLevel::Explicit, None, vec![], vec![]),
            command(
                &["mcp", "get"],
                CoverageLevel::Explicit,
                None,
                vec![flag("--help", CoverageLevel::Passthrough)],
                vec![],
            ),
            command(
                &["mcp", "add"],
                CoverageLevel::Explicit,
                None,
                vec![
                    flag("--help", CoverageLevel::Passthrough),
                    flag("--scope", CoverageLevel::Explicit),
                    flag("--transport", CoverageLevel::Explicit),
                    flag("--env", CoverageLevel::Explicit),
                    flag("--header", CoverageLevel::Explicit),
                ],
                vec![],
            ),
            command(
                &["mcp", "remove"],
                CoverageLevel::Explicit,
                None,
                vec![
                    flag("--help", CoverageLevel::Passthrough),
                    flag("--scope", CoverageLevel::Explicit),
                ],
                vec![],
            ),
            command(
                &["mcp", "add-json"],
                CoverageLevel::Explicit,
                None,
                vec![
                    flag("--help", CoverageLevel::Passthrough),
                    flag("--scope", CoverageLevel::Explicit),
                ],
                vec![],
            ),
            command(
                &["mcp", "reset-project-choices"],
                CoverageLevel::Explicit,
                None,
                vec![],
                vec![],
            ),
            // Best-effort MCP commands: usable via `run_command`, but not typed yet.
            command(
                &["mcp", "serve"],
                CoverageLevel::Passthrough,
                Some("Supported via run_command; no typed API yet."),
                vec![flag("--help", CoverageLevel::Passthrough)],
                vec![],
            ),
            command(
                &["mcp", "add-from-claude-desktop"],
                CoverageLevel::Passthrough,
                Some("Supported via run_command; platform-gated upstream."),
                vec![flag("--help", CoverageLevel::Passthrough)],
                vec![],
            ),

            // Plugin management is best-effort passthrough for parity.
            command(
                &["plugin"],
                CoverageLevel::Passthrough,
                Some("Supported via run_command; no typed API yet."),
                vec![flag("--help", CoverageLevel::Passthrough)],
                vec![],
            ),
            command(
                &["plugin", "manifest"],
                CoverageLevel::Passthrough,
                None,
                vec![flag("--help", CoverageLevel::Passthrough)],
                vec![],
            ),
            command(
                &["plugin", "manifest", "marketplace"],
                CoverageLevel::Passthrough,
                None,
                vec![flag("--help", CoverageLevel::Passthrough)],
                vec![],
            ),
            command(
                &["plugin", "marketplace"],
                CoverageLevel::Passthrough,
                None,
                vec![flag("--help", CoverageLevel::Passthrough)],
                vec![],
            ),
            command(
                &["plugin", "marketplace", "repo"],
                CoverageLevel::Passthrough,
                None,
                vec![flag("--help", CoverageLevel::Passthrough)],
                vec![],
            ),
        ],
    }
}
