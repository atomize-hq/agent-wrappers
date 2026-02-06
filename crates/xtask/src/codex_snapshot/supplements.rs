use std::{collections::BTreeMap, fs, path::Path};

use super::{schema::SupplementV1, CommandSnapshot, Error, FlagSnapshot};

pub(super) fn normalize_command_entries(entries: &mut BTreeMap<Vec<String>, CommandSnapshot>) {
    for cmd in entries.values_mut() {
        if let Some(args) = cmd.args.as_mut() {
            args.sort_by(|a, b| a.name.cmp(&b.name));
        }
        if let Some(flags) = cmd.flags.as_mut() {
            flags.sort_by(flag_sort_key);
        }
    }
}

pub(super) fn merge_flags(dst: &mut Vec<FlagSnapshot>, src: Vec<FlagSnapshot>) {
    for f in src {
        let idx = if let Some(long) = f.long.as_deref() {
            dst.iter().position(|e| e.long.as_deref() == Some(long))
        } else if let Some(short) = f.short.as_deref() {
            dst.iter().position(|e| e.short.as_deref() == Some(short))
        } else {
            None
        };

        match idx {
            Some(i) => {
                let e = &mut dst[i];
                if e.long.is_none() {
                    e.long = f.long;
                }
                if e.short.is_none() {
                    e.short = f.short;
                }
                e.takes_value |= f.takes_value;
                if e.value_name.is_none() {
                    e.value_name = f.value_name;
                }
                if e.repeatable.is_none() {
                    e.repeatable = f.repeatable;
                }
                if e.stability.is_none() {
                    e.stability = f.stability;
                }
                if e.platforms.is_none() {
                    e.platforms = f.platforms;
                }
            }
            None => dst.push(f),
        }
    }
}

pub(super) fn flag_sort_key(a: &FlagSnapshot, b: &FlagSnapshot) -> std::cmp::Ordering {
    cmp_opt_str(&a.long, &b.long).then_with(|| cmp_opt_str(&a.short, &b.short))
}

fn cmp_opt_str(a: &Option<String>, b: &Option<String>) -> std::cmp::Ordering {
    match (a, b) {
        (Some(a), Some(b)) => a.cmp(b),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    }
}

pub(super) fn cmp_path(a: &[String], b: &[String]) -> std::cmp::Ordering {
    let mut i = 0usize;
    while i < a.len() && i < b.len() {
        match a[i].cmp(&b[i]) {
            std::cmp::Ordering::Equal => i += 1,
            non_eq => return non_eq,
        }
    }
    a.len().cmp(&b.len())
}

pub(super) fn apply_supplements(
    supplement_path: Option<&Path>,
    commands: &mut BTreeMap<Vec<String>, CommandSnapshot>,
) -> Result<(Vec<String>, bool), Error> {
    let Some(path) = supplement_path else {
        return Ok((Vec::new(), false));
    };
    let text = fs::read_to_string(path)?;
    let supplement: SupplementV1 = serde_json::from_str(&text)?;
    if supplement.version != 1 {
        return Err(Error::SupplementVersion(supplement.version));
    }

    let mut known_omissions = Vec::new();
    let mut any_applied = false;

    for item in supplement.commands {
        let mut applied = false;
        let existed = commands.contains_key(&item.path);
        let entry = commands
            .entry(item.path.clone())
            .or_insert_with(|| CommandSnapshot {
                path: item.path.clone(),
                about: None,
                usage: None,
                stability: None,
                platforms: None,
                args: None,
                flags: None,
            });

        if !existed {
            applied = true;
        }

        if let Some(platforms) = item.platforms {
            if entry.platforms.as_ref() != Some(&platforms) {
                entry.platforms = Some(platforms);
                applied = true;
            }
        }

        if applied {
            any_applied = true;
            known_omissions.push(format!(
                "supplement/commands.json:v1:{}",
                item.path.join(" ")
            ));
        }
    }

    known_omissions.sort();
    known_omissions.dedup();
    Ok((known_omissions, any_applied))
}
