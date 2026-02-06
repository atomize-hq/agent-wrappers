use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use sha2::{Digest, Sha256};

use crate::codex_snapshot::{ArgSnapshot, CommandSnapshot, FlagSnapshot, SnapshotV1};

use super::schema::{
    UnionArgSnapshotV2, UnionCommandSnapshotV2, UnionConflictEntryV2, UnionConflictEvidenceV2,
    UnionFlagSnapshotV2,
};

pub(super) fn build_union_commands(
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
