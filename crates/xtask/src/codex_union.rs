use std::{
    collections::{BTreeMap, BTreeSet},
    fs, io,
    path::PathBuf,
};

use clap::Parser;
use serde::Deserialize;
use thiserror::Error;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use crate::codex_snapshot::SnapshotV1;

mod merge;
mod schema;
use merge::build_union_commands;
use schema::{SnapshotUnionV2, UnionCommandSnapshotV2, UnionInputV2};

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
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("failed to parse JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid rules file: {0}")]
    Rules(String),
    #[error("missing required snapshot for target {target_triple}: {path}")]
    MissingRequiredSnapshot {
        target_triple: String,
        path: PathBuf,
    },
    #[error("snapshot tool mismatch in {path} (expected {expected}, got {got})")]
    SnapshotToolMismatch {
        path: PathBuf,
        expected: String,
        got: String,
    },
    #[error("snapshot semantic version mismatch in {path} (expected {expected}, got {got:?})")]
    SnapshotVersionMismatch {
        path: PathBuf,
        expected: String,
        got: Option<String>,
    },
}

#[derive(Debug, Deserialize)]
struct RulesFile {
    union: RulesUnion,
    #[serde(default)]
    globals: RulesGlobals,
    sorting: RulesSorting,
}

#[derive(Debug, Default, Deserialize)]
struct RulesGlobals {
    #[serde(default)]
    effective_flags_model: RulesEffectiveFlagsModel,
}

#[derive(Debug, Default, Deserialize)]
struct RulesEffectiveFlagsModel {
    #[serde(default)]
    enabled: bool,
    #[serde(default)]
    union_normalization: RulesUnionNormalization,
}

#[derive(Debug, Default, Deserialize)]
struct RulesUnionNormalization {
    #[serde(default)]
    dedupe_per_command_flags_against_root: bool,
    #[serde(default)]
    dedupe_key: String,
}

#[derive(Debug, Deserialize)]
struct RulesUnion {
    required_target: String,
    expected_targets: Vec<String>,
    #[serde(default)]
    require_same_tool: bool,
    #[serde(default)]
    require_same_semantic_version: bool,
    #[serde(default)]
    require_semantic_version: bool,
}

#[derive(Debug, Deserialize)]
struct RulesSorting {
    commands: String,
    flags: String,
    args: String,
    inputs: String,
    available_on: String,
    conflicts: String,
}

pub fn run(args: Args) -> Result<(), Error> {
    let root = fs::canonicalize(&args.root).unwrap_or(args.root.clone());
    let rules_path = args
        .rules
        .clone()
        .unwrap_or_else(|| root.join("RULES.json"));

    let rules: RulesFile = serde_json::from_slice(&fs::read(&rules_path)?)?;
    assert_supported_sorting(&rules.sorting)?;
    assert_supported_globals(&rules.globals)?;

    let expected_targets = rules.union.expected_targets;
    if expected_targets.is_empty() {
        return Err(Error::Rules(
            "union.expected_targets must not be empty".to_string(),
        ));
    }

    let snapshots_dir = root.join("snapshots").join(&args.version);
    let mut snapshots_by_target: BTreeMap<String, SnapshotV1> = BTreeMap::new();

    for target in &expected_targets {
        let snapshot_path = snapshots_dir.join(format!("{target}.json"));
        if !snapshot_path.is_file() {
            continue;
        }

        let snapshot: SnapshotV1 = serde_json::from_slice(&fs::read(&snapshot_path)?)?;

        if rules.union.require_same_tool && snapshot.tool != "codex-cli" {
            return Err(Error::SnapshotToolMismatch {
                path: snapshot_path,
                expected: "codex-cli".to_string(),
                got: snapshot.tool,
            });
        }

        if rules.union.require_same_semantic_version
            && snapshot.binary.semantic_version.as_deref() != Some(args.version.as_str())
        {
            return Err(Error::SnapshotVersionMismatch {
                path: snapshot_path,
                expected: args.version.clone(),
                got: snapshot.binary.semantic_version,
            });
        }

        if rules.union.require_semantic_version && snapshot.binary.semantic_version.is_none() {
            return Err(Error::SnapshotVersionMismatch {
                path: snapshot_path,
                expected: args.version.clone(),
                got: snapshot.binary.semantic_version,
            });
        }

        snapshots_by_target.insert(target.clone(), snapshot);
    }

    if !snapshots_by_target.contains_key(&rules.union.required_target) {
        let required_path = snapshots_dir.join(format!("{}.json", rules.union.required_target));
        return Err(Error::MissingRequiredSnapshot {
            target_triple: rules.union.required_target,
            path: required_path,
        });
    }

    let present_targets: Vec<String> = expected_targets
        .iter()
        .filter(|t| snapshots_by_target.contains_key(*t))
        .cloned()
        .collect();
    let complete = present_targets.len() == expected_targets.len();
    let missing_targets: Vec<String> = expected_targets
        .iter()
        .filter(|t| !snapshots_by_target.contains_key(*t))
        .cloned()
        .collect();

    let collected_at = deterministic_rfc3339_now();

    let mut inputs = Vec::new();
    for target in &present_targets {
        let snapshot = &snapshots_by_target[target];
        inputs.push(UnionInputV2 {
            target_triple: target.clone(),
            collected_at: Some(snapshot.collected_at.clone()),
            binary: snapshot.binary.clone(),
            features: snapshot.features.clone(),
            known_omissions: snapshot.known_omissions.clone(),
        });
    }

    let raw_help_root = root.join("raw_help").join(&args.version);
    let mut commands = build_union_commands(
        &expected_targets,
        &rules.union.required_target,
        &args.version,
        &raw_help_root,
        &present_targets,
        &snapshots_by_target,
    );
    normalize_union_commands(&mut commands, &rules.globals.effective_flags_model);

    let union = SnapshotUnionV2 {
        snapshot_schema_version: 2,
        tool: "codex-cli".to_string(),
        mode: "union".to_string(),
        collected_at,
        expected_targets,
        complete,
        missing_targets: if complete {
            None
        } else {
            Some(missing_targets)
        },
        inputs,
        commands,
    };

    let out_path = snapshots_dir.join("union.json");
    fs::create_dir_all(&snapshots_dir)?;
    fs::write(
        out_path,
        format!("{}\n", serde_json::to_string_pretty(&union)?),
    )?;
    Ok(())
}

fn normalize_union_commands(
    commands: &mut [UnionCommandSnapshotV2],
    model: &RulesEffectiveFlagsModel,
) {
    if !model.enabled
        || !model
            .union_normalization
            .dedupe_per_command_flags_against_root
    {
        return;
    }

    // v1 uses key identity at union layer already (long_or_short), so we only need the `key`.
    let root_keys: BTreeSet<String> = commands
        .iter()
        .find(|cmd| cmd.path.is_empty())
        .and_then(|cmd| cmd.flags.as_ref())
        .map(|flags| flags.iter().map(|f| f.key.clone()).collect())
        .unwrap_or_default();

    if root_keys.is_empty() {
        return;
    }

    for cmd in commands.iter_mut() {
        if cmd.path.is_empty() {
            continue;
        }

        if let Some(flags) = cmd.flags.as_mut() {
            flags.retain(|f| !root_keys.contains(&f.key));
            if flags.is_empty() {
                cmd.flags = None;
            }
        }

        if let Some(conflicts) = cmd.conflicts.as_mut() {
            conflicts.retain(|c| {
                if c.unit != "flag" {
                    return true;
                }
                if c.path != cmd.path {
                    return true;
                }
                let Some(key) = c.key.as_deref() else {
                    return true;
                };
                !root_keys.contains(key)
            });
            if conflicts.is_empty() {
                cmd.conflicts = None;
            }
        }
    }
}

fn assert_supported_sorting(sorting: &RulesSorting) -> Result<(), Error> {
    let mut unsupported = Vec::new();

    if sorting.commands != "lexicographic_path" {
        unsupported.push(format!("sorting.commands={}", sorting.commands));
    }
    if sorting.flags != "by_key_then_long_then_short" {
        unsupported.push(format!("sorting.flags={}", sorting.flags));
    }
    if sorting.args != "by_name" {
        unsupported.push(format!("sorting.args={}", sorting.args));
    }
    if sorting.inputs != "rules_expected_targets_order" {
        unsupported.push(format!("sorting.inputs={}", sorting.inputs));
    }
    if sorting.available_on != "rules_expected_targets_order" {
        unsupported.push(format!("sorting.available_on={}", sorting.available_on));
    }
    if sorting.conflicts != "by_unit_then_path_then_key_or_name_then_field" {
        unsupported.push(format!("sorting.conflicts={}", sorting.conflicts));
    }

    if unsupported.is_empty() {
        Ok(())
    } else {
        Err(Error::Rules(format!(
            "unsupported sorting rules: {}",
            unsupported.join(", ")
        )))
    }
}

fn assert_supported_globals(globals: &RulesGlobals) -> Result<(), Error> {
    let model = &globals.effective_flags_model;
    if !model.enabled
        || !model
            .union_normalization
            .dedupe_per_command_flags_against_root
    {
        return Ok(());
    }

    let key = model.union_normalization.dedupe_key.trim();
    if key.is_empty() || key == "flag_key" {
        Ok(())
    } else {
        Err(Error::Rules(format!(
            "unsupported globals.effective_flags_model.union_normalization.dedupe_key={key}"
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
