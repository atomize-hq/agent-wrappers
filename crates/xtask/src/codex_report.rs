use std::{
    collections::{BTreeMap, BTreeSet},
    fs, io,
    path::{Path, PathBuf},
};

use clap::Parser;
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
}

#[derive(Debug, Error)]
pub enum ReportError {
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
    #[error("unsupported or inconsistent wrapper coverage for {unit}: {detail}")]
    WrapperResolution { unit: String, detail: String },
}

#[derive(Debug, Deserialize)]
struct RulesFile {
    #[serde(rename = "rules_schema_version")]
    rules_schema_version: u32,
    union: RulesUnion,
    report: RulesReport,
    sorting: RulesSorting,
    #[serde(default)]
    parity_exclusions: Option<RulesParityExclusions>,
}

#[derive(Debug, Deserialize)]
struct RulesUnion {
    expected_targets: Vec<String>,
    platform_mapping: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct RulesSorting {
    report: RulesReportSorting,
}

#[derive(Debug, Deserialize)]
struct RulesReportSorting {
    missing_commands: String,
    missing_flags: String,
    missing_args: String,
    excluded_commands: String,
    excluded_flags: String,
    excluded_args: String,
    passthrough_candidates: String,
    unsupported: String,
    intentionally_unsupported: String,
    wrapper_only_commands: String,
    wrapper_only_flags: String,
    wrapper_only_args: String,
}

#[derive(Debug, Deserialize)]
struct RulesParityExclusions {
    schema_version: u32,
    units: Vec<ParityExclusionUnit>,
}

#[derive(Debug, Deserialize, Clone)]
struct ParityExclusionUnit {
    unit: String,
    path: Vec<String>,
    #[serde(default)]
    key: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    category: Option<String>,
    note: String,
}

#[derive(Debug, Deserialize)]
struct RulesReport {
    file_naming: RulesReportFileNaming,
    filter_semantics: RulesFilterSemantics,
}

#[derive(Debug, Deserialize)]
struct RulesReportFileNaming {
    any: String,
    all: String,
    per_target: String,
}

#[derive(Debug, Deserialize)]
struct RulesFilterSemantics {
    when_union_incomplete: RulesWhenUnionIncomplete,
}

#[derive(Debug, Deserialize)]
struct RulesWhenUnionIncomplete {
    all: String,
}

#[derive(Debug, Deserialize, Clone)]
struct UnionSnapshotV2 {
    snapshot_schema_version: u32,
    mode: String,
    complete: bool,
    expected_targets: Vec<String>,
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
    wrapper_version: Option<String>,
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
struct CoverageResolution {
    present: bool,
    targets: BTreeSet<String>,
    level: Option<String>,
    note: Option<String>,
}

#[derive(Debug, Clone, Copy)]
enum FilterMode<'a> {
    Any,
    All,
    ExactTarget(&'a str),
}

pub fn run(args: Args) -> Result<(), ReportError> {
    let root = fs::canonicalize(&args.root).unwrap_or(args.root.clone());
    let rules_path = args
        .rules
        .clone()
        .unwrap_or_else(|| root.join("RULES.json"));

    let rules: RulesFile = serde_json::from_slice(&fs::read(&rules_path)?)?;
    assert_supported_rules(&rules)?;
    let parity_exclusions = rules
        .parity_exclusions
        .as_ref()
        .filter(|ex| ex.schema_version == 1)
        .map(build_parity_exclusions_index);

    let union_path = root
        .join("snapshots")
        .join(&args.version)
        .join("union.json");
    if !union_path.is_file() {
        return Err(ReportError::MissingInput { path: union_path });
    }
    let union: UnionSnapshotV2 = serde_json::from_slice(&fs::read(&union_path)?)?;
    if union.snapshot_schema_version != 2 || union.mode != "union" {
        return Err(ReportError::InvalidUnionKind { path: union_path });
    }

    let wrapper_path = root.join("wrapper_coverage.json");
    if !wrapper_path.is_file() {
        return Err(ReportError::MissingInput { path: wrapper_path });
    }
    let wrapper: WrapperCoverageV1 = serde_json::from_slice(&fs::read(&wrapper_path)?)?;
    if wrapper.schema_version != 1 {
        return Err(ReportError::InvalidWrapperKind { path: wrapper_path });
    }

    let input_targets: Vec<String> = union
        .inputs
        .iter()
        .map(|i| i.target_triple.clone())
        .collect();
    if input_targets.is_empty() {
        return Err(ReportError::Rules(
            "union.inputs must not be empty".to_string(),
        ));
    }

    let upstream = index_upstream(&union);
    let wrapper_index = index_wrapper(
        &rules.union.expected_targets,
        &rules.union.platform_mapping,
        &wrapper,
    );

    let reports_dir = root.join("reports").join(&args.version);
    fs::create_dir_all(&reports_dir)?;

    require_source_date_epoch_if_ci()?;
    let generated_at = deterministic_rfc3339_now();

    // coverage.any.json (always)
    {
        let report = build_report(
            &rules,
            parity_exclusions.as_ref(),
            &args.version,
            "any",
            None,
            FilterMode::Any,
            &input_targets,
            &upstream,
            &wrapper,
            &wrapper_index,
            &generated_at,
        )?;
        let out_path = reports_dir.join(&rules.report.file_naming.any);
        write_json_pretty(&out_path, &serde_json::to_string_pretty(&report)?)?;
    }

    // coverage.<target_triple>.json (one per included input target)
    for target in &input_targets {
        let report = build_report(
            &rules,
            parity_exclusions.as_ref(),
            &args.version,
            "exact_target",
            Some(target.as_str()),
            FilterMode::ExactTarget(target),
            std::slice::from_ref(target),
            &upstream,
            &wrapper,
            &wrapper_index,
            &generated_at,
        )?;
        let filename = rules
            .report
            .file_naming
            .per_target
            .replace("<target_triple>", target);
        let out_path = reports_dir.join(filename);
        write_json_pretty(&out_path, &serde_json::to_string_pretty(&report)?)?;
    }

    // coverage.all.json (only when union.complete=true)
    if union.complete {
        let report = build_report(
            &rules,
            parity_exclusions.as_ref(),
            &args.version,
            "all",
            None,
            FilterMode::All,
            &union.expected_targets,
            &upstream,
            &wrapper,
            &wrapper_index,
            &generated_at,
        )?;
        let out_path = reports_dir.join(&rules.report.file_naming.all);
        write_json_pretty(&out_path, &serde_json::to_string_pretty(&report)?)?;
    }

    Ok(())
}

fn assert_supported_rules(rules: &RulesFile) -> Result<(), ReportError> {
    let mut unsupported = Vec::new();

    if rules.report.file_naming.any != "coverage.any.json" {
        unsupported.push(format!(
            "report.file_naming.any={}",
            rules.report.file_naming.any
        ));
    }
    if rules.report.file_naming.all != "coverage.all.json" {
        unsupported.push(format!(
            "report.file_naming.all={}",
            rules.report.file_naming.all
        ));
    }
    if rules.report.file_naming.per_target != "coverage.<target_triple>.json" {
        unsupported.push(format!(
            "report.file_naming.per_target={}",
            rules.report.file_naming.per_target
        ));
    }

    if rules.sorting.report.missing_commands != "by_path" {
        unsupported.push(format!(
            "sorting.report.missing_commands={}",
            rules.sorting.report.missing_commands
        ));
    }
    if rules.sorting.report.missing_flags != "by_path_then_key" {
        unsupported.push(format!(
            "sorting.report.missing_flags={}",
            rules.sorting.report.missing_flags
        ));
    }
    if rules.sorting.report.missing_args != "by_path_then_name" {
        unsupported.push(format!(
            "sorting.report.missing_args={}",
            rules.sorting.report.missing_args
        ));
    }
    if rules.sorting.report.excluded_commands != "by_path" {
        unsupported.push(format!(
            "sorting.report.excluded_commands={}",
            rules.sorting.report.excluded_commands
        ));
    }
    if rules.sorting.report.excluded_flags != "by_path_then_key" {
        unsupported.push(format!(
            "sorting.report.excluded_flags={}",
            rules.sorting.report.excluded_flags
        ));
    }
    if rules.sorting.report.excluded_args != "by_path_then_name" {
        unsupported.push(format!(
            "sorting.report.excluded_args={}",
            rules.sorting.report.excluded_args
        ));
    }
    if rules.sorting.report.passthrough_candidates != "by_path" {
        unsupported.push(format!(
            "sorting.report.passthrough_candidates={}",
            rules.sorting.report.passthrough_candidates
        ));
    }
    if rules.sorting.report.unsupported != "by_path" {
        unsupported.push(format!(
            "sorting.report.unsupported={}",
            rules.sorting.report.unsupported
        ));
    }
    if rules.sorting.report.intentionally_unsupported != "by_kind_then_path_then_key_or_name" {
        unsupported.push(format!(
            "sorting.report.intentionally_unsupported={}",
            rules.sorting.report.intentionally_unsupported
        ));
    }
    if rules.sorting.report.wrapper_only_commands != "by_path" {
        unsupported.push(format!(
            "sorting.report.wrapper_only_commands={}",
            rules.sorting.report.wrapper_only_commands
        ));
    }
    if rules.sorting.report.wrapper_only_flags != "by_path_then_key" {
        unsupported.push(format!(
            "sorting.report.wrapper_only_flags={}",
            rules.sorting.report.wrapper_only_flags
        ));
    }
    if rules.sorting.report.wrapper_only_args != "by_path_then_name" {
        unsupported.push(format!(
            "sorting.report.wrapper_only_args={}",
            rules.sorting.report.wrapper_only_args
        ));
    }

    if rules.report.filter_semantics.when_union_incomplete.all != "error" {
        unsupported.push(format!(
            "report.filter_semantics.when_union_incomplete.all={}",
            rules.report.filter_semantics.when_union_incomplete.all
        ));
    }

    if rules.union.expected_targets.is_empty() {
        unsupported.push("union.expected_targets must not be empty".to_string());
    }

    if unsupported.is_empty() {
        Ok(())
    } else {
        Err(ReportError::Rules(format!(
            "unsupported rules: {}",
            unsupported.join(", ")
        )))
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

fn require_source_date_epoch_if_ci() -> Result<(), ReportError> {
    if std::env::var("CI").is_err() {
        return Ok(());
    }
    if std::env::var("SOURCE_DATE_EPOCH").is_err() {
        return Err(ReportError::Rules(
            "CI requires SOURCE_DATE_EPOCH for deterministic generated_at".to_string(),
        ));
    }
    Ok(())
}

fn write_json_pretty(path: &Path, json: &str) -> Result<(), io::Error> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, format!("{json}\n"))?;
    Ok(())
}

fn index_upstream(union: &UnionSnapshotV2) -> BTreeMap<Vec<String>, UnionCommandV2> {
    let mut out = BTreeMap::new();
    for cmd in &union.commands {
        out.insert(cmd.path.clone(), cmd.clone());
    }
    out
}

#[derive(Debug, Clone)]
struct WrapperIndex {
    commands: BTreeMap<Vec<String>, Vec<ScopedCoverage>>,
    flags: BTreeMap<(Vec<String>, String), Vec<ScopedCoverage>>,
    args: BTreeMap<(Vec<String>, String), Vec<ScopedCoverage>>,
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

#[allow(clippy::too_many_arguments)]
fn build_report(
    rules: &RulesFile,
    parity_exclusions: Option<&ParityExclusionsIndex>,
    version: &str,
    platform_mode: &str,
    target_triple: Option<&str>,
    filter_mode: FilterMode<'_>,
    report_targets: &[String],
    upstream: &BTreeMap<Vec<String>, UnionCommandV2>,
    wrapper: &WrapperCoverageV1,
    wrapper_index: &WrapperIndex,
    generated_at: &str,
) -> Result<CoverageReportV1, ReportError> {
    let report_target_set: BTreeSet<String> = report_targets.iter().cloned().collect();
    let expected_set: BTreeSet<String> = rules.union.expected_targets.iter().cloned().collect();
    let iu_roots = build_iu_roots(
        wrapper,
        wrapper_index,
        &report_target_set,
        &rules.union.expected_targets,
        filter_mode,
    )?;

    if matches!(filter_mode, FilterMode::All)
        && !expected_set.is_subset(&report_target_set)
        && rules.report.filter_semantics.when_union_incomplete.all == "error"
    {
        return Err(ReportError::Rules(
            "cannot generate platform_filter.mode=all with an incomplete union target set"
                .to_string(),
        ));
    }

    let mut missing_commands: Vec<ReportCommandDeltaV1> = Vec::new();
    let mut missing_flags: Vec<ReportFlagDeltaV1> = Vec::new();
    let mut missing_args: Vec<ReportArgDeltaV1> = Vec::new();

    let mut excluded_commands: Vec<ReportCommandDeltaV1> = Vec::new();
    let mut excluded_flags: Vec<ReportFlagDeltaV1> = Vec::new();
    let mut excluded_args: Vec<ReportArgDeltaV1> = Vec::new();

    let mut passthrough_candidates: Vec<ReportCommandDeltaV1> = Vec::new();
    let mut unsupported: Vec<ReportCommandDeltaV1> = Vec::new();
    let mut intentionally_unsupported: Vec<ReportIntentionallyUnsupportedDeltaV1> = Vec::new();
    let mut wrapper_only_commands: Vec<ReportCommandDeltaV1> = Vec::new();
    let mut wrapper_only_flags: Vec<ReportFlagDeltaV1> = Vec::new();
    let mut wrapper_only_args: Vec<ReportArgDeltaV1> = Vec::new();

    // Upstream → missing/unsupported/iu/passthrough
    for (path, cmd) in upstream {
        if !present_on_filter(
            &cmd.available_on,
            &report_target_set,
            &rules.union.expected_targets,
            filter_mode,
        ) {
            continue;
        }

        if let Some(ex) = parity_exclusions.and_then(|idx| idx.commands.get(path)) {
            let cmd_res = resolve_wrapper(
                wrapper_index
                    .commands
                    .get(path)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]),
                &report_target_set,
                &rules.union.expected_targets,
                filter_mode,
                "command",
                &format!("path={}", format_path(path)),
            )?;
            excluded_commands.push(ReportCommandDeltaV1 {
                path: path.clone(),
                upstream_available_on: cmd.available_on.clone(),
                wrapper_level: cmd_res.level.clone(),
                note: Some(ex.note.clone()),
            });
            continue;
        }

        let cmd_res = resolve_wrapper(
            wrapper_index
                .commands
                .get(path)
                .map(Vec::as_slice)
                .unwrap_or(&[]),
            &report_target_set,
            &rules.union.expected_targets,
            filter_mode,
            "command",
            &format!("path={}", format_path(path)),
        )?;

        if cmd_res.level.is_none() {
            if let Some(root) = find_inherited_iu_root(
                &iu_roots,
                path,
                &cmd.available_on,
                &report_target_set,
                "command",
            )? {
                intentionally_unsupported.push(ReportIntentionallyUnsupportedDeltaV1::Command(
                    ReportCommandDeltaV1 {
                        path: path.clone(),
                        upstream_available_on: cmd.available_on.clone(),
                        wrapper_level: Some("intentionally_unsupported".to_string()),
                        note: Some(root.note.clone()),
                    },
                ));
            } else {
                classify_command_delta(
                    &mut missing_commands,
                    &mut passthrough_candidates,
                    &mut unsupported,
                    path,
                    &cmd.available_on,
                    &cmd_res,
                );
            }
        } else if cmd_res.level.as_deref() == Some("intentionally_unsupported") {
            let note = require_non_empty_note(
                cmd_res.note.as_deref(),
                "command",
                &format!("path={}", format_path(path)),
            )?;
            intentionally_unsupported.push(ReportIntentionallyUnsupportedDeltaV1::Command(
                ReportCommandDeltaV1 {
                    path: path.clone(),
                    upstream_available_on: cmd.available_on.clone(),
                    wrapper_level: Some("intentionally_unsupported".to_string()),
                    note: Some(note),
                },
            ));
        } else {
            classify_command_delta(
                &mut missing_commands,
                &mut passthrough_candidates,
                &mut unsupported,
                path,
                &cmd.available_on,
                &cmd_res,
            );
        }

        for flag in &cmd.flags {
            if !present_on_filter(
                &flag.available_on,
                &report_target_set,
                &rules.union.expected_targets,
                filter_mode,
            ) {
                continue;
            }
            if let Some(ex) =
                parity_exclusions.and_then(|idx| idx.flags.get(&(path.clone(), flag.key.clone())))
            {
                let key = (path.clone(), flag.key.clone());
                let res = resolve_wrapper(
                    wrapper_index
                        .flags
                        .get(&key)
                        .map(Vec::as_slice)
                        .unwrap_or(&[]),
                    &report_target_set,
                    &rules.union.expected_targets,
                    filter_mode,
                    "flag",
                    &format!("path={} key={}", format_path(path), flag.key),
                )?;
                excluded_flags.push(ReportFlagDeltaV1 {
                    path: path.clone(),
                    key: flag.key.clone(),
                    upstream_available_on: flag.available_on.clone(),
                    wrapper_level: res.level.clone(),
                    note: Some(ex.note.clone()),
                });
                continue;
            }
            let key = (path.clone(), flag.key.clone());
            let res = resolve_wrapper(
                wrapper_index
                    .flags
                    .get(&key)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]),
                &report_target_set,
                &rules.union.expected_targets,
                filter_mode,
                "flag",
                &format!("path={} key={}", format_path(path), flag.key),
            )?;
            if res.level.is_none() {
                if let Some(root) = find_inherited_iu_root(
                    &iu_roots,
                    path,
                    &flag.available_on,
                    &report_target_set,
                    "flag",
                )? {
                    intentionally_unsupported.push(ReportIntentionallyUnsupportedDeltaV1::Flag(
                        ReportFlagDeltaV1 {
                            path: path.to_vec(),
                            key: flag.key.clone(),
                            upstream_available_on: flag.available_on.clone(),
                            wrapper_level: Some("intentionally_unsupported".to_string()),
                            note: Some(root.note.clone()),
                        },
                    ));
                } else {
                    classify_flag_delta(&mut missing_flags, path, flag, &res);
                }
            } else if res.level.as_deref() == Some("intentionally_unsupported") {
                let note = require_non_empty_note(
                    res.note.as_deref(),
                    "flag",
                    &format!("path={} key={}", format_path(path), flag.key),
                )?;
                intentionally_unsupported.push(ReportIntentionallyUnsupportedDeltaV1::Flag(
                    ReportFlagDeltaV1 {
                        path: path.to_vec(),
                        key: flag.key.clone(),
                        upstream_available_on: flag.available_on.clone(),
                        wrapper_level: Some("intentionally_unsupported".to_string()),
                        note: Some(note),
                    },
                ));
            } else {
                classify_flag_delta(&mut missing_flags, path, flag, &res);
            }
        }

        for arg in &cmd.args {
            if !present_on_filter(
                &arg.available_on,
                &report_target_set,
                &rules.union.expected_targets,
                filter_mode,
            ) {
                continue;
            }
            if let Some(ex) =
                parity_exclusions.and_then(|idx| idx.args.get(&(path.clone(), arg.name.clone())))
            {
                let key = (path.clone(), arg.name.clone());
                let res = resolve_wrapper(
                    wrapper_index
                        .args
                        .get(&key)
                        .map(Vec::as_slice)
                        .unwrap_or(&[]),
                    &report_target_set,
                    &rules.union.expected_targets,
                    filter_mode,
                    "arg",
                    &format!("path={} name={}", format_path(path), arg.name),
                )?;
                excluded_args.push(ReportArgDeltaV1 {
                    path: path.clone(),
                    name: arg.name.clone(),
                    upstream_available_on: arg.available_on.clone(),
                    wrapper_level: res.level.clone(),
                    note: Some(ex.note.clone()),
                });
                continue;
            }
            let key = (path.clone(), arg.name.clone());
            let res = resolve_wrapper(
                wrapper_index
                    .args
                    .get(&key)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]),
                &report_target_set,
                &rules.union.expected_targets,
                filter_mode,
                "arg",
                &format!("path={} name={}", format_path(path), arg.name),
            )?;
            if res.level.is_none() {
                if let Some(root) = find_inherited_iu_root(
                    &iu_roots,
                    path,
                    &arg.available_on,
                    &report_target_set,
                    "arg",
                )? {
                    intentionally_unsupported.push(ReportIntentionallyUnsupportedDeltaV1::Arg(
                        ReportArgDeltaV1 {
                            path: path.to_vec(),
                            name: arg.name.clone(),
                            upstream_available_on: arg.available_on.clone(),
                            wrapper_level: Some("intentionally_unsupported".to_string()),
                            note: Some(root.note.clone()),
                        },
                    ));
                } else {
                    classify_arg_delta(&mut missing_args, path, arg, &res);
                }
            } else if res.level.as_deref() == Some("intentionally_unsupported") {
                let note = require_non_empty_note(
                    res.note.as_deref(),
                    "arg",
                    &format!("path={} name={}", format_path(path), arg.name),
                )?;
                intentionally_unsupported.push(ReportIntentionallyUnsupportedDeltaV1::Arg(
                    ReportArgDeltaV1 {
                        path: path.to_vec(),
                        name: arg.name.clone(),
                        upstream_available_on: arg.available_on.clone(),
                        wrapper_level: Some("intentionally_unsupported".to_string()),
                        note: Some(note),
                    },
                ));
            } else {
                classify_arg_delta(&mut missing_args, path, arg, &res);
            }
        }
    }

    // Wrapper → wrapper-only (relative to platform filter semantics)
    for (path, entries) in &wrapper_index.commands {
        let res = resolve_wrapper(
            entries,
            &report_target_set,
            &rules.union.expected_targets,
            filter_mode,
            "command",
            &format!("path={}", format_path(path)),
        )?;
        if !res.present {
            continue;
        }

        let upstream_avail = upstream
            .get(path)
            .map(|c| c.available_on.clone())
            .unwrap_or_else(|| ordered_subset(&rules.union.expected_targets, &res.targets));
        let upstream_present = upstream.get(path).is_some_and(|c| {
            present_on_filter(
                &c.available_on,
                &report_target_set,
                &rules.union.expected_targets,
                filter_mode,
            )
        });

        if !upstream_present {
            wrapper_only_commands.push(ReportCommandDeltaV1 {
                path: path.clone(),
                upstream_available_on: upstream_avail,
                wrapper_level: res.level.clone(),
                note: res.note.clone(),
            });
        }
    }

    for ((path, key), entries) in &wrapper_index.flags {
        let res = resolve_wrapper(
            entries,
            &report_target_set,
            &rules.union.expected_targets,
            filter_mode,
            "flag",
            &format!("path={} key={key}", format_path(path)),
        )?;
        if !res.present {
            continue;
        }
        let (upstream_avail, upstream_present) = upstream_flag_availability(
            upstream,
            path,
            key,
            &res,
            &report_target_set,
            &rules.union.expected_targets,
            filter_mode,
        );
        if !upstream_present {
            wrapper_only_flags.push(ReportFlagDeltaV1 {
                path: path.clone(),
                key: key.clone(),
                upstream_available_on: upstream_avail,
                wrapper_level: res.level.clone(),
                note: res.note.clone(),
            });
        }
    }

    for ((path, name), entries) in &wrapper_index.args {
        let res = resolve_wrapper(
            entries,
            &report_target_set,
            &rules.union.expected_targets,
            filter_mode,
            "arg",
            &format!("path={} name={name}", format_path(path)),
        )?;
        if !res.present {
            continue;
        }
        let (upstream_avail, upstream_present) = upstream_arg_availability(
            upstream,
            path,
            name,
            &res,
            &report_target_set,
            &rules.union.expected_targets,
            filter_mode,
        );
        if !upstream_present {
            wrapper_only_args.push(ReportArgDeltaV1 {
                path: path.clone(),
                name: name.clone(),
                upstream_available_on: upstream_avail,
                wrapper_level: res.level.clone(),
                note: res.note.clone(),
            });
        }
    }

    missing_commands.sort_by(|a, b| cmp_path(&a.path, &b.path));
    missing_flags.sort_by(|a, b| cmp_path(&a.path, &b.path).then_with(|| a.key.cmp(&b.key)));
    missing_args.sort_by(|a, b| cmp_path(&a.path, &b.path).then_with(|| a.name.cmp(&b.name)));

    excluded_commands.sort_by(|a, b| cmp_path(&a.path, &b.path));
    excluded_flags.sort_by(|a, b| cmp_path(&a.path, &b.path).then_with(|| a.key.cmp(&b.key)));
    excluded_args.sort_by(|a, b| cmp_path(&a.path, &b.path).then_with(|| a.name.cmp(&b.name)));

    passthrough_candidates.sort_by(|a, b| cmp_path(&a.path, &b.path));
    unsupported.sort_by(|a, b| cmp_path(&a.path, &b.path));
    intentionally_unsupported.sort_by(cmp_iu_delta);

    wrapper_only_commands.sort_by(|a, b| cmp_path(&a.path, &b.path));
    wrapper_only_flags.sort_by(|a, b| cmp_path(&a.path, &b.path).then_with(|| a.key.cmp(&b.key)));
    wrapper_only_args.sort_by(|a, b| cmp_path(&a.path, &b.path).then_with(|| a.name.cmp(&b.name)));

    let deltas = ReportDeltasV1 {
        missing_commands,
        missing_flags,
        missing_args,
        excluded_commands: if excluded_commands.is_empty() {
            None
        } else {
            Some(excluded_commands)
        },
        excluded_flags: if excluded_flags.is_empty() {
            None
        } else {
            Some(excluded_flags)
        },
        excluded_args: if excluded_args.is_empty() {
            None
        } else {
            Some(excluded_args)
        },
        passthrough_candidates: if passthrough_candidates.is_empty() {
            None
        } else {
            Some(passthrough_candidates)
        },
        unsupported: if unsupported.is_empty() {
            None
        } else {
            Some(unsupported)
        },
        intentionally_unsupported: if intentionally_unsupported.is_empty() {
            None
        } else {
            Some(intentionally_unsupported)
        },
        wrapper_only_commands: if wrapper_only_commands.is_empty() {
            None
        } else {
            Some(wrapper_only_commands)
        },
        wrapper_only_flags: if wrapper_only_flags.is_empty() {
            None
        } else {
            Some(wrapper_only_flags)
        },
        wrapper_only_args: if wrapper_only_args.is_empty() {
            None
        } else {
            Some(wrapper_only_args)
        },
    };

    Ok(CoverageReportV1 {
        schema_version: 1,
        generated_at: generated_at.to_string(),
        inputs: ReportInputsV1 {
            upstream: ReportUpstreamInputsV1 {
                semantic_version: version.to_string(),
                mode: "union".to_string(),
                targets: report_targets.to_vec(),
            },
            wrapper: ReportWrapperInputsV1 {
                schema_version: wrapper.schema_version,
                wrapper_version: wrapper.wrapper_version.clone(),
            },
            rules: ReportRulesInputsV1 {
                rules_schema_version: rules.rules_schema_version,
            },
        },
        platform_filter: PlatformFilterV1 {
            mode: platform_mode.to_string(),
            target_triple: target_triple.map(ToString::to_string),
        },
        deltas,
    })
}

fn present_on_filter(
    available_on: &[String],
    report_targets: &BTreeSet<String>,
    expected_targets: &[String],
    mode: FilterMode<'_>,
) -> bool {
    match mode {
        FilterMode::Any => available_on.iter().any(|t| report_targets.contains(t)),
        FilterMode::ExactTarget(t) => available_on.iter().any(|x| x == t),
        FilterMode::All => expected_targets
            .iter()
            .all(|t| available_on.iter().any(|x| x == t)),
    }
}

fn resolve_wrapper(
    entries: &[ScopedCoverage],
    report_targets: &BTreeSet<String>,
    expected_targets: &[String],
    mode: FilterMode<'_>,
    unit: &str,
    detail: &str,
) -> Result<CoverageResolution, ReportError> {
    let relevant_target_set: BTreeSet<String> = match mode {
        FilterMode::Any => report_targets.clone(),
        FilterMode::ExactTarget(t) => BTreeSet::from([t.to_string()]),
        FilterMode::All => expected_targets.iter().cloned().collect(),
    };

    let mut union_targets = BTreeSet::<String>::new();
    let mut levels = BTreeSet::<String>::new();
    let mut note_by_index: BTreeMap<usize, String> = BTreeMap::new();

    for e in entries {
        let intersection: BTreeSet<String> = e
            .targets
            .intersection(&relevant_target_set)
            .cloned()
            .collect();
        if intersection.is_empty() {
            continue;
        }
        union_targets.extend(intersection);
        levels.insert(e.level.clone());
        if let Some(note) = e.note.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            note_by_index
                .entry(e.index)
                .or_insert_with(|| note.to_string());
        }
    }

    let present = match mode {
        FilterMode::Any => !union_targets.is_empty(),
        FilterMode::ExactTarget(t) => union_targets.contains(t),
        FilterMode::All => expected_targets.iter().all(|t| union_targets.contains(t)),
    };

    let level = match levels.len() {
        0 => None,
        1 => levels.into_iter().next(),
        _ => {
            return Err(ReportError::WrapperResolution {
                unit: unit.to_string(),
                detail: format!("{detail} has multiple wrapper levels across relevant scopes"),
            })
        }
    };

    let note = note_by_index.into_values().next();

    Ok(CoverageResolution {
        present,
        targets: union_targets,
        level,
        note,
    })
}

fn classify_command_delta(
    missing: &mut Vec<ReportCommandDeltaV1>,
    passthrough_candidates: &mut Vec<ReportCommandDeltaV1>,
    unsupported: &mut Vec<ReportCommandDeltaV1>,
    path: &[String],
    upstream_available_on: &[String],
    wrapper: &CoverageResolution,
) {
    let entry = ReportCommandDeltaV1 {
        path: path.to_vec(),
        upstream_available_on: upstream_available_on.to_vec(),
        wrapper_level: wrapper.level.clone(),
        note: wrapper.note.clone(),
    };

    match wrapper.level.as_deref() {
        None => missing.push(entry),
        Some("unknown") => missing.push(entry),
        Some("unsupported") => unsupported.push(entry),
        Some("intentionally_unsupported") => {}
        Some("passthrough") => passthrough_candidates.push(entry),
        Some("explicit") => {}
        Some(other) => missing.push(ReportCommandDeltaV1 {
            wrapper_level: Some(other.to_string()),
            ..entry
        }),
    }
}

fn classify_flag_delta(
    out: &mut Vec<ReportFlagDeltaV1>,
    path: &[String],
    flag: &UnionFlagV2,
    wrapper: &CoverageResolution,
) {
    match wrapper.level.as_deref() {
        None | Some("unknown") | Some("unsupported") => out.push(ReportFlagDeltaV1 {
            path: path.to_vec(),
            key: flag.key.clone(),
            upstream_available_on: flag.available_on.clone(),
            wrapper_level: wrapper.level.clone(),
            note: wrapper.note.clone(),
        }),
        Some("intentionally_unsupported") => {}
        Some("explicit") | Some("passthrough") => {}
        Some(other) => out.push(ReportFlagDeltaV1 {
            path: path.to_vec(),
            key: flag.key.clone(),
            upstream_available_on: flag.available_on.clone(),
            wrapper_level: Some(other.to_string()),
            note: wrapper.note.clone(),
        }),
    }
}

fn classify_arg_delta(
    out: &mut Vec<ReportArgDeltaV1>,
    path: &[String],
    arg: &UnionArgV2,
    wrapper: &CoverageResolution,
) {
    match wrapper.level.as_deref() {
        None | Some("unknown") | Some("unsupported") => out.push(ReportArgDeltaV1 {
            path: path.to_vec(),
            name: arg.name.clone(),
            upstream_available_on: arg.available_on.clone(),
            wrapper_level: wrapper.level.clone(),
            note: wrapper.note.clone(),
        }),
        Some("intentionally_unsupported") => {}
        Some("explicit") | Some("passthrough") => {}
        Some(other) => out.push(ReportArgDeltaV1 {
            path: path.to_vec(),
            name: arg.name.clone(),
            upstream_available_on: arg.available_on.clone(),
            wrapper_level: Some(other.to_string()),
            note: wrapper.note.clone(),
        }),
    }
}

fn upstream_flag_availability(
    upstream: &BTreeMap<Vec<String>, UnionCommandV2>,
    path: &[String],
    key: &str,
    wrapper_res: &CoverageResolution,
    report_targets: &BTreeSet<String>,
    expected_targets: &[String],
    mode: FilterMode<'_>,
) -> (Vec<String>, bool) {
    if let Some(cmd) = upstream.get(path) {
        if let Some(flag) = cmd.flags.iter().find(|f| f.key == key) {
            let present =
                present_on_filter(&flag.available_on, report_targets, expected_targets, mode);
            return (flag.available_on.clone(), present);
        }
        return (cmd.available_on.clone(), false);
    }
    (
        ordered_subset(expected_targets, &wrapper_res.targets),
        false,
    )
}

fn upstream_arg_availability(
    upstream: &BTreeMap<Vec<String>, UnionCommandV2>,
    path: &[String],
    name: &str,
    wrapper_res: &CoverageResolution,
    report_targets: &BTreeSet<String>,
    expected_targets: &[String],
    mode: FilterMode<'_>,
) -> (Vec<String>, bool) {
    if let Some(cmd) = upstream.get(path) {
        if let Some(arg) = cmd.args.iter().find(|a| a.name == name) {
            let present =
                present_on_filter(&arg.available_on, report_targets, expected_targets, mode);
            return (arg.available_on.clone(), present);
        }
        return (cmd.available_on.clone(), false);
    }
    (
        ordered_subset(expected_targets, &wrapper_res.targets),
        false,
    )
}

fn ordered_subset(expected_targets: &[String], targets: &BTreeSet<String>) -> Vec<String> {
    expected_targets
        .iter()
        .filter(|t| targets.contains(*t))
        .cloned()
        .collect()
}

fn cmp_path(a: &[String], b: &[String]) -> std::cmp::Ordering {
    let mut i = 0usize;
    while i < a.len() && i < b.len() {
        match a[i].cmp(&b[i]) {
            std::cmp::Ordering::Equal => i += 1,
            non_eq => return non_eq,
        }
    }
    a.len().cmp(&b.len())
}

fn format_path(path: &[String]) -> String {
    if path.is_empty() {
        "<root>".to_string()
    } else {
        path.join(" ")
    }
}

#[derive(Debug, Serialize)]
struct CoverageReportV1 {
    schema_version: u32,
    generated_at: String,
    inputs: ReportInputsV1,
    platform_filter: PlatformFilterV1,
    deltas: ReportDeltasV1,
}

#[derive(Debug, Serialize)]
struct ReportInputsV1 {
    upstream: ReportUpstreamInputsV1,
    wrapper: ReportWrapperInputsV1,
    rules: ReportRulesInputsV1,
}

#[derive(Debug, Serialize)]
struct ReportUpstreamInputsV1 {
    semantic_version: String,
    mode: String,
    targets: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ReportWrapperInputsV1 {
    schema_version: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    wrapper_version: Option<String>,
}

#[derive(Debug, Serialize)]
struct ReportRulesInputsV1 {
    rules_schema_version: u32,
}

#[derive(Debug, Serialize)]
struct PlatformFilterV1 {
    mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    target_triple: Option<String>,
}

#[derive(Debug, Serialize)]
struct ReportDeltasV1 {
    missing_commands: Vec<ReportCommandDeltaV1>,
    missing_flags: Vec<ReportFlagDeltaV1>,
    missing_args: Vec<ReportArgDeltaV1>,

    #[serde(skip_serializing_if = "Option::is_none")]
    excluded_commands: Option<Vec<ReportCommandDeltaV1>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    excluded_flags: Option<Vec<ReportFlagDeltaV1>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    excluded_args: Option<Vec<ReportArgDeltaV1>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    passthrough_candidates: Option<Vec<ReportCommandDeltaV1>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    unsupported: Option<Vec<ReportCommandDeltaV1>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    intentionally_unsupported: Option<Vec<ReportIntentionallyUnsupportedDeltaV1>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    wrapper_only_commands: Option<Vec<ReportCommandDeltaV1>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    wrapper_only_flags: Option<Vec<ReportFlagDeltaV1>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    wrapper_only_args: Option<Vec<ReportArgDeltaV1>>,
}

#[derive(Debug, Serialize)]
struct ReportCommandDeltaV1 {
    path: Vec<String>,
    upstream_available_on: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    wrapper_level: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    note: Option<String>,
}

#[derive(Debug, Serialize)]
struct ReportFlagDeltaV1 {
    path: Vec<String>,
    key: String,
    upstream_available_on: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    wrapper_level: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    note: Option<String>,
}

#[derive(Debug, Serialize)]
struct ReportArgDeltaV1 {
    path: Vec<String>,
    name: String,
    upstream_available_on: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    wrapper_level: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    note: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum ReportIntentionallyUnsupportedDeltaV1 {
    Command(ReportCommandDeltaV1),
    Flag(ReportFlagDeltaV1),
    Arg(ReportArgDeltaV1),
}

#[derive(Debug, Clone)]
struct IuRoot {
    path: Vec<String>,
    targets: BTreeSet<String>,
    note: String,
}

fn build_iu_roots(
    wrapper: &WrapperCoverageV1,
    wrapper_index: &WrapperIndex,
    report_target_set: &BTreeSet<String>,
    expected_targets: &[String],
    filter_mode: FilterMode<'_>,
) -> Result<Vec<IuRoot>, ReportError> {
    let mut unique_paths: BTreeSet<Vec<String>> = BTreeSet::new();
    for cmd in &wrapper.coverage {
        if cmd.level == "intentionally_unsupported" {
            unique_paths.insert(cmd.path.clone());
        }
    }

    let mut roots = Vec::new();
    for path in unique_paths {
        let res = resolve_wrapper(
            wrapper_index
                .commands
                .get(&path)
                .map(Vec::as_slice)
                .unwrap_or(&[]),
            report_target_set,
            expected_targets,
            filter_mode,
            "command",
            &format!("path={}", format_path(&path)),
        )?;
        if res.level.as_deref() != Some("intentionally_unsupported") {
            continue;
        }
        let note = require_non_empty_note(
            res.note.as_deref(),
            "command",
            &format!("path={}", format_path(&path)),
        )?;
        roots.push(IuRoot {
            path,
            targets: res.targets,
            note,
        });
    }

    roots.sort_by(|a, b| {
        b.path
            .len()
            .cmp(&a.path.len())
            .then_with(|| cmp_path(&a.path, &b.path))
    });
    Ok(roots)
}

fn find_inherited_iu_root<'a>(
    roots: &'a [IuRoot],
    unit_path: &[String],
    unit_available_on: &[String],
    report_target_set: &BTreeSet<String>,
    unit_kind: &'static str,
) -> Result<Option<&'a IuRoot>, ReportError> {
    let relevant_targets: BTreeSet<String> = unit_available_on
        .iter()
        .filter(|t| report_target_set.contains(*t))
        .cloned()
        .collect();
    if relevant_targets.is_empty() {
        return Ok(None);
    }

    for root in roots {
        if !is_prefix(&root.path, unit_path) {
            continue;
        }

        let overlap: BTreeSet<String> = relevant_targets
            .intersection(&root.targets)
            .cloned()
            .collect();
        if overlap.is_empty() {
            continue;
        }
        if overlap != relevant_targets {
            return Err(ReportError::WrapperResolution {
                unit: unit_kind.to_string(),
                detail: format!(
                    "IU subtree root scope mismatch: root_path={} does not cover all upstream targets for unit_path={} (root_targets={} unit_targets={})",
                    format_path(&root.path),
                    format_path(unit_path),
                    root.targets.iter().cloned().collect::<Vec<_>>().join(","),
                    relevant_targets
                        .iter()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(","),
                ),
            });
        }

        return Ok(Some(root));
    }

    Ok(None)
}

fn is_prefix(prefix: &[String], path: &[String]) -> bool {
    prefix.len() <= path.len() && prefix.iter().zip(path).all(|(a, b)| a == b)
}

fn require_non_empty_note(
    note: Option<&str>,
    unit_kind: &'static str,
    detail: &str,
) -> Result<String, ReportError> {
    match note.map(str::trim).filter(|s| !s.is_empty()) {
        Some(v) => Ok(v.to_string()),
        None => Err(ReportError::WrapperResolution {
            unit: unit_kind.to_string(),
            detail: format!("{detail} intentionally_unsupported requires non-empty note"),
        }),
    }
}

fn iu_kind_rank(entry: &ReportIntentionallyUnsupportedDeltaV1) -> u8 {
    match entry {
        ReportIntentionallyUnsupportedDeltaV1::Command(_) => 0,
        ReportIntentionallyUnsupportedDeltaV1::Flag(_) => 1,
        ReportIntentionallyUnsupportedDeltaV1::Arg(_) => 2,
    }
}

fn iu_path(entry: &ReportIntentionallyUnsupportedDeltaV1) -> &[String] {
    match entry {
        ReportIntentionallyUnsupportedDeltaV1::Command(v) => &v.path,
        ReportIntentionallyUnsupportedDeltaV1::Flag(v) => &v.path,
        ReportIntentionallyUnsupportedDeltaV1::Arg(v) => &v.path,
    }
}

fn cmp_iu_delta(
    a: &ReportIntentionallyUnsupportedDeltaV1,
    b: &ReportIntentionallyUnsupportedDeltaV1,
) -> std::cmp::Ordering {
    iu_kind_rank(a).cmp(&iu_kind_rank(b)).then_with(|| {
        cmp_path(iu_path(a), iu_path(b)).then_with(|| match (a, b) {
            (
                ReportIntentionallyUnsupportedDeltaV1::Flag(a),
                ReportIntentionallyUnsupportedDeltaV1::Flag(b),
            ) => a.key.cmp(&b.key),
            (
                ReportIntentionallyUnsupportedDeltaV1::Arg(a),
                ReportIntentionallyUnsupportedDeltaV1::Arg(b),
            ) => a.name.cmp(&b.name),
            _ => std::cmp::Ordering::Equal,
        })
    })
}

#[derive(Debug)]
struct ParityExclusionsIndex {
    commands: BTreeMap<Vec<String>, ParityExclusionUnit>,
    flags: BTreeMap<(Vec<String>, String), ParityExclusionUnit>,
    args: BTreeMap<(Vec<String>, String), ParityExclusionUnit>,
}

fn build_parity_exclusions_index(exclusions: &RulesParityExclusions) -> ParityExclusionsIndex {
    let mut commands = BTreeMap::new();
    let mut flags = BTreeMap::new();
    let mut args = BTreeMap::new();

    for unit in &exclusions.units {
        match unit.unit.as_str() {
            "command" => {
                commands.insert(unit.path.clone(), unit.clone());
            }
            "flag" => {
                if let Some(key) = unit.key.as_ref() {
                    flags.insert((unit.path.clone(), key.clone()), unit.clone());
                }
            }
            "arg" => {
                if let Some(name) = unit.name.as_ref() {
                    args.insert((unit.path.clone(), name.clone()), unit.clone());
                }
            }
            _ => {}
        }
    }

    ParityExclusionsIndex {
        commands,
        flags,
        args,
    }
}
