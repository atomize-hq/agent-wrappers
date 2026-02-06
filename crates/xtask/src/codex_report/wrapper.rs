use std::collections::{BTreeMap, BTreeSet};

use super::{models::WrapperCoverageV1, models::WrapperScope, util, ReportError};

#[derive(Debug, Clone, Copy)]
pub(super) enum FilterMode<'a> {
    Any,
    All,
    ExactTarget(&'a str),
}

#[derive(Debug, Clone)]
pub(super) struct ScopedCoverage {
    pub(super) index: usize,
    pub(super) targets: BTreeSet<String>,
    pub(super) level: String,
    pub(super) note: Option<String>,
}

#[derive(Debug, Clone)]
pub(super) struct CoverageResolution {
    pub(super) present: bool,
    pub(super) targets: BTreeSet<String>,
    pub(super) level: Option<String>,
    pub(super) note: Option<String>,
}

#[derive(Debug, Clone)]
pub(super) struct WrapperIndex {
    pub(super) commands: BTreeMap<Vec<String>, Vec<ScopedCoverage>>,
    pub(super) flags: BTreeMap<(Vec<String>, String), Vec<ScopedCoverage>>,
    pub(super) args: BTreeMap<(Vec<String>, String), Vec<ScopedCoverage>>,
}

pub(super) fn index_wrapper(
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
            let effective = util::intersect(&cmd_targets, &flag_targets);
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
            let effective = util::intersect(&cmd_targets, &arg_targets);
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

pub(super) fn resolve_wrapper(
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
