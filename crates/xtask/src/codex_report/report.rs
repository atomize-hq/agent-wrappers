use std::collections::{BTreeMap, BTreeSet};

use super::{
    models::{UnionCommandV2, UnionSnapshotV2, WrapperCoverageV1},
    rules::RulesFile,
    util,
    wrapper::{self, FilterMode, WrapperIndex},
    ReportError,
};

mod filtering;
mod iu;
mod parity;
mod schema;

use filtering::{
    classify_arg_delta, classify_command_delta, classify_flag_delta, present_on_filter,
    upstream_arg_availability, upstream_flag_availability,
};
use iu::{build_iu_roots, cmp_iu_delta, find_inherited_iu_root, require_non_empty_note};
pub(super) use parity::{build_parity_exclusions_index, ParityExclusionsIndex};
use schema::{
    CoverageReportV1, PlatformFilterV1, ReportArgDeltaV1, ReportCommandDeltaV1, ReportDeltasV1,
    ReportFlagDeltaV1, ReportInputsV1, ReportIntentionallyUnsupportedDeltaV1, ReportRulesInputsV1,
    ReportUpstreamInputsV1, ReportWrapperInputsV1,
};

pub(super) fn index_upstream(union: &UnionSnapshotV2) -> BTreeMap<Vec<String>, UnionCommandV2> {
    let mut out = BTreeMap::new();
    for cmd in &union.commands {
        out.insert(cmd.path.clone(), cmd.clone());
    }
    out
}

#[allow(clippy::too_many_arguments)]
pub(super) fn build_report(
    rules: &RulesFile,
    parity_exclusions: Option<&ParityExclusionsIndex>,
    version: &str,
    platform_mode: &str,
    target_triple: Option<&str>,
    filter_mode: FilterMode<'_>,
    report_targets: &[String],
    upstream: &BTreeMap<Vec<String>, UnionCommandV2>,
    wrapper: &WrapperCoverageV1,
    wrapper_index: &WrapperIndex,
    generated_at: &str,
) -> Result<CoverageReportV1, ReportError> {
    let report_target_set: BTreeSet<String> = report_targets.iter().cloned().collect();
    let expected_set: BTreeSet<String> = rules.union.expected_targets.iter().cloned().collect();
    let iu_roots = build_iu_roots(
        wrapper,
        wrapper_index,
        &report_target_set,
        &rules.union.expected_targets,
        filter_mode,
    )?;

    if matches!(filter_mode, FilterMode::All)
        && !expected_set.is_subset(&report_target_set)
        && rules.report.filter_semantics.when_union_incomplete.all == "error"
    {
        return Err(ReportError::Rules(
            "cannot generate platform_filter.mode=all with an incomplete union target set"
                .to_string(),
        ));
    }

    let mut missing_commands: Vec<ReportCommandDeltaV1> = Vec::new();
    let mut missing_flags: Vec<ReportFlagDeltaV1> = Vec::new();
    let mut missing_args: Vec<ReportArgDeltaV1> = Vec::new();

    let mut excluded_commands: Vec<ReportCommandDeltaV1> = Vec::new();
    let mut excluded_flags: Vec<ReportFlagDeltaV1> = Vec::new();
    let mut excluded_args: Vec<ReportArgDeltaV1> = Vec::new();

    let mut passthrough_candidates: Vec<ReportCommandDeltaV1> = Vec::new();
    let mut unsupported: Vec<ReportCommandDeltaV1> = Vec::new();
    let mut intentionally_unsupported: Vec<ReportIntentionallyUnsupportedDeltaV1> = Vec::new();
    let mut wrapper_only_commands: Vec<ReportCommandDeltaV1> = Vec::new();
    let mut wrapper_only_flags: Vec<ReportFlagDeltaV1> = Vec::new();
    let mut wrapper_only_args: Vec<ReportArgDeltaV1> = Vec::new();

    // Upstream → missing/unsupported/iu/passthrough
    for (path, cmd) in upstream {
        if !present_on_filter(
            &cmd.available_on,
            &report_target_set,
            &rules.union.expected_targets,
            filter_mode,
        ) {
            continue;
        }

        if let Some(ex) = parity_exclusions.and_then(|idx| idx.commands.get(path)) {
            let cmd_res = wrapper::resolve_wrapper(
                wrapper_index
                    .commands
                    .get(path)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]),
                &report_target_set,
                &rules.union.expected_targets,
                filter_mode,
                "command",
                &format!("path={}", util::format_path(path)),
            )?;
            excluded_commands.push(ReportCommandDeltaV1 {
                path: path.clone(),
                upstream_available_on: cmd.available_on.clone(),
                wrapper_level: cmd_res.level.clone(),
                note: Some(ex.note.clone()),
            });
            continue;
        }

        let cmd_res = wrapper::resolve_wrapper(
            wrapper_index
                .commands
                .get(path)
                .map(Vec::as_slice)
                .unwrap_or(&[]),
            &report_target_set,
            &rules.union.expected_targets,
            filter_mode,
            "command",
            &format!("path={}", util::format_path(path)),
        )?;

        if cmd_res.level.is_none() {
            if let Some(root) = find_inherited_iu_root(
                &iu_roots,
                path,
                &cmd.available_on,
                &report_target_set,
                "command",
            )? {
                intentionally_unsupported.push(ReportIntentionallyUnsupportedDeltaV1::Command(
                    ReportCommandDeltaV1 {
                        path: path.clone(),
                        upstream_available_on: cmd.available_on.clone(),
                        wrapper_level: Some("intentionally_unsupported".to_string()),
                        note: Some(root.note.clone()),
                    },
                ));
            } else {
                classify_command_delta(
                    &mut missing_commands,
                    &mut passthrough_candidates,
                    &mut unsupported,
                    path,
                    &cmd.available_on,
                    &cmd_res,
                );
            }
        } else if cmd_res.level.as_deref() == Some("intentionally_unsupported") {
            let note = require_non_empty_note(
                cmd_res.note.as_deref(),
                "command",
                &format!("path={}", util::format_path(path)),
            )?;
            intentionally_unsupported.push(ReportIntentionallyUnsupportedDeltaV1::Command(
                ReportCommandDeltaV1 {
                    path: path.clone(),
                    upstream_available_on: cmd.available_on.clone(),
                    wrapper_level: Some("intentionally_unsupported".to_string()),
                    note: Some(note),
                },
            ));
        } else {
            classify_command_delta(
                &mut missing_commands,
                &mut passthrough_candidates,
                &mut unsupported,
                path,
                &cmd.available_on,
                &cmd_res,
            );
        }

        for flag in &cmd.flags {
            if !present_on_filter(
                &flag.available_on,
                &report_target_set,
                &rules.union.expected_targets,
                filter_mode,
            ) {
                continue;
            }
            if let Some(ex) =
                parity_exclusions.and_then(|idx| idx.flags.get(&(path.clone(), flag.key.clone())))
            {
                let key = (path.clone(), flag.key.clone());
                let res = wrapper::resolve_wrapper(
                    wrapper_index
                        .flags
                        .get(&key)
                        .map(Vec::as_slice)
                        .unwrap_or(&[]),
                    &report_target_set,
                    &rules.union.expected_targets,
                    filter_mode,
                    "flag",
                    &format!("path={} key={}", util::format_path(path), flag.key),
                )?;
                excluded_flags.push(ReportFlagDeltaV1 {
                    path: path.clone(),
                    key: flag.key.clone(),
                    upstream_available_on: flag.available_on.clone(),
                    wrapper_level: res.level.clone(),
                    note: Some(ex.note.clone()),
                });
                continue;
            }
            let key = (path.clone(), flag.key.clone());
            let res = wrapper::resolve_wrapper(
                wrapper_index
                    .flags
                    .get(&key)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]),
                &report_target_set,
                &rules.union.expected_targets,
                filter_mode,
                "flag",
                &format!("path={} key={}", util::format_path(path), flag.key),
            )?;
            if res.level.is_none() {
                if let Some(root) = find_inherited_iu_root(
                    &iu_roots,
                    path,
                    &flag.available_on,
                    &report_target_set,
                    "flag",
                )? {
                    intentionally_unsupported.push(ReportIntentionallyUnsupportedDeltaV1::Flag(
                        ReportFlagDeltaV1 {
                            path: path.to_vec(),
                            key: flag.key.clone(),
                            upstream_available_on: flag.available_on.clone(),
                            wrapper_level: Some("intentionally_unsupported".to_string()),
                            note: Some(root.note.clone()),
                        },
                    ));
                } else {
                    classify_flag_delta(&mut missing_flags, path, flag, &res);
                }
            } else if res.level.as_deref() == Some("intentionally_unsupported") {
                let note = require_non_empty_note(
                    res.note.as_deref(),
                    "flag",
                    &format!("path={} key={}", util::format_path(path), flag.key),
                )?;
                intentionally_unsupported.push(ReportIntentionallyUnsupportedDeltaV1::Flag(
                    ReportFlagDeltaV1 {
                        path: path.to_vec(),
                        key: flag.key.clone(),
                        upstream_available_on: flag.available_on.clone(),
                        wrapper_level: Some("intentionally_unsupported".to_string()),
                        note: Some(note),
                    },
                ));
            } else {
                classify_flag_delta(&mut missing_flags, path, flag, &res);
            }
        }

        for arg in &cmd.args {
            if !present_on_filter(
                &arg.available_on,
                &report_target_set,
                &rules.union.expected_targets,
                filter_mode,
            ) {
                continue;
            }
            if let Some(ex) =
                parity_exclusions.and_then(|idx| idx.args.get(&(path.clone(), arg.name.clone())))
            {
                let key = (path.clone(), arg.name.clone());
                let res = wrapper::resolve_wrapper(
                    wrapper_index
                        .args
                        .get(&key)
                        .map(Vec::as_slice)
                        .unwrap_or(&[]),
                    &report_target_set,
                    &rules.union.expected_targets,
                    filter_mode,
                    "arg",
                    &format!("path={} name={}", util::format_path(path), arg.name),
                )?;
                excluded_args.push(ReportArgDeltaV1 {
                    path: path.clone(),
                    name: arg.name.clone(),
                    upstream_available_on: arg.available_on.clone(),
                    wrapper_level: res.level.clone(),
                    note: Some(ex.note.clone()),
                });
                continue;
            }
            let key = (path.clone(), arg.name.clone());
            let res = wrapper::resolve_wrapper(
                wrapper_index
                    .args
                    .get(&key)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]),
                &report_target_set,
                &rules.union.expected_targets,
                filter_mode,
                "arg",
                &format!("path={} name={}", util::format_path(path), arg.name),
            )?;
            if res.level.is_none() {
                if let Some(root) = find_inherited_iu_root(
                    &iu_roots,
                    path,
                    &arg.available_on,
                    &report_target_set,
                    "arg",
                )? {
                    intentionally_unsupported.push(ReportIntentionallyUnsupportedDeltaV1::Arg(
                        ReportArgDeltaV1 {
                            path: path.to_vec(),
                            name: arg.name.clone(),
                            upstream_available_on: arg.available_on.clone(),
                            wrapper_level: Some("intentionally_unsupported".to_string()),
                            note: Some(root.note.clone()),
                        },
                    ));
                } else {
                    classify_arg_delta(&mut missing_args, path, arg, &res);
                }
            } else if res.level.as_deref() == Some("intentionally_unsupported") {
                let note = require_non_empty_note(
                    res.note.as_deref(),
                    "arg",
                    &format!("path={} name={}", util::format_path(path), arg.name),
                )?;
                intentionally_unsupported.push(ReportIntentionallyUnsupportedDeltaV1::Arg(
                    ReportArgDeltaV1 {
                        path: path.to_vec(),
                        name: arg.name.clone(),
                        upstream_available_on: arg.available_on.clone(),
                        wrapper_level: Some("intentionally_unsupported".to_string()),
                        note: Some(note),
                    },
                ));
            } else {
                classify_arg_delta(&mut missing_args, path, arg, &res);
            }
        }
    }

    // Wrapper → wrapper-only (relative to platform filter semantics)
    for (path, entries) in &wrapper_index.commands {
        let res = wrapper::resolve_wrapper(
            entries,
            &report_target_set,
            &rules.union.expected_targets,
            filter_mode,
            "command",
            &format!("path={}", util::format_path(path)),
        )?;
        if !res.present {
            continue;
        }

        let upstream_avail = upstream
            .get(path)
            .map(|c| c.available_on.clone())
            .unwrap_or_else(|| util::ordered_subset(&rules.union.expected_targets, &res.targets));
        let upstream_present = upstream.get(path).is_some_and(|c| {
            present_on_filter(
                &c.available_on,
                &report_target_set,
                &rules.union.expected_targets,
                filter_mode,
            )
        });

        if !upstream_present {
            wrapper_only_commands.push(ReportCommandDeltaV1 {
                path: path.clone(),
                upstream_available_on: upstream_avail,
                wrapper_level: res.level.clone(),
                note: res.note.clone(),
            });
        }
    }

    for ((path, key), entries) in &wrapper_index.flags {
        let res = wrapper::resolve_wrapper(
            entries,
            &report_target_set,
            &rules.union.expected_targets,
            filter_mode,
            "flag",
            &format!("path={} key={key}", util::format_path(path)),
        )?;
        if !res.present {
            continue;
        }
        let (upstream_avail, upstream_present) = upstream_flag_availability(
            upstream,
            path,
            key,
            &res,
            &report_target_set,
            &rules.union.expected_targets,
            filter_mode,
        );
        if !upstream_present {
            wrapper_only_flags.push(ReportFlagDeltaV1 {
                path: path.clone(),
                key: key.clone(),
                upstream_available_on: upstream_avail,
                wrapper_level: res.level.clone(),
                note: res.note.clone(),
            });
        }
    }

    for ((path, name), entries) in &wrapper_index.args {
        let res = wrapper::resolve_wrapper(
            entries,
            &report_target_set,
            &rules.union.expected_targets,
            filter_mode,
            "arg",
            &format!("path={} name={name}", util::format_path(path)),
        )?;
        if !res.present {
            continue;
        }
        let (upstream_avail, upstream_present) = upstream_arg_availability(
            upstream,
            path,
            name,
            &res,
            &report_target_set,
            &rules.union.expected_targets,
            filter_mode,
        );
        if !upstream_present {
            wrapper_only_args.push(ReportArgDeltaV1 {
                path: path.clone(),
                name: name.clone(),
                upstream_available_on: upstream_avail,
                wrapper_level: res.level.clone(),
                note: res.note.clone(),
            });
        }
    }

    missing_commands.sort_by(|a, b| util::cmp_path(&a.path, &b.path));
    missing_flags.sort_by(|a, b| util::cmp_path(&a.path, &b.path).then_with(|| a.key.cmp(&b.key)));
    missing_args.sort_by(|a, b| util::cmp_path(&a.path, &b.path).then_with(|| a.name.cmp(&b.name)));

    excluded_commands.sort_by(|a, b| util::cmp_path(&a.path, &b.path));
    excluded_flags.sort_by(|a, b| util::cmp_path(&a.path, &b.path).then_with(|| a.key.cmp(&b.key)));
    excluded_args
        .sort_by(|a, b| util::cmp_path(&a.path, &b.path).then_with(|| a.name.cmp(&b.name)));

    passthrough_candidates.sort_by(|a, b| util::cmp_path(&a.path, &b.path));
    unsupported.sort_by(|a, b| util::cmp_path(&a.path, &b.path));
    intentionally_unsupported.sort_by(cmp_iu_delta);

    wrapper_only_commands.sort_by(|a, b| util::cmp_path(&a.path, &b.path));
    wrapper_only_flags
        .sort_by(|a, b| util::cmp_path(&a.path, &b.path).then_with(|| a.key.cmp(&b.key)));
    wrapper_only_args
        .sort_by(|a, b| util::cmp_path(&a.path, &b.path).then_with(|| a.name.cmp(&b.name)));

    let deltas = ReportDeltasV1 {
        missing_commands,
        missing_flags,
        missing_args,
        excluded_commands: if excluded_commands.is_empty() {
            None
        } else {
            Some(excluded_commands)
        },
        excluded_flags: if excluded_flags.is_empty() {
            None
        } else {
            Some(excluded_flags)
        },
        excluded_args: if excluded_args.is_empty() {
            None
        } else {
            Some(excluded_args)
        },
        passthrough_candidates: if passthrough_candidates.is_empty() {
            None
        } else {
            Some(passthrough_candidates)
        },
        unsupported: if unsupported.is_empty() {
            None
        } else {
            Some(unsupported)
        },
        intentionally_unsupported: if intentionally_unsupported.is_empty() {
            None
        } else {
            Some(intentionally_unsupported)
        },
        wrapper_only_commands: if wrapper_only_commands.is_empty() {
            None
        } else {
            Some(wrapper_only_commands)
        },
        wrapper_only_flags: if wrapper_only_flags.is_empty() {
            None
        } else {
            Some(wrapper_only_flags)
        },
        wrapper_only_args: if wrapper_only_args.is_empty() {
            None
        } else {
            Some(wrapper_only_args)
        },
    };

    Ok(CoverageReportV1 {
        schema_version: 1,
        generated_at: generated_at.to_string(),
        inputs: ReportInputsV1 {
            upstream: ReportUpstreamInputsV1 {
                semantic_version: version.to_string(),
                mode: "union".to_string(),
                targets: report_targets.to_vec(),
            },
            wrapper: ReportWrapperInputsV1 {
                schema_version: wrapper.schema_version,
                wrapper_version: wrapper.wrapper_version.clone(),
            },
            rules: ReportRulesInputsV1 {
                rules_schema_version: rules.rules_schema_version,
            },
        },
        platform_filter: PlatformFilterV1 {
            mode: platform_mode.to_string(),
            target_triple: target_triple.map(ToString::to_string),
        },
        deltas,
    })
}
