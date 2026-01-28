use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use clap::Args;
use codex::wrapper_coverage_manifest::{
    wrapper_coverage_manifest, wrapper_crate_version, CoverageLevel, WrapperArgCoverageV1,
    WrapperCommandCoverageV1, WrapperCoverageManifestV1, WrapperFlagCoverageV1,
    WrapperSurfaceScopedTargets,
};
use serde::Deserialize;
use thiserror::Error;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

#[derive(Debug, Args)]
pub struct CliArgs {
    /// Output file path for `wrapper_coverage.json`.
    #[arg(long)]
    pub out: PathBuf,
    /// Path to RULES.json (used for expected target ordering + timestamp policy).
    #[arg(long, default_value = "cli_manifests/codex/RULES.json")]
    pub rules: PathBuf,
}

#[derive(Debug, Error)]
pub enum WrapperCoverageError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse rules.json: {0}")]
    RulesParse(#[from] serde_json::Error),
    #[error("unsupported rules: {0}")]
    RulesUnsupported(String),
    #[error("invalid wrapper coverage manifest: {0}")]
    ManifestInvalid(String),
}

#[derive(Debug, Deserialize)]
struct Rules {
    union: RulesUnion,
    sorting: RulesSorting,
    wrapper_coverage: RulesWrapperCoverage,
}

#[derive(Debug, Deserialize)]
struct RulesUnion {
    expected_targets: Vec<String>,
    platform_mapping: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct RulesSorting {
    commands: String,
    flags: String,
    args: String,
    expected_targets: String,
}

#[derive(Debug, Deserialize)]
struct RulesWrapperCoverage {
    scope_semantics: RulesWrapperScopeSemantics,
}

#[derive(Debug, Deserialize)]
struct RulesWrapperScopeSemantics {
    platforms_expand_to_expected_targets: bool,
    platforms_expand_using: String,
}

pub fn run(args: CliArgs) -> Result<(), WrapperCoverageError> {
    let rules: Rules = serde_json::from_slice(&fs::read(&args.rules)?)?;
    assert_supported_rules(&rules)?;

    let expected_targets = rules.union.expected_targets;
    let platform_mapping = rules.union.platform_mapping;
    let platform_to_targets = invert_platform_mapping(&expected_targets, &platform_mapping)?;

    let mut manifest: WrapperCoverageManifestV1 = wrapper_coverage_manifest();
    if manifest.schema_version != 1 {
        return Err(WrapperCoverageError::ManifestInvalid(format!(
            "schema_version must be 1 (got {})",
            manifest.schema_version
        )));
    }

    normalize_manifest(&mut manifest, &expected_targets, &platform_to_targets)?;

    manifest.generated_at = Some(deterministic_rfc3339_now());
    manifest.wrapper_version = Some(wrapper_crate_version().to_string());

    let json = serde_json::to_string_pretty(&manifest)
        .map_err(|e| WrapperCoverageError::ManifestInvalid(e.to_string()))?;
    write_json_pretty(&args.out, &json)?;
    Ok(())
}

fn assert_supported_rules(rules: &Rules) -> Result<(), WrapperCoverageError> {
    let mut unsupported = Vec::new();

    if rules.sorting.commands != "lexicographic_path" {
        unsupported.push(format!("sorting.commands={}", rules.sorting.commands));
    }
    if rules.sorting.flags != "by_key_then_long_then_short" {
        unsupported.push(format!("sorting.flags={}", rules.sorting.flags));
    }
    if rules.sorting.args != "by_name" {
        unsupported.push(format!("sorting.args={}", rules.sorting.args));
    }
    if rules.sorting.expected_targets != "rules_expected_targets_order" {
        unsupported.push(format!(
            "sorting.expected_targets={}",
            rules.sorting.expected_targets
        ));
    }

    if !rules
        .wrapper_coverage
        .scope_semantics
        .platforms_expand_to_expected_targets
    {
        unsupported.push(
            "wrapper_coverage.scope_semantics.platforms_expand_to_expected_targets=false"
                .to_string(),
        );
    }
    if rules
        .wrapper_coverage
        .scope_semantics
        .platforms_expand_using
        != "union.platform_mapping"
    {
        unsupported.push(format!(
            "wrapper_coverage.scope_semantics.platforms_expand_using={}",
            rules
                .wrapper_coverage
                .scope_semantics
                .platforms_expand_using
        ));
    }

    if unsupported.is_empty() {
        Ok(())
    } else {
        Err(WrapperCoverageError::RulesUnsupported(
            unsupported.join(", "),
        ))
    }
}

fn invert_platform_mapping(
    expected_targets: &[String],
    platform_mapping: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, Vec<String>>, WrapperCoverageError> {
    let mut out: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for target in expected_targets {
        let Some(platform) = platform_mapping.get(target) else {
            return Err(WrapperCoverageError::RulesUnsupported(format!(
                "union.platform_mapping missing expected target: {target}"
            )));
        };
        out.entry(platform.clone())
            .or_default()
            .push(target.clone());
    }
    Ok(out)
}

fn normalize_manifest(
    manifest: &mut WrapperCoverageManifestV1,
    expected_targets: &[String],
    platform_to_targets: &BTreeMap<String, Vec<String>>,
) -> Result<(), WrapperCoverageError> {
    for cmd in &mut manifest.coverage {
        normalize_command(cmd, expected_targets, platform_to_targets)?;
    }

    manifest.coverage.sort_by(|a, b| {
        command_sort_key(a, expected_targets).cmp(&command_sort_key(b, expected_targets))
    });
    Ok(())
}

fn normalize_command(
    cmd: &mut WrapperCommandCoverageV1,
    expected_targets: &[String],
    platform_to_targets: &BTreeMap<String, Vec<String>>,
) -> Result<(), WrapperCoverageError> {
    normalize_scope(&mut cmd.scope, expected_targets, platform_to_targets)?;

    let mut clear_flags = false;
    if let Some(flags) = cmd.flags.as_mut() {
        for flag in flags.iter_mut() {
            normalize_scope(&mut flag.scope, expected_targets, platform_to_targets)?;
        }
        flags.sort_by(|a, b| {
            flag_sort_key(a, expected_targets).cmp(&flag_sort_key(b, expected_targets))
        });
        clear_flags = flags.is_empty();
    }
    if clear_flags {
        cmd.flags = None;
    }

    let mut clear_args = false;
    if let Some(args) = cmd.args.as_mut() {
        for arg in args.iter_mut() {
            normalize_scope(&mut arg.scope, expected_targets, platform_to_targets)?;
        }
        args.sort_by(|a, b| {
            arg_sort_key(a, expected_targets).cmp(&arg_sort_key(b, expected_targets))
        });
        clear_args = args.is_empty();
    }
    if clear_args {
        cmd.args = None;
    }

    Ok(())
}

fn normalize_scope(
    scope: &mut Option<WrapperSurfaceScopedTargets>,
    expected_targets: &[String],
    platform_to_targets: &BTreeMap<String, Vec<String>>,
) -> Result<(), WrapperCoverageError> {
    let Some(s) = scope.as_mut() else {
        return Ok(());
    };

    let expected_set: BTreeSet<&str> = expected_targets.iter().map(|s| s.as_str()).collect();
    let mut targets: BTreeSet<String> = BTreeSet::new();

    if let Some(existing) = s.target_triples.as_deref() {
        for target in existing {
            if !expected_set.contains(target.as_str()) {
                return Err(WrapperCoverageError::ManifestInvalid(format!(
                    "scope target_triples contains non-expected target: {target}"
                )));
            }
        }
        targets.extend(existing.iter().cloned());
    }
    if let Some(platforms) = s.platforms.as_deref() {
        for platform in platforms {
            let Some(mapped) = platform_to_targets.get(platform) else {
                return Err(WrapperCoverageError::ManifestInvalid(format!(
                    "scope platforms contains unknown platform label: {platform}"
                )));
            };
            targets.extend(mapped.iter().cloned());
        }
    }

    if targets.is_empty() {
        *scope = None;
        return Ok(());
    }

    let mut target_triples: Vec<String> = targets.into_iter().collect();
    target_triples.sort_by(|a, b| {
        target_order_key(a, expected_targets).cmp(&target_order_key(b, expected_targets))
    });

    s.platforms = None;
    s.target_triples = Some(target_triples);
    Ok(())
}

fn target_order_key<'a>(target: &'a str, expected_targets: &[String]) -> (usize, &'a str) {
    (
        expected_targets
            .iter()
            .position(|t| t == target)
            .unwrap_or(usize::MAX),
        target,
    )
}

fn command_sort_key(
    cmd: &WrapperCommandCoverageV1,
    expected_targets: &[String],
) -> (Vec<String>, u8, String, u8, String) {
    let (scope_kind, scope_key) = scope_sort_key(cmd.scope.as_ref(), expected_targets);
    (
        cmd.path.clone(),
        scope_kind,
        scope_key,
        coverage_level_sort_key(cmd.level),
        cmd.note.clone().unwrap_or_default(),
    )
}

fn flag_sort_key(
    flag: &WrapperFlagCoverageV1,
    expected_targets: &[String],
) -> (String, u8, String, u8, String) {
    let (scope_kind, scope_key) = scope_sort_key(flag.scope.as_ref(), expected_targets);
    (
        flag.key.clone(),
        scope_kind,
        scope_key,
        coverage_level_sort_key(flag.level),
        flag.note.clone().unwrap_or_default(),
    )
}

fn arg_sort_key(
    arg: &WrapperArgCoverageV1,
    expected_targets: &[String],
) -> (String, u8, String, u8, String) {
    let (scope_kind, scope_key) = scope_sort_key(arg.scope.as_ref(), expected_targets);
    (
        arg.name.clone(),
        scope_kind,
        scope_key,
        coverage_level_sort_key(arg.level),
        arg.note.clone().unwrap_or_default(),
    )
}

fn scope_sort_key(
    scope: Option<&WrapperSurfaceScopedTargets>,
    expected_targets: &[String],
) -> (u8, String) {
    let Some(scope) = scope else {
        return (2, String::new());
    };

    let Some(targets) = scope.target_triples.as_deref() else {
        return (2, String::new());
    };

    let mut normalized: Vec<&str> = targets.iter().map(|s| s.as_str()).collect();
    normalized.sort_by(|a, b| {
        target_order_key(a, expected_targets).cmp(&target_order_key(b, expected_targets))
    });
    (0, normalized.join(","))
}

fn coverage_level_sort_key(level: CoverageLevel) -> u8 {
    match level {
        CoverageLevel::Explicit => 0,
        CoverageLevel::Passthrough => 1,
        CoverageLevel::IntentionallyUnsupported => 2,
        CoverageLevel::Unsupported => 3,
        CoverageLevel::Unknown => 4,
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

fn write_json_pretty(
    path: &Path,
    pretty_json_without_newline: &str,
) -> Result<(), WrapperCoverageError> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    fs::write(path, format!("{pretty_json_without_newline}\n"))?;
    Ok(())
}
