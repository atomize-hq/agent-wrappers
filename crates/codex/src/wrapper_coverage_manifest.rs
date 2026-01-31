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
    WrapperCoverageManifestV1 {
        schema_version: 1,
        generated_at: None,
        wrapper_version: None,
        coverage: vec![
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
