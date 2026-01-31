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
/// This value is consumed by `xtask codex-wrapper-coverage` to generate
/// `cli_manifests/codex/wrapper_coverage.json`.
pub fn wrapper_coverage_manifest() -> WrapperCoverageManifestV1 {
    fn flag(key: &str, level: CoverageLevel) -> WrapperFlagCoverageV1 {
        WrapperFlagCoverageV1 {
            key: key.to_string(),
            level,
            note: None,
            scope: None,
        }
    }

    fn flag_note(key: &str, level: CoverageLevel, note: &str) -> WrapperFlagCoverageV1 {
        WrapperFlagCoverageV1 {
            key: key.to_string(),
            level,
            note: Some(note.to_string()),
            scope: None,
        }
    }

    fn arg(name: &str, level: CoverageLevel) -> WrapperArgCoverageV1 {
        WrapperArgCoverageV1 {
            name: name.to_string(),
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
            // Scenario 0: root/global flags and probe flags.
            command(
                &[],
                CoverageLevel::Explicit,
                None,
                vec![
                    flag("--help", CoverageLevel::Explicit),
                    flag("--version", CoverageLevel::Explicit),
                    flag("--model", CoverageLevel::Explicit),
                    flag("--image", CoverageLevel::Explicit),
                    flag_note("--add-dir", CoverageLevel::Explicit, "capability-guarded"),
                    flag("--config", CoverageLevel::Passthrough),
                    flag("--enable", CoverageLevel::Passthrough),
                    flag("--disable", CoverageLevel::Passthrough),
                    flag("--profile", CoverageLevel::Explicit),
                    flag("--cd", CoverageLevel::Explicit),
                    flag("--ask-for-approval", CoverageLevel::Explicit),
                    flag("--sandbox", CoverageLevel::Explicit),
                    flag("--full-auto", CoverageLevel::Explicit),
                    flag(
                        "--dangerously-bypass-approvals-and-sandbox",
                        CoverageLevel::Explicit,
                    ),
                    flag("--local-provider", CoverageLevel::Explicit),
                    flag("--oss", CoverageLevel::Explicit),
                    flag("--search", CoverageLevel::Explicit),
                ],
                vec![],
            ),

            // Scenario 1+2: `codex exec` (single-response + streaming).
            command(
                &["exec"],
                CoverageLevel::Explicit,
                None,
                vec![
                    flag("--color", CoverageLevel::Explicit),
                    flag("--skip-git-repo-check", CoverageLevel::Explicit),
                    flag("--json", CoverageLevel::Explicit),
                    flag("--output-last-message", CoverageLevel::Explicit),
                    flag_note(
                        "--output-schema",
                        CoverageLevel::Explicit,
                        "capability-guarded",
                    ),
                ],
                vec![arg("PROMPT", CoverageLevel::Explicit)],
            ),

            // Scenario 3: `codex exec resume` (streaming resume).
            command(
                &["exec", "resume"],
                CoverageLevel::Explicit,
                None,
                vec![
                    flag("--json", CoverageLevel::Explicit),
                    flag("--skip-git-repo-check", CoverageLevel::Explicit),
                    flag("--last", CoverageLevel::Explicit),
                    flag("--all", CoverageLevel::Explicit),
                ],
                vec![
                    arg("PROMPT", CoverageLevel::Explicit),
                    arg("SESSION_ID", CoverageLevel::Explicit),
                ],
            ),

            // Scenario 4: `codex apply <TASK_ID>`.
            command(
                &["apply"],
                CoverageLevel::Explicit,
                None,
                vec![],
                vec![arg("TASK_ID", CoverageLevel::Explicit)],
            ),

            // Scenario 4: `codex cloud diff <TASK_ID>`.
            command(
                &["cloud", "diff"],
                CoverageLevel::Explicit,
                None,
                vec![],
                vec![arg("TASK_ID", CoverageLevel::Explicit)],
            ),

            // Scenario 5: login/logout.
            command(
                &["login"],
                CoverageLevel::Explicit,
                None,
                vec![
                    flag_note("--mcp", CoverageLevel::Explicit, "capability-guarded"),
                    flag("--api-key", CoverageLevel::Explicit),
                ],
                vec![],
            ),
            command(&["login", "status"], CoverageLevel::Explicit, None, vec![], vec![]),
            command(&["logout"], CoverageLevel::Explicit, None, vec![], vec![]),

            // Scenario 6: `codex features list`.
            command(
                &["features", "list"],
                CoverageLevel::Explicit,
                None,
                vec![flag("--json", CoverageLevel::Explicit)],
                vec![],
            ),

            // Scenario 7: `codex app-server generate-*`.
            command(
                &["app-server", "generate-ts"],
                CoverageLevel::Explicit,
                None,
                vec![
                    flag("--out", CoverageLevel::Explicit),
                    flag("--prettier", CoverageLevel::Explicit),
                ],
                vec![],
            ),
            command(
                &["app-server", "generate-json-schema"],
                CoverageLevel::Explicit,
                None,
                vec![flag("--out", CoverageLevel::Explicit)],
                vec![],
            ),

            // Scenario 8: `codex responses-api-proxy`.
            command(
                &["responses-api-proxy"],
                CoverageLevel::Explicit,
                None,
                vec![
                    flag("--port", CoverageLevel::Explicit),
                    flag("--server-info", CoverageLevel::Explicit),
                    flag("--http-shutdown", CoverageLevel::Explicit),
                    flag("--upstream-url", CoverageLevel::Explicit),
                ],
                vec![],
            ),

            // Scenario 9: `codex stdio-to-uds`.
            command(
                &["stdio-to-uds"],
                CoverageLevel::Explicit,
                None,
                vec![],
                vec![arg("SOCKET_PATH", CoverageLevel::Explicit)],
            ),

            // Scenario 10: `codex sandbox <platform>`.
            command(
                &["sandbox", "macos"],
                CoverageLevel::Explicit,
                None,
                vec![flag("--log-denials", CoverageLevel::Explicit)],
                vec![arg("COMMAND", CoverageLevel::Explicit)],
            ),
            command(
                &["sandbox", "linux"],
                CoverageLevel::Explicit,
                None,
                vec![],
                vec![arg("COMMAND", CoverageLevel::Explicit)],
            ),
            command(
                &["sandbox", "windows"],
                CoverageLevel::Explicit,
                None,
                vec![],
                vec![arg("COMMAND", CoverageLevel::Explicit)],
            ),

            // Scenario 11: `codex execpolicy check`.
            command(
                &["execpolicy", "check"],
                CoverageLevel::Explicit,
                None,
                vec![
                    flag("--policy", CoverageLevel::Explicit),
                    flag("--pretty", CoverageLevel::Explicit),
                ],
                vec![arg("COMMAND", CoverageLevel::Explicit)],
            ),

            // Scenario 12: stdio servers.
            command(&["mcp-server"], CoverageLevel::Explicit, None, vec![], vec![]),
            command(&["app-server"], CoverageLevel::Explicit, None, vec![], vec![]),

            WrapperCommandCoverageV1 {
                path: vec!["completion".to_string()],
                level: CoverageLevel::IntentionallyUnsupported,
                note: Some(
                    "Shell completion generation is out of scope for the wrapper.".to_string(),
                ),
                scope: None,
                flags: None,
                args: None,
            },
            WrapperCommandCoverageV1 {
                path: vec!["cloud".to_string()],
                level: CoverageLevel::IntentionallyUnsupported,
                note: Some(
                    "Cloud command family is intentionally unwrapped (setup/experimental utility)."
                        .to_string(),
                ),
                scope: None,
                flags: None,
                args: None,
            },
            WrapperCommandCoverageV1 {
                path: vec!["mcp".to_string()],
                level: CoverageLevel::IntentionallyUnsupported,
                note: Some(
                    "MCP management commands are intentionally unwrapped (experimental/admin surface)."
                        .to_string(),
                ),
                scope: None,
                flags: None,
                args: None,
            },
        ],
    }
}
