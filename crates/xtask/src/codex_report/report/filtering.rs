use std::collections::{BTreeMap, BTreeSet};

use super::{
    super::{
        models::{UnionArgV2, UnionCommandV2, UnionFlagV2},
        util,
        wrapper::{CoverageResolution, FilterMode},
    },
    schema::{ReportArgDeltaV1, ReportCommandDeltaV1, ReportFlagDeltaV1},
};

pub(super) fn present_on_filter(
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

pub(super) fn classify_command_delta(
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

pub(super) fn classify_flag_delta(
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

pub(super) fn classify_arg_delta(
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

pub(super) fn upstream_flag_availability(
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
        util::ordered_subset(expected_targets, &wrapper_res.targets),
        false,
    )
}

pub(super) fn upstream_arg_availability(
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
        util::ordered_subset(expected_targets, &wrapper_res.targets),
        false,
    )
}
