use std::{
    collections::{BTreeMap, BTreeSet},
    fs, io,
    path::{Path, PathBuf},
};

use clap::{Parser, ValueEnum};
use semver::Version;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

mod coverage;
mod models;

use coverage::compute_coverage;
use models::{UnionSnapshotV2, WrapperCoverageV1};

#[derive(Debug, Parser)]
pub struct Args {
    /// Root `cli_manifests/codex` directory.
    #[arg(long, default_value = "cli_manifests/codex")]
    pub root: PathBuf,

    /// Path to `RULES.json` (default: <root>/RULES.json).
    #[arg(long)]
    pub rules: Option<PathBuf>,

    /// Upstream Codex semantic version (e.g., 0.12.0).
    #[arg(long)]
    pub version: String,

    /// Desired status to materialize.
    #[arg(long, value_enum)]
    pub status: Status,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
#[clap(rename_all = "snake_case")]
pub enum Status {
    Snapshotted,
    Reported,
    Validated,
    Supported,
}

#[derive(Debug, Error)]
pub enum VersionMetadataError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("failed to parse JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid rules file: {0}")]
    Rules(String),
    #[error("missing required input file: {path}")]
    MissingInput { path: PathBuf },
    #[error(
        "invalid union snapshot kind in {path} (expected snapshot_schema_version=2, mode=union)"
    )]
    InvalidUnionKind { path: PathBuf },
    #[error("invalid wrapper coverage kind in {path} (expected schema_version=1)")]
    InvalidWrapperKind { path: PathBuf },
    #[error("cannot set status to {status}: {reason}")]
    Gate { status: String, reason: String },
}

#[derive(Debug, Deserialize)]
struct RulesFile {
    union: RulesUnion,
    version_metadata: RulesVersionMetadata,
    #[serde(default)]
    parity_exclusions: Option<RulesParityExclusions>,
}

#[derive(Debug, Deserialize)]
struct RulesUnion {
    required_target: String,
    expected_targets: Vec<String>,
    platform_mapping: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct RulesVersionMetadata {
    supported_policy: RulesSupportedPolicy,
}

#[derive(Debug, Deserialize)]
struct RulesSupportedPolicy {
    requires_union_complete: bool,
    requires_semantic_version: bool,
    coverage_requirement: RulesCoverageRequirement,
    intentionally_unsupported_requires_note: bool,
}

#[derive(Debug, Deserialize)]
struct RulesCoverageRequirement {
    allowed_levels: Vec<String>,
    disallowed_levels: Vec<String>,
    treat_missing_as: String,
}

#[derive(Debug, Deserialize)]
struct RulesParityExclusions {
    schema_version: u32,
    units: Vec<ParityExclusionUnit>,
}

#[derive(Debug, Deserialize)]
struct ParityExclusionUnit {
    unit: String,
    path: Vec<String>,
    #[serde(default)]
    key: Option<String>,
    #[serde(default)]
    name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct VersionMetadataV1 {
    schema_version: u32,
    semantic_version: String,
    status: String,
    updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    artifacts: Option<ArtifactsV1>,
    #[serde(skip_serializing_if = "Option::is_none")]
    coverage: Option<CoverageV1>,
    #[serde(skip_serializing_if = "Option::is_none")]
    validation: Option<ValidationV1>,
    #[serde(skip_serializing_if = "Option::is_none")]
    promotion: Option<PromotionV1>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ArtifactsV1 {
    #[serde(skip_serializing_if = "Option::is_none")]
    snapshots_dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reports_dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    union_complete: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct CoverageV1 {
    #[serde(skip_serializing_if = "Option::is_none")]
    supported_targets: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    supported_required_target: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ValidationV1 {
    #[serde(skip_serializing_if = "Option::is_none")]
    passed_targets: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    failed_targets: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    skipped_targets: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct PromotionV1 {
    #[serde(skip_serializing_if = "Option::is_none")]
    eligible_for_latest_validated: Option<bool>,
}

pub fn run(args: Args) -> Result<(), VersionMetadataError> {
    let root = fs::canonicalize(&args.root).unwrap_or(args.root.clone());
    let rules_path = args
        .rules
        .clone()
        .unwrap_or_else(|| root.join("RULES.json"));
    let rules: RulesFile = serde_json::from_slice(&fs::read(&rules_path)?)?;

    if rules.union.expected_targets.is_empty() {
        return Err(VersionMetadataError::Rules(
            "union.expected_targets must not be empty".to_string(),
        ));
    }

    if rules
        .version_metadata
        .supported_policy
        .requires_semantic_version
    {
        Version::parse(&args.version).map_err(|e| VersionMetadataError::Gate {
            status: args.status.to_string(),
            reason: format!("version is not a semantic version: {e}"),
        })?;
    }

    let union_path = root
        .join("snapshots")
        .join(&args.version)
        .join("union.json");
    if !union_path.is_file() {
        return Err(VersionMetadataError::MissingInput { path: union_path });
    }
    let union: UnionSnapshotV2 = serde_json::from_slice(&fs::read(&union_path)?)?;
    if union.snapshot_schema_version != 2 || union.mode != "union" {
        return Err(VersionMetadataError::InvalidUnionKind { path: union_path });
    }

    let any_report_path = root
        .join("reports")
        .join(&args.version)
        .join("coverage.any.json");
    if matches!(
        args.status,
        Status::Reported | Status::Validated | Status::Supported
    ) && !any_report_path.is_file()
    {
        return Err(VersionMetadataError::MissingInput {
            path: any_report_path,
        });
    }

    let wrapper_path = root.join("wrapper_coverage.json");
    let wrapper = if matches!(args.status, Status::Snapshotted) && !wrapper_path.is_file() {
        None
    } else {
        if !wrapper_path.is_file() {
            return Err(VersionMetadataError::MissingInput { path: wrapper_path });
        }
        let wc: WrapperCoverageV1 = serde_json::from_slice(&fs::read(&wrapper_path)?)?;
        if wc.schema_version != 1 {
            return Err(VersionMetadataError::InvalidWrapperKind { path: wrapper_path });
        }
        Some(wc)
    };

    let version_path = root.join("versions").join(format!("{}.json", args.version));
    let existing = read_existing_metadata(&version_path)?;

    let updated_at = deterministic_rfc3339_now();

    let artifacts = Some(ArtifactsV1 {
        snapshots_dir: Some(format!("snapshots/{}", args.version)),
        reports_dir: Some(format!("reports/{}", args.version)),
        union_complete: Some(union.complete),
    });

    let coverage = wrapper
        .as_ref()
        .map(|wc| compute_coverage(&rules, &union, wc))
        .transpose()?;

    let mut out = VersionMetadataV1 {
        schema_version: 1,
        semantic_version: args.version.clone(),
        status: args.status.to_string(),
        updated_at,
        notes: existing.as_ref().and_then(|m| m.notes.clone()),
        artifacts,
        coverage,
        validation: existing.as_ref().and_then(|m| m.validation.clone()),
        promotion: None,
    };

    enforce_gates(&rules, &union, &args, &out)?;

    out.promotion = Some(PromotionV1 {
        eligible_for_latest_validated: Some(matches!(args.status, Status::Validated)),
    });

    fs::create_dir_all(root.join("versions"))?;
    write_json_pretty(&version_path, &serde_json::to_string_pretty(&out)?)?;
    Ok(())
}

fn read_existing_metadata(path: &Path) -> Result<Option<VersionMetadataV1>, VersionMetadataError> {
    if !path.is_file() {
        return Ok(None);
    }
    let bytes = fs::read(path)?;
    let parsed: VersionMetadataV1 = serde_json::from_slice(&bytes)?;
    Ok(Some(parsed))
}

fn enforce_gates(
    rules: &RulesFile,
    union: &UnionSnapshotV2,
    args: &Args,
    meta: &VersionMetadataV1,
) -> Result<(), VersionMetadataError> {
    let supported_policy = &rules.version_metadata.supported_policy;

    match args.status {
        Status::Snapshotted => Ok(()),
        Status::Reported => Ok(()),
        Status::Validated => {
            let supported_required = meta
                .coverage
                .as_ref()
                .and_then(|c| c.supported_required_target)
                .unwrap_or(false);
            if !supported_required {
                return Err(VersionMetadataError::Gate {
                    status: args.status.to_string(),
                    reason: format!(
                        "supported_on_required_target=false (required_target={})",
                        rules.union.required_target
                    ),
                });
            }

            let passed_required = meta
                .validation
                .as_ref()
                .and_then(|v| v.passed_targets.as_ref())
                .is_some_and(|arr| arr.iter().any(|t| t == &rules.union.required_target));
            if !passed_required {
                return Err(VersionMetadataError::Gate {
                    status: args.status.to_string(),
                    reason: format!(
                        "validation_passed_on_required_target=false (required_target={})",
                        rules.union.required_target
                    ),
                });
            }
            Ok(())
        }
        Status::Supported => {
            if supported_policy.requires_union_complete && !union.complete {
                return Err(VersionMetadataError::Gate {
                    status: args.status.to_string(),
                    reason: "requires union.complete=true".to_string(),
                });
            }

            let expected = rules
                .union
                .expected_targets
                .iter()
                .cloned()
                .collect::<BTreeSet<_>>();
            let supported_targets = meta
                .coverage
                .as_ref()
                .and_then(|c| c.supported_targets.as_ref())
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .collect::<BTreeSet<_>>();

            if supported_targets != expected {
                return Err(VersionMetadataError::Gate {
                    status: args.status.to_string(),
                    reason: "supported_on_all_expected_targets=false".to_string(),
                });
            }

            Ok(())
        }
    }
}

fn deterministic_rfc3339_now() -> String {
    if let Ok(v) = std::env::var("SOURCE_DATE_EPOCH") {
        if let Ok(secs) = v.parse::<i64>() {
            if let Ok(ts) = OffsetDateTime::from_unix_timestamp(secs) {
                return ts
                    .format(&Rfc3339)
                    .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string());
            }
        }
    }
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

fn write_json_pretty(path: &Path, json: &str) -> Result<(), io::Error> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, format!("{json}\n"))?;
    Ok(())
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Status::Snapshotted => "snapshotted",
            Status::Reported => "reported",
            Status::Validated => "validated",
            Status::Supported => "supported",
        };
        write!(f, "{s}")
    }
}
