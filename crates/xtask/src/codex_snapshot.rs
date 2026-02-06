use std::{fs, io, path::PathBuf};

use clap::Parser;
use thiserror::Error;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

mod discovery;
mod layout;
mod probes;
mod schema;
mod supplements;
mod util;

pub(crate) use schema::{
    ArgSnapshot, BinaryPlatform, BinarySnapshot, CommandSnapshot, FlagSnapshot, SnapshotV1,
};

#[derive(Debug, Parser)]
pub struct Args {
    /// Path to the `codex` binary to snapshot.
    #[arg(long)]
    pub codex_binary: PathBuf,

    /// Output directory (legacy mode; writes `current.json` under this directory).
    #[arg(
        long,
        required_unless_present = "out_file",
        conflicts_with = "out_file"
    )]
    pub out_dir: Option<PathBuf>,

    /// Output snapshot file (per-target mode; writes an UpstreamSnapshotV1 JSON file).
    #[arg(long, required_unless_present = "out_dir", conflicts_with = "out_dir")]
    pub out_file: Option<PathBuf>,

    /// Also write raw `--help` output under `raw_help/<version>/...` for debugging parser drift.
    #[arg(long)]
    pub capture_raw_help: bool,

    /// Target triple used for raw help capture layout: `raw_help/<version>/<target_triple>/**`.
    #[arg(long)]
    pub raw_help_target: Option<String>,

    /// Path to `cli_manifests/codex/supplement/commands.json` (schema v1).
    #[arg(long)]
    pub supplement: Option<PathBuf>,

    /// Override `collected_at` (RFC3339). Intended for determinism in tests/CI.
    #[arg(long)]
    pub collected_at: Option<String>,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid codex binary path: {0}")]
    InvalidCodexBinary(PathBuf),
    #[error("--capture-raw-help requires a target triple (use --raw-help-target, or infer it from --out-file)")]
    MissingRawHelpTarget,
    #[error("failed to probe semantic version; required for per-target snapshots")]
    MissingSemanticVersion,
    #[error("failed to read rules file: {0}")]
    RulesRead(String),
    #[error("unsupported rules configuration: {0}")]
    RulesUnsupported(String),
    #[error("raw_help_target={0} is not in RULES.json.union.expected_targets")]
    RawHelpTargetNotExpected(String),
    #[error("invalid --out-file layout: {0}")]
    InvalidOutFileLayout(String),
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("command failed: {0}")]
    CommandFailed(String),
    #[error("failed to parse JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("supplement file version must be 1 (got {0})")]
    SupplementVersion(u32),
    #[error("invalid collected_at (must be RFC3339): {0}")]
    CollectedAt(String),
}

pub fn run(args: Args) -> Result<(), Error> {
    let codex_binary = fs::canonicalize(&args.codex_binary)
        .map_err(|_| Error::InvalidCodexBinary(args.codex_binary.clone()))?;
    if !codex_binary.is_file() {
        return Err(Error::InvalidCodexBinary(codex_binary));
    }

    let collected_at = if let Some(s) = args.collected_at.as_ref() {
        OffsetDateTime::parse(s, &Rfc3339).map_err(|_| Error::CollectedAt(s.clone()))?;
        s.clone()
    } else {
        probes::deterministic_rfc3339_now()
    };

    let binary_meta = probes::BinaryMetadata::collect(&codex_binary)?;
    let (version_output, semantic_version, channel, commit) = probes::probe_version(&codex_binary)?;

    let version_dir = match (&args.out_file, &semantic_version) {
        (Some(_), None) => return Err(Error::MissingSemanticVersion),
        (_, Some(v)) => v.clone(),
        (None, None) => "unknown".to_string(),
    };

    let (snapshot_out_path, raw_help_dir, inferred_target_triple) =
        layout::resolve_outputs(&args, &version_dir)?;
    let (features_list, features_probe_error) = probes::probe_features(&codex_binary);
    let enabled_feature_names = features_list
        .as_ref()
        .map(|f| f.iter().map(|x| x.name.clone()).collect::<Vec<_>>())
        .unwrap_or_default();
    let enable_args = enabled_feature_names
        .iter()
        .flat_map(|name| ["--enable".to_string(), name.clone()])
        .collect::<Vec<_>>();

    // Always snapshot the default surface. If we can probe features, do a second discovery pass
    // with all known features enabled and merge results for maximum coverage.
    let default_entries =
        discovery::discover_commands(&codex_binary, raw_help_dir.as_deref(), false, &[])?;

    let (mut command_entries, commands_added_when_all_enabled) = if enable_args.is_empty() {
        if args.capture_raw_help {
            // If we can't enable features, capture raw help for the default surface.
            let _ = discovery::discover_commands(&codex_binary, raw_help_dir.as_deref(), true, &[]);
        }
        (default_entries, None)
    } else {
        let enabled_entries = discovery::discover_commands(
            &codex_binary,
            raw_help_dir.as_deref(),
            args.capture_raw_help,
            &enable_args,
        )?;
        let added = enabled_entries
            .keys()
            .filter(|k| !default_entries.contains_key(*k))
            .cloned()
            .collect::<Vec<_>>();
        let mut merged = default_entries;
        discovery::merge_command_entries(&mut merged, enabled_entries);
        (merged, Some(added))
    };

    let (known_omissions, supplemented) =
        supplements::apply_supplements(args.supplement.as_deref(), &mut command_entries)?;

    supplements::normalize_command_entries(&mut command_entries);

    let mut commands: Vec<CommandSnapshot> = command_entries.into_values().collect();
    commands.sort_by(|a, b| supplements::cmp_path(&a.path, &b.path));

    let features = probes::build_features_metadata(
        features_list,
        features_probe_error,
        enabled_feature_names,
        commands_added_when_all_enabled,
    );

    let snapshot = SnapshotV1 {
        snapshot_schema_version: 1,
        tool: "codex-cli".to_string(),
        collected_at,
        binary: BinarySnapshot {
            sha256: binary_meta.sha256,
            size_bytes: binary_meta.size_bytes,
            platform: BinaryPlatform {
                os: std::env::consts::OS.to_string(),
                arch: std::env::consts::ARCH.to_string(),
            },
            target_triple: inferred_target_triple,
            version_output,
            semantic_version,
            channel,
            commit,
        },
        commands,
        features,
        known_omissions: if supplemented {
            Some(known_omissions)
        } else {
            None
        },
    };

    let json = serde_json::to_string_pretty(&snapshot)?;
    if let Some(parent) = snapshot_out_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(snapshot_out_path, format!("{json}\n"))?;
    Ok(())
}
