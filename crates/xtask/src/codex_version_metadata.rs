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

#[derive(Debug, Deserialize, Clone)]
struct UnionSnapshotV2 {
    snapshot_schema_version: u32,
    mode: String,
    complete: bool,
    inputs: Vec<UnionInputV2>,
    commands: Vec<UnionCommandV2>,
}

#[derive(Debug, Deserialize, Clone)]
struct UnionInputV2 {
    target_triple: String,
}

#[derive(Debug, Deserialize, Clone)]
struct UnionCommandV2 {
    path: Vec<String>,
    available_on: Vec<String>,
    #[serde(default)]
    flags: Vec<UnionFlagV2>,
    #[serde(default)]
    args: Vec<UnionArgV2>,
}

#[derive(Debug, Deserialize, Clone)]
struct UnionFlagV2 {
    key: String,
    available_on: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct UnionArgV2 {
    name: String,
    available_on: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct WrapperCoverageV1 {
    schema_version: u32,
    coverage: Vec<WrapperCommandCoverageV1>,
}

#[derive(Debug, Deserialize)]
struct WrapperCommandCoverageV1 {
    path: Vec<String>,
    level: String,
    note: Option<String>,
    scope: Option<WrapperScope>,
    #[serde(default)]
    flags: Vec<WrapperFlagCoverageV1>,
    #[serde(default)]
    args: Vec<WrapperArgCoverageV1>,
}

#[derive(Debug, Deserialize)]
struct WrapperFlagCoverageV1 {
    key: String,
    level: String,
    note: Option<String>,
    scope: Option<WrapperScope>,
}

#[derive(Debug, Deserialize)]
struct WrapperArgCoverageV1 {
    name: String,
    level: String,
    note: Option<String>,
    scope: Option<WrapperScope>,
}

#[derive(Debug, Deserialize, Clone)]
struct WrapperScope {
    platforms: Option<Vec<String>>,
    target_triples: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
struct ScopedCoverage {
    index: usize,
    targets: BTreeSet<String>,
    level: String,
    note: Option<String>,
}

#[derive(Debug, Clone)]
struct WrapperIndex {
    commands: BTreeMap<Vec<String>, Vec<ScopedCoverage>>,
    flags: BTreeMap<(Vec<String>, String), Vec<ScopedCoverage>>,
    args: BTreeMap<(Vec<String>, String), Vec<ScopedCoverage>>,
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

fn compute_coverage(
    rules: &RulesFile,
    union: &UnionSnapshotV2,
    wrapper: &WrapperCoverageV1,
) -> Result<CoverageV1, VersionMetadataError> {
    let input_targets: BTreeSet<String> = union
        .inputs
        .iter()
        .map(|i| i.target_triple.clone())
        .collect();

    let wrapper_index = index_wrapper(
        &rules.union.expected_targets,
        &rules.union.platform_mapping,
        wrapper,
    );

    let allowed: BTreeSet<&str> = rules
        .version_metadata
        .supported_policy
        .coverage_requirement
        .allowed_levels
        .iter()
        .map(|s| s.as_str())
        .collect();
    let disallowed: BTreeSet<&str> = rules
        .version_metadata
        .supported_policy
        .coverage_requirement
        .disallowed_levels
        .iter()
        .map(|s| s.as_str())
        .collect();

    let treat_missing_as = rules
        .version_metadata
        .supported_policy
        .coverage_requirement
        .treat_missing_as
        .as_str();

    let mut supported_targets = Vec::new();

    for target in &rules.union.expected_targets {
        if !input_targets.contains(target) {
            continue;
        }

        if is_supported_on_target(
            rules,
            union,
            &wrapper_index,
            target,
            &allowed,
            &disallowed,
            treat_missing_as,
        ) {
            supported_targets.push(target.clone());
        }
    }

    let supported_required_target = supported_targets
        .iter()
        .any(|t| t == &rules.union.required_target);

    Ok(CoverageV1 {
        supported_targets: Some(supported_targets),
        supported_required_target: Some(supported_required_target),
    })
}

fn is_supported_on_target(
    rules: &RulesFile,
    union: &UnionSnapshotV2,
    wrapper_index: &WrapperIndex,
    target: &str,
    allowed: &BTreeSet<&str>,
    disallowed: &BTreeSet<&str>,
    treat_missing_as: &str,
) -> bool {
    for cmd in &union.commands {
        if !cmd.available_on.iter().any(|t| t == target) {
            continue;
        }

        let cmd_level = resolve_level_for_target(
            wrapper_index
                .commands
                .get(&cmd.path)
                .map(Vec::as_slice)
                .unwrap_or(&[]),
            target,
        )
        .unwrap_or_else(|| treat_missing_as.to_string());

        if disallowed.contains(cmd_level.as_str()) || !allowed.contains(cmd_level.as_str()) {
            return false;
        }
        if cmd_level == "intentionally_unsupported"
            && rules
                .version_metadata
                .supported_policy
                .intentionally_unsupported_requires_note
        {
            let note_ok = resolve_note_for_target(
                wrapper_index
                    .commands
                    .get(&cmd.path)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]),
                target,
            )
            .is_some_and(|n| !n.trim().is_empty());
            if !note_ok {
                return false;
            }
        }

        for flag in &cmd.flags {
            if !flag.available_on.iter().any(|t| t == target) {
                continue;
            }
            let key = (cmd.path.clone(), flag.key.clone());
            let flag_level = resolve_level_for_target(
                wrapper_index
                    .flags
                    .get(&key)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]),
                target,
            )
            .unwrap_or_else(|| treat_missing_as.to_string());

            if disallowed.contains(flag_level.as_str()) || !allowed.contains(flag_level.as_str()) {
                return false;
            }
            if flag_level == "intentionally_unsupported"
                && rules
                    .version_metadata
                    .supported_policy
                    .intentionally_unsupported_requires_note
            {
                let note_ok = resolve_note_for_target(
                    wrapper_index
                        .flags
                        .get(&key)
                        .map(Vec::as_slice)
                        .unwrap_or(&[]),
                    target,
                )
                .is_some_and(|n| !n.trim().is_empty());
                if !note_ok {
                    return false;
                }
            }
        }

        for arg in &cmd.args {
            if !arg.available_on.iter().any(|t| t == target) {
                continue;
            }
            let key = (cmd.path.clone(), arg.name.clone());
            let arg_level = resolve_level_for_target(
                wrapper_index
                    .args
                    .get(&key)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]),
                target,
            )
            .unwrap_or_else(|| treat_missing_as.to_string());

            if disallowed.contains(arg_level.as_str()) || !allowed.contains(arg_level.as_str()) {
                return false;
            }
            if arg_level == "intentionally_unsupported"
                && rules
                    .version_metadata
                    .supported_policy
                    .intentionally_unsupported_requires_note
            {
                let note_ok = resolve_note_for_target(
                    wrapper_index
                        .args
                        .get(&key)
                        .map(Vec::as_slice)
                        .unwrap_or(&[]),
                    target,
                )
                .is_some_and(|n| !n.trim().is_empty());
                if !note_ok {
                    return false;
                }
            }
        }
    }
    true
}

fn resolve_level_for_target(entries: &[ScopedCoverage], target: &str) -> Option<String> {
    let mut levels = BTreeSet::<String>::new();
    for e in entries {
        if e.targets.contains(target) {
            levels.insert(e.level.clone());
        }
    }
    if levels.len() == 1 {
        levels.into_iter().next()
    } else {
        None
    }
}

fn resolve_note_for_target(entries: &[ScopedCoverage], target: &str) -> Option<String> {
    let mut by_index: BTreeMap<usize, String> = BTreeMap::new();
    for e in entries {
        if !e.targets.contains(target) {
            continue;
        }
        if let Some(note) = e.note.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            by_index.entry(e.index).or_insert_with(|| note.to_string());
        }
    }
    by_index.into_values().next()
}

fn index_wrapper(
    expected_targets: &[String],
    platform_mapping: &BTreeMap<String, String>,
    wrapper: &WrapperCoverageV1,
) -> WrapperIndex {
    let expected_set: BTreeSet<String> = expected_targets.iter().cloned().collect();

    let mut commands: BTreeMap<Vec<String>, Vec<ScopedCoverage>> = BTreeMap::new();
    let mut flags: BTreeMap<(Vec<String>, String), Vec<ScopedCoverage>> = BTreeMap::new();
    let mut args: BTreeMap<(Vec<String>, String), Vec<ScopedCoverage>> = BTreeMap::new();

    for (cmd_idx, cmd) in wrapper.coverage.iter().enumerate() {
        let cmd_targets = scope_to_targets(
            expected_targets,
            platform_mapping,
            &expected_set,
            cmd.scope.as_ref(),
        );
        commands
            .entry(cmd.path.clone())
            .or_default()
            .push(ScopedCoverage {
                index: cmd_idx,
                targets: cmd_targets.clone(),
                level: cmd.level.clone(),
                note: cmd.note.clone(),
            });

        for flag in &cmd.flags {
            let flag_targets = scope_to_targets(
                expected_targets,
                platform_mapping,
                &expected_set,
                flag.scope.as_ref(),
            );
            let effective = intersect(&cmd_targets, &flag_targets);
            flags
                .entry((cmd.path.clone(), flag.key.clone()))
                .or_default()
                .push(ScopedCoverage {
                    index: cmd_idx,
                    targets: effective,
                    level: flag.level.clone(),
                    note: flag.note.clone(),
                });
        }

        for arg in &cmd.args {
            let arg_targets = scope_to_targets(
                expected_targets,
                platform_mapping,
                &expected_set,
                arg.scope.as_ref(),
            );
            let effective = intersect(&cmd_targets, &arg_targets);
            args.entry((cmd.path.clone(), arg.name.clone()))
                .or_default()
                .push(ScopedCoverage {
                    index: cmd_idx,
                    targets: effective,
                    level: arg.level.clone(),
                    note: arg.note.clone(),
                });
        }
    }

    WrapperIndex {
        commands,
        flags,
        args,
    }
}

fn scope_to_targets(
    expected_targets: &[String],
    platform_mapping: &BTreeMap<String, String>,
    expected_set: &BTreeSet<String>,
    scope: Option<&WrapperScope>,
) -> BTreeSet<String> {
    let Some(scope) = scope else {
        return expected_set.clone();
    };

    let mut out = BTreeSet::<String>::new();
    if let Some(tt) = scope.target_triples.as_ref() {
        for t in tt {
            if expected_set.contains(t) {
                out.insert(t.clone());
            }
        }
    }
    if let Some(platforms) = scope.platforms.as_ref() {
        for target in expected_targets {
            if let Some(platform) = platform_mapping.get(target) {
                if platforms.iter().any(|pl| pl == platform) {
                    out.insert(target.clone());
                }
            }
        }
    }
    out
}

fn intersect(a: &BTreeSet<String>, b: &BTreeSet<String>) -> BTreeSet<String> {
    a.intersection(b).cloned().collect()
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
