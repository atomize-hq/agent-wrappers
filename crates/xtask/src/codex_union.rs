use std::{
    collections::{BTreeMap, BTreeSet},
    fs, io,
    path::{Path, PathBuf},
};

use clap::Parser;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use crate::codex_snapshot::{
    ArgSnapshot, BinarySnapshot, CommandSnapshot, FlagSnapshot, SnapshotV1,
};

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
    normalize_union_commands(
        &mut commands,
        &rules.globals.effective_flags_model,
    );

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

fn normalize_union_commands(commands: &mut [UnionCommandSnapshotV2], model: &RulesEffectiveFlagsModel) {
    if !model.enabled || !model.union_normalization.dedupe_per_command_flags_against_root {
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
    if !model.enabled || !model.union_normalization.dedupe_per_command_flags_against_root {
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

#[derive(Debug, Serialize)]
struct SnapshotUnionV2 {
    snapshot_schema_version: u32,
    tool: String,
    mode: String,
    collected_at: String,
    expected_targets: Vec<String>,
    complete: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    missing_targets: Option<Vec<String>>,
    inputs: Vec<UnionInputV2>,
    commands: Vec<UnionCommandSnapshotV2>,
}

#[derive(Debug, Serialize)]
struct UnionInputV2 {
    target_triple: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    collected_at: Option<String>,
    binary: BinarySnapshot,
    #[serde(skip_serializing_if = "Option::is_none")]
    features: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    known_omissions: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
struct UnionCommandSnapshotV2 {
    path: Vec<String>,
    available_on: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    about: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    usage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    args: Option<Vec<UnionArgSnapshotV2>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    flags: Option<Vec<UnionFlagSnapshotV2>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    conflicts: Option<Vec<UnionConflictEntryV2>>,
}

#[derive(Debug, Serialize)]
struct UnionFlagSnapshotV2 {
    key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    long: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    short: Option<String>,
    takes_value: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    value_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    repeatable: Option<bool>,
    available_on: Vec<String>,
}

#[derive(Debug, Serialize)]
struct UnionArgSnapshotV2 {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    required: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    variadic: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    inferred_from_usage: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    note: Option<String>,
    available_on: Vec<String>,
}

#[derive(Debug, Serialize)]
struct UnionConflictEntryV2 {
    unit: String,
    path: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    field: String,
    values_by_target: BTreeMap<String, serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    help_context: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    evidence: Option<UnionConflictEvidenceV2>,
}

#[derive(Debug, Serialize)]
struct UnionConflictEvidenceV2 {
    #[serde(skip_serializing_if = "Option::is_none")]
    help_ref_by_target: Option<BTreeMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    help_sha256_by_target: Option<BTreeMap<String, String>>,
}

fn build_union_commands(
    expected_targets: &[String],
    required_target: &str,
    version: &str,
    raw_help_root: &Path,
    present_targets: &[String],
    snapshots_by_target: &BTreeMap<String, SnapshotV1>,
) -> Vec<UnionCommandSnapshotV2> {
    let mut commands_by_path: BTreeMap<Vec<String>, BTreeMap<String, CommandSnapshot>> =
        BTreeMap::new();

    for target in present_targets {
        let snapshot = &snapshots_by_target[target];
        for cmd in &snapshot.commands {
            commands_by_path
                .entry(cmd.path.clone())
                .or_default()
                .insert(target.clone(), cmd.clone());
        }
    }

    let mut out = Vec::new();

    for (path, by_target) in commands_by_path {
        let available_on = ordered_subset(expected_targets, by_target.keys());
        let canonical = canonical_target(required_target, expected_targets, &available_on);
        let canonical_cmd = by_target
            .get(&canonical)
            .expect("canonical target must be present");

        let (flags, flag_conflicts) = merge_flags(
            expected_targets,
            required_target,
            version,
            raw_help_root,
            &available_on,
            &path,
            &by_target,
        );
        let (args, arg_conflicts) = merge_args(
            expected_targets,
            required_target,
            version,
            raw_help_root,
            &available_on,
            &path,
            &by_target,
        );

        let mut conflicts = Vec::new();
        conflicts.extend(command_conflicts(
            expected_targets,
            version,
            raw_help_root,
            &path,
            &by_target,
        ));
        conflicts.extend(flag_conflicts);
        conflicts.extend(arg_conflicts);
        conflicts.sort_by(conflict_sort_key);

        out.push(UnionCommandSnapshotV2 {
            path,
            available_on,
            about: canonical_cmd.about.clone(),
            usage: canonical_cmd.usage.clone(),
            args,
            flags,
            conflicts: if conflicts.is_empty() {
                None
            } else {
                Some(conflicts)
            },
        });
    }

    out.sort_by(|a, b| cmp_path(&a.path, &b.path));
    out
}

fn merge_flags(
    expected_targets: &[String],
    required_target: &str,
    version: &str,
    raw_help_root: &Path,
    command_available_on: &[String],
    cmd_path: &[String],
    commands_by_target: &BTreeMap<String, CommandSnapshot>,
) -> (Option<Vec<UnionFlagSnapshotV2>>, Vec<UnionConflictEntryV2>) {
    let mut flags_by_key: BTreeMap<String, BTreeMap<String, FlagSnapshot>> = BTreeMap::new();

    for target in command_available_on {
        let cmd = &commands_by_target[target];
        for flag in cmd.flags.as_deref().unwrap_or_default() {
            let Some(key) = flag.long.clone().or_else(|| flag.short.clone()) else {
                continue;
            };
            flags_by_key
                .entry(key)
                .or_default()
                .insert(target.clone(), flag.clone());
        }
    }

    let mut out_flags = Vec::new();
    let mut conflicts = Vec::new();

    for (key, by_target) in flags_by_key {
        let available_on = ordered_subset(expected_targets, by_target.keys());
        let canonical = canonical_target(required_target, expected_targets, &available_on);
        let canon_flag = &by_target[&canonical];

        out_flags.push(UnionFlagSnapshotV2 {
            key: key.clone(),
            long: canon_flag.long.clone(),
            short: canon_flag.short.clone(),
            takes_value: canon_flag.takes_value,
            value_name: canon_flag.value_name.clone(),
            repeatable: canon_flag.repeatable,
            available_on,
        });

        conflicts.extend(flag_conflicts(
            expected_targets,
            version,
            raw_help_root,
            cmd_path,
            &key,
            &by_target,
        ));
    }

    out_flags.sort_by(|a, b| {
        a.key
            .cmp(&b.key)
            .then_with(|| a.long.cmp(&b.long))
            .then_with(|| a.short.cmp(&b.short))
    });

    (
        if out_flags.is_empty() {
            None
        } else {
            Some(out_flags)
        },
        conflicts,
    )
}

fn flag_conflicts(
    expected_targets: &[String],
    version: &str,
    raw_help_root: &Path,
    cmd_path: &[String],
    key: &str,
    by_target: &BTreeMap<String, FlagSnapshot>,
) -> Vec<UnionConflictEntryV2> {
    let mut out = Vec::new();

    let values = values_by_target(expected_targets, by_target, |f: &FlagSnapshot| {
        Some(serde_json::Value::Bool(f.takes_value))
    });
    if is_conflict(&values) {
        out.push(UnionConflictEntryV2 {
            unit: "flag".to_string(),
            path: cmd_path.to_vec(),
            key: Some(key.to_string()),
            name: None,
            field: "takes_value".to_string(),
            evidence: build_help_evidence(version, raw_help_root, cmd_path, values.keys()),
            values_by_target: values,
            help_context: Some("Options".to_string()),
        });
    }

    let values = values_by_target(expected_targets, by_target, |f: &FlagSnapshot| {
        Some(
            f.value_name
                .clone()
                .map_or(serde_json::Value::Null, serde_json::Value::String),
        )
    });
    if is_conflict(&values) {
        out.push(UnionConflictEntryV2 {
            unit: "flag".to_string(),
            path: cmd_path.to_vec(),
            key: Some(key.to_string()),
            name: None,
            field: "value_name".to_string(),
            evidence: build_help_evidence(version, raw_help_root, cmd_path, values.keys()),
            values_by_target: values,
            help_context: Some("Options".to_string()),
        });
    }

    let values = values_by_target(expected_targets, by_target, |f: &FlagSnapshot| {
        Some(
            f.repeatable
                .map_or(serde_json::Value::Null, serde_json::Value::Bool),
        )
    });
    if is_conflict(&values) {
        out.push(UnionConflictEntryV2 {
            unit: "flag".to_string(),
            path: cmd_path.to_vec(),
            key: Some(key.to_string()),
            name: None,
            field: "repeatable".to_string(),
            evidence: build_help_evidence(version, raw_help_root, cmd_path, values.keys()),
            values_by_target: values,
            help_context: Some("Options".to_string()),
        });
    }

    out
}

fn merge_args(
    expected_targets: &[String],
    required_target: &str,
    version: &str,
    raw_help_root: &Path,
    command_available_on: &[String],
    cmd_path: &[String],
    commands_by_target: &BTreeMap<String, CommandSnapshot>,
) -> (Option<Vec<UnionArgSnapshotV2>>, Vec<UnionConflictEntryV2>) {
    let mut args_by_name: BTreeMap<String, BTreeMap<String, ArgSnapshot>> = BTreeMap::new();

    for target in command_available_on {
        let cmd = &commands_by_target[target];
        for arg in cmd.args.as_deref().unwrap_or_default() {
            args_by_name
                .entry(arg.name.clone())
                .or_default()
                .insert(target.clone(), arg.clone());
        }
    }

    let mut out_args = Vec::new();
    let mut conflicts = Vec::new();

    for (name, by_target) in args_by_name {
        let available_on = ordered_subset(expected_targets, by_target.keys());
        let canonical = canonical_target(required_target, expected_targets, &available_on);
        let canon_arg = &by_target[&canonical];

        let inferred_from_usage = canon_arg
            .note
            .as_deref()
            .is_some_and(|n| n.trim() == "inferred from usage");

        out_args.push(UnionArgSnapshotV2 {
            name: name.clone(),
            required: Some(canon_arg.required),
            variadic: Some(canon_arg.variadic),
            inferred_from_usage: if inferred_from_usage {
                Some(true)
            } else {
                None
            },
            note: canon_arg.note.clone(),
            available_on,
        });

        conflicts.extend(arg_conflicts(
            expected_targets,
            version,
            raw_help_root,
            cmd_path,
            &name,
            &by_target,
        ));
    }

    out_args.sort_by(|a, b| a.name.cmp(&b.name));

    (
        if out_args.is_empty() {
            None
        } else {
            Some(out_args)
        },
        conflicts,
    )
}

fn arg_conflicts(
    expected_targets: &[String],
    version: &str,
    raw_help_root: &Path,
    cmd_path: &[String],
    name: &str,
    by_target: &BTreeMap<String, ArgSnapshot>,
) -> Vec<UnionConflictEntryV2> {
    let mut out = Vec::new();

    let values = values_by_target(expected_targets, by_target, |a: &ArgSnapshot| {
        Some(serde_json::Value::Bool(a.required))
    });
    if is_conflict(&values) {
        out.push(UnionConflictEntryV2 {
            unit: "arg".to_string(),
            path: cmd_path.to_vec(),
            key: None,
            name: Some(name.to_string()),
            field: "required".to_string(),
            evidence: build_help_evidence(version, raw_help_root, cmd_path, values.keys()),
            values_by_target: values,
            help_context: Some("Arguments".to_string()),
        });
    }

    let values = values_by_target(expected_targets, by_target, |a: &ArgSnapshot| {
        Some(serde_json::Value::Bool(a.variadic))
    });
    if is_conflict(&values) {
        out.push(UnionConflictEntryV2 {
            unit: "arg".to_string(),
            path: cmd_path.to_vec(),
            key: None,
            name: Some(name.to_string()),
            field: "variadic".to_string(),
            evidence: build_help_evidence(version, raw_help_root, cmd_path, values.keys()),
            values_by_target: values,
            help_context: Some("Arguments".to_string()),
        });
    }

    out
}

fn command_conflicts(
    expected_targets: &[String],
    version: &str,
    raw_help_root: &Path,
    cmd_path: &[String],
    by_target: &BTreeMap<String, CommandSnapshot>,
) -> Vec<UnionConflictEntryV2> {
    let mut out = Vec::new();

    let values = values_by_target(expected_targets, by_target, |c: &CommandSnapshot| {
        Some(
            c.about
                .clone()
                .map_or(serde_json::Value::Null, serde_json::Value::String),
        )
    });
    if is_conflict(&values) {
        out.push(UnionConflictEntryV2 {
            unit: "command".to_string(),
            path: cmd_path.to_vec(),
            key: None,
            name: None,
            field: "about".to_string(),
            evidence: build_help_evidence(version, raw_help_root, cmd_path, values.keys()),
            values_by_target: values,
            help_context: Some("Usage".to_string()),
        });
    }

    let values = values_by_target(expected_targets, by_target, |c: &CommandSnapshot| {
        Some(
            c.usage
                .clone()
                .map_or(serde_json::Value::Null, serde_json::Value::String),
        )
    });
    if is_conflict(&values) {
        out.push(UnionConflictEntryV2 {
            unit: "command".to_string(),
            path: cmd_path.to_vec(),
            key: None,
            name: None,
            field: "usage".to_string(),
            evidence: build_help_evidence(version, raw_help_root, cmd_path, values.keys()),
            values_by_target: values,
            help_context: Some("Usage".to_string()),
        });
    }

    out
}

fn build_help_evidence<'a, I: Iterator<Item = &'a String>>(
    version: &str,
    raw_help_root: &Path,
    cmd_path: &[String],
    targets: I,
) -> Option<UnionConflictEvidenceV2> {
    let mut refs: BTreeMap<String, String> = BTreeMap::new();
    let mut shas: BTreeMap<String, String> = BTreeMap::new();

    for target in targets {
        let (rel, full) = raw_help_paths(version, raw_help_root, target, cmd_path);
        if !full.is_file() {
            continue;
        }
        refs.insert(target.clone(), rel);
        if let Ok(bytes) = fs::read(&full) {
            let mut hasher = Sha256::new();
            hasher.update(&bytes);
            shas.insert(target.clone(), hex::encode(hasher.finalize()));
        }
    }

    if refs.is_empty() && shas.is_empty() {
        None
    } else {
        Some(UnionConflictEvidenceV2 {
            help_ref_by_target: if refs.is_empty() { None } else { Some(refs) },
            help_sha256_by_target: if shas.is_empty() { None } else { Some(shas) },
        })
    }
}

fn raw_help_paths(
    version: &str,
    raw_help_root: &Path,
    target: &str,
    cmd_path: &[String],
) -> (String, PathBuf) {
    let rel = if cmd_path.is_empty() {
        PathBuf::from("raw_help")
            .join(version)
            .join(target)
            .join("help.txt")
    } else {
        let mut p = PathBuf::from("raw_help")
            .join(version)
            .join(target)
            .join("commands");
        for token in cmd_path {
            p.push(token);
        }
        p.join("help.txt")
    };

    let full = if cmd_path.is_empty() {
        raw_help_root.join(target).join("help.txt")
    } else {
        let mut p = raw_help_root.join(target).join("commands");
        for token in cmd_path {
            p.push(token);
        }
        p.join("help.txt")
    };

    (rel.to_string_lossy().to_string(), full)
}

fn values_by_target<T, F>(
    expected_targets: &[String],
    by_target: &BTreeMap<String, T>,
    mut f: F,
) -> BTreeMap<String, serde_json::Value>
where
    F: FnMut(&T) -> Option<serde_json::Value>,
{
    let mut out = BTreeMap::new();
    for target in expected_targets {
        if let Some(v) = by_target.get(target).and_then(&mut f) {
            out.insert(target.clone(), v);
        }
    }
    out
}

fn is_conflict(values_by_target: &BTreeMap<String, serde_json::Value>) -> bool {
    let mut uniq: BTreeSet<String> = BTreeSet::new();
    for v in values_by_target.values() {
        uniq.insert(v.to_string());
        if uniq.len() > 1 {
            return true;
        }
    }
    false
}

fn ordered_subset<'a, I>(expected_targets: &[String], keys: I) -> Vec<String>
where
    I: IntoIterator<Item = &'a String>,
{
    let set: BTreeSet<&String> = keys.into_iter().collect();
    expected_targets
        .iter()
        .filter(|t| set.contains(*t))
        .cloned()
        .collect()
}

fn canonical_target(
    required_target: &str,
    expected_targets: &[String],
    available_on: &[String],
) -> String {
    if available_on.iter().any(|t| t == required_target) {
        return required_target.to_string();
    }
    expected_targets
        .iter()
        .find(|t| available_on.iter().any(|a| a == *t))
        .cloned()
        .unwrap_or_else(|| required_target.to_string())
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

fn conflict_sort_key(a: &UnionConflictEntryV2, b: &UnionConflictEntryV2) -> std::cmp::Ordering {
    let unit_order = |u: &str| match u {
        "command" => 0u8,
        "flag" => 1u8,
        "arg" => 2u8,
        _ => 3u8,
    };

    unit_order(&a.unit)
        .cmp(&unit_order(&b.unit))
        .then_with(|| cmp_path(&a.path, &b.path))
        .then_with(|| a.key.cmp(&b.key))
        .then_with(|| a.name.cmp(&b.name))
        .then_with(|| a.field.cmp(&b.field))
}
