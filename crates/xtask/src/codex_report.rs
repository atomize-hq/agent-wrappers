use std::{fs, io, path::PathBuf};

use clap::Parser;
use thiserror::Error;

mod models;
mod report;
mod rules;
mod util;
mod wrapper;

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

pub fn run(args: Args) -> Result<(), ReportError> {
    let root = fs::canonicalize(&args.root).unwrap_or(args.root.clone());
    let rules_path = args
        .rules
        .clone()
        .unwrap_or_else(|| root.join("RULES.json"));

    let rules = rules::load_rules(&rules_path)?;
    rules::assert_supported_rules(&rules)?;
    let parity_exclusions = rules
        .parity_exclusions
        .as_ref()
        .filter(|ex| ex.schema_version == 1)
        .map(report::build_parity_exclusions_index);

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

    let upstream = report::index_upstream(&union);
    let wrapper_index = wrapper::index_wrapper(
        &rules.union.expected_targets,
        &rules.union.platform_mapping,
        &wrapper,
    );

    let reports_dir = root.join("reports").join(&args.version);
    fs::create_dir_all(&reports_dir)?;

    util::require_source_date_epoch_if_ci()?;
    let generated_at = util::deterministic_rfc3339_now();

    // coverage.any.json (always)
    {
        let report = report::build_report(
            &rules,
            parity_exclusions.as_ref(),
            &args.version,
            "any",
            None,
            wrapper::FilterMode::Any,
            &input_targets,
            &upstream,
            &wrapper,
            &wrapper_index,
            &generated_at,
        )?;
        let out_path = reports_dir.join(&rules.report.file_naming.any);
        util::write_json_pretty(&out_path, &serde_json::to_string_pretty(&report)?)?;
    }

    // coverage.<target_triple>.json (one per included input target)
    for target in &input_targets {
        let report = report::build_report(
            &rules,
            parity_exclusions.as_ref(),
            &args.version,
            "exact_target",
            Some(target.as_str()),
            wrapper::FilterMode::ExactTarget(target),
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
        util::write_json_pretty(&out_path, &serde_json::to_string_pretty(&report)?)?;
    }

    // coverage.all.json (only when union.complete=true)
    if union.complete {
        let report = report::build_report(
            &rules,
            parity_exclusions.as_ref(),
            &args.version,
            "all",
            None,
            wrapper::FilterMode::All,
            &union.expected_targets,
            &upstream,
            &wrapper,
            &wrapper_index,
            &generated_at,
        )?;
        let out_path = reports_dir.join(&rules.report.file_naming.all);
        util::write_json_pretty(&out_path, &serde_json::to_string_pretty(&report)?)?;
    }

    Ok(())
}
