use std::collections::BTreeSet;

use super::{
    super::{
        models::WrapperCoverageV1,
        util,
        wrapper::{self, FilterMode, WrapperIndex},
        ReportError,
    },
    schema::ReportIntentionallyUnsupportedDeltaV1,
};

#[derive(Debug, Clone)]
pub(super) struct IuRoot {
    pub(super) path: Vec<String>,
    targets: BTreeSet<String>,
    pub(super) note: String,
}

pub(super) fn build_iu_roots(
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
        let res = wrapper::resolve_wrapper(
            wrapper_index
                .commands
                .get(&path)
                .map(Vec::as_slice)
                .unwrap_or(&[]),
            report_target_set,
            expected_targets,
            filter_mode,
            "command",
            &format!("path={}", util::format_path(&path)),
        )?;
        if res.level.as_deref() != Some("intentionally_unsupported") {
            continue;
        }
        let note = require_non_empty_note(
            res.note.as_deref(),
            "command",
            &format!("path={}", util::format_path(&path)),
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
            .then_with(|| util::cmp_path(&a.path, &b.path))
    });
    Ok(roots)
}

pub(super) fn find_inherited_iu_root<'a>(
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
        if !util::is_prefix(&root.path, unit_path) {
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
                    util::format_path(&root.path),
                    util::format_path(unit_path),
                    root.targets.iter().cloned().collect::<Vec<_>>().join(","),
                    relevant_targets.iter().cloned().collect::<Vec<_>>().join(","),
                ),
            });
        }

        return Ok(Some(root));
    }

    Ok(None)
}

pub(super) fn require_non_empty_note(
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

pub(super) fn cmp_iu_delta(
    a: &ReportIntentionallyUnsupportedDeltaV1,
    b: &ReportIntentionallyUnsupportedDeltaV1,
) -> std::cmp::Ordering {
    iu_kind_rank(a).cmp(&iu_kind_rank(b)).then_with(|| {
        util::cmp_path(iu_path(a), iu_path(b)).then_with(|| match (a, b) {
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
