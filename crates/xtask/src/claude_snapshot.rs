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
    /// Path to the `claude` binary to snapshot.
    #[arg(long)]
    pub claude_binary: PathBuf,

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

    /// Path to `cli_manifests/claude_code/supplement/commands.json` (schema v1).
    #[arg(long)]
    pub supplement: Option<PathBuf>,

    /// Override `collected_at` (RFC3339). Intended for determinism in tests/CI.
    #[arg(long)]
    pub collected_at: Option<String>,

    /// Timeout for each `claude ... --help` invocation, in milliseconds.
    ///
    /// This is a safety valve to prevent snapshot generation from hanging on any single command.
    #[arg(long, default_value_t = 20_000)]
    pub help_timeout_ms: u64,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid claude binary path: {0}")]
    InvalidClaudeBinary(PathBuf),
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
    let claude_binary = fs::canonicalize(&args.claude_binary)
        .map_err(|_| Error::InvalidClaudeBinary(args.claude_binary.clone()))?;
    if !claude_binary.is_file() {
        return Err(Error::InvalidClaudeBinary(claude_binary));
    }

    let collected_at = if let Some(s) = args.collected_at.as_ref() {
        OffsetDateTime::parse(s, &Rfc3339).map_err(|_| Error::CollectedAt(s.clone()))?;
        s.clone()
    } else {
        probes::deterministic_rfc3339_now()
    };

    let binary_meta = probes::BinaryMetadata::collect(&claude_binary)?;
    let (version_output, semantic_version) = probes::probe_version(&claude_binary)?;

    let version_dir = match (&args.out_file, &semantic_version) {
        (Some(_), None) => return Err(Error::MissingSemanticVersion),
        (_, Some(v)) => v.clone(),
        (None, None) => "unknown".to_string(),
    };

    let (snapshot_out_path, raw_help_dir, inferred_target_triple) =
        layout::resolve_outputs(&args, &version_dir)?;

    let discovery = discovery::discover_commands(
        &claude_binary,
        raw_help_dir.as_deref(),
        args.capture_raw_help,
        args.help_timeout_ms,
    )?;
    let mut command_entries = discovery.commands;
    let mut known_omissions = discovery.known_omissions;

    let (supplement_omissions, _supplemented) =
        supplements::apply_supplements(args.supplement.as_deref(), &mut command_entries)?;

    known_omissions.extend(supplement_omissions);

    supplements::normalize_command_entries(&mut command_entries);

    let mut commands: Vec<CommandSnapshot> = command_entries.into_values().collect();
    commands.sort_by(|a, b| supplements::cmp_path(&a.path, &b.path));

    let snapshot = SnapshotV1 {
        snapshot_schema_version: 1,
        tool: "claude-code-cli".to_string(),
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
            channel: None,
            commit: None,
        },
        commands,
        features: None,
        known_omissions: if known_omissions.is_empty() {
            None
        } else {
            Some(known_omissions)
        },
    };

    let json = serde_json::to_string_pretty(&snapshot)?;
    if let Some(parent) = snapshot_out_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(snapshot_out_path, format!("{json}\n"))?;
    Ok(())
}
