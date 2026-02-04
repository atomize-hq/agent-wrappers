use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

use serde_json::json;

use super::{
    format_command_path, rel_path, schema, ValidateCtx, Violation, WrapperCoverageFile,
    WrapperScope,
};

pub(super) fn validate_wrapper_coverage(ctx: &mut ValidateCtx, violations: &mut Vec<Violation>) {
    let path = ctx.root.join("wrapper_coverage.json");
    let value = match schema::read_json_file(
        &ctx.root,
        &path,
        violations,
        "WRAPPER_COVERAGE_INVALID_JSON",
    ) {
        Some(v) => {
            schema::schema_validate(
                ctx,
                violations,
                &ctx.schema,
                &v,
                &path,
                "WRAPPER_COVERAGE_SCHEMA_INVALID",
            );
            v
        }
        None => {
            if path.exists() {
                return;
            }
            violations.push(Violation {
                code: "WRAPPER_COVERAGE_MISSING",
                path: rel_path(&ctx.root, &path),
                json_pointer: None,
                message: "missing required file: wrapper_coverage.json".to_string(),
                unit: Some("wrapper_coverage"),
                command_path: None,
                key_or_name: None,
                field: Some("wrapper_coverage"),
                target_triple: None,
                details: None,
            });
            return;
        }
    };

    let parsed: WrapperCoverageFile = match serde_json::from_value(value) {
        Ok(v) => v,
        Err(e) => {
            violations.push(Violation {
                code: "WRAPPER_COVERAGE_PARSE_FAILED",
                path: rel_path(&ctx.root, &path),
                json_pointer: None,
                message: format!("failed to parse wrapper_coverage.json for semantic checks: {e}"),
                unit: Some("wrapper_coverage"),
                command_path: None,
                key_or_name: None,
                field: Some("wrapper_coverage"),
                target_triple: None,
                details: None,
            });
            return;
        }
    };

    if parsed.schema_version != 1 {
        violations.push(Violation {
            code: "WRAPPER_COVERAGE_SCHEMA_VERSION",
            path: rel_path(&ctx.root, &path),
            json_pointer: Some("/schema_version".to_string()),
            message: format!(
                "wrapper_coverage.json schema_version must be 1 (got {})",
                parsed.schema_version
            ),
            unit: Some("wrapper_coverage"),
            command_path: None,
            key_or_name: None,
            field: Some("schema_version"),
            target_triple: None,
            details: None,
        });
    }

    validate_wrapper_coverage_exclusions(ctx, violations, &parsed, &path);
    validate_wrapper_iu_notes(ctx, violations, &parsed, &path);
    validate_wrapper_scope_overlaps(ctx, violations, &parsed, &path);
}

fn validate_wrapper_iu_notes(
    ctx: &ValidateCtx,
    violations: &mut Vec<Violation>,
    wc: &WrapperCoverageFile,
    path: &Path,
) {
    for (cmd_idx, cmd) in wc.coverage.iter().enumerate() {
        if cmd.level == "intentionally_unsupported"
            && cmd.note.as_deref().unwrap_or("").trim().is_empty()
        {
            violations.push(Violation {
                code: "IU_NOTE_MISSING",
                path: rel_path(&ctx.root, path),
                json_pointer: Some(format!("/coverage/{cmd_idx}/note")),
                message: format!(
                    "intentionally_unsupported requires non-empty note (unit=command command_path={})",
                    format_command_path(&cmd.path)
                ),
                unit: Some("wrapper_command"),
                command_path: Some(format_command_path(&cmd.path)),
                key_or_name: None,
                field: Some("note"),
                target_triple: None,
                details: None,
            });
        }
        for (flag_idx, flag) in cmd.flags.as_deref().unwrap_or(&[]).iter().enumerate() {
            if flag.level == "intentionally_unsupported"
                && flag.note.as_deref().unwrap_or("").trim().is_empty()
            {
                violations.push(Violation {
                    code: "IU_NOTE_MISSING",
                    path: rel_path(&ctx.root, path),
                    json_pointer: Some(format!("/coverage/{cmd_idx}/flags/{flag_idx}/note")),
                    message: format!(
                        "intentionally_unsupported requires non-empty note (unit=flag command_path={} key={})",
                        format_command_path(&cmd.path),
                        flag.key
                    ),
                    unit: Some("wrapper_flag"),
                    command_path: Some(format_command_path(&cmd.path)),
                    key_or_name: Some(flag.key.clone()),
                    field: Some("note"),
                    target_triple: None,
                    details: None,
                });
            }
        }
        for (arg_idx, arg) in cmd.args.as_deref().unwrap_or(&[]).iter().enumerate() {
            if arg.level == "intentionally_unsupported"
                && arg.note.as_deref().unwrap_or("").trim().is_empty()
            {
                violations.push(Violation {
                    code: "IU_NOTE_MISSING",
                    path: rel_path(&ctx.root, path),
                    json_pointer: Some(format!("/coverage/{cmd_idx}/args/{arg_idx}/note")),
                    message: format!(
                        "intentionally_unsupported requires non-empty note (unit=arg command_path={} name={})",
                        format_command_path(&cmd.path),
                        arg.name
                    ),
                    unit: Some("wrapper_arg"),
                    command_path: Some(format_command_path(&cmd.path)),
                    key_or_name: Some(arg.name.clone()),
                    field: Some("note"),
                    target_triple: None,
                    details: None,
                });
            }
        }
    }
}

fn validate_wrapper_scope_overlaps(
    ctx: &ValidateCtx,
    violations: &mut Vec<Violation>,
    wc: &WrapperCoverageFile,
    path: &Path,
) {
    if !ctx.wrapper_rules.validation.disallow_overlapping_scopes {
        return;
    }

    let expected = ctx
        .expected_targets
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();

    // command_path overlap
    if ctx
        .wrapper_rules
        .validation
        .overlap_units
        .iter()
        .any(|u| u == "command_path")
    {
        let mut by_cmd = BTreeMap::<String, Vec<super::ScopedEntry>>::new();
        for (cmd_idx, cmd) in wc.coverage.iter().enumerate() {
            let cmd_targets = scope_to_targets(ctx, &expected, cmd.scope.as_ref());
            by_cmd
                .entry(format_command_path(&cmd.path))
                .or_default()
                .push(super::ScopedEntry {
                    index: format!("coverage[{cmd_idx}]"),
                    scope_kind: scope_kind(cmd.scope.as_ref()),
                    targets: cmd_targets,
                });
        }
        for (cmd_path, entries) in by_cmd {
            detect_overlaps(
                ctx,
                violations,
                path,
                "WRAPPER_SCOPE_OVERLAP",
                "command_path",
                &cmd_path,
                None,
                entries,
            );
        }
    }

    // command_flag_key overlap
    if ctx
        .wrapper_rules
        .validation
        .overlap_units
        .iter()
        .any(|u| u == "command_flag_key")
    {
        let mut by_flag = BTreeMap::<(String, String), Vec<super::ScopedEntry>>::new();
        for (cmd_idx, cmd) in wc.coverage.iter().enumerate() {
            let cmd_targets = scope_to_targets(ctx, &expected, cmd.scope.as_ref());
            for (flag_idx, flag) in cmd.flags.as_deref().unwrap_or(&[]).iter().enumerate() {
                let flag_targets = scope_to_targets(ctx, &expected, flag.scope.as_ref());
                let effective = intersect(&cmd_targets, &flag_targets);
                by_flag
                    .entry((format_command_path(&cmd.path), flag.key.clone()))
                    .or_default()
                    .push(super::ScopedEntry {
                        index: format!("coverage[{cmd_idx}].flags[{flag_idx}]"),
                        scope_kind: scope_kind(flag.scope.as_ref()),
                        targets: effective,
                    });
            }
        }
        for ((cmd_path, key), entries) in by_flag {
            detect_overlaps(
                ctx,
                violations,
                path,
                "WRAPPER_SCOPE_OVERLAP",
                "flag",
                &cmd_path,
                Some(key),
                entries,
            );
        }
    }

    // command_arg_name overlap
    if ctx
        .wrapper_rules
        .validation
        .overlap_units
        .iter()
        .any(|u| u == "command_arg_name")
    {
        let mut by_arg = BTreeMap::<(String, String), Vec<super::ScopedEntry>>::new();
        for (cmd_idx, cmd) in wc.coverage.iter().enumerate() {
            let cmd_targets = scope_to_targets(ctx, &expected, cmd.scope.as_ref());
            for (arg_idx, arg) in cmd.args.as_deref().unwrap_or(&[]).iter().enumerate() {
                let arg_targets = scope_to_targets(ctx, &expected, arg.scope.as_ref());
                let effective = intersect(&cmd_targets, &arg_targets);
                by_arg
                    .entry((format_command_path(&cmd.path), arg.name.clone()))
                    .or_default()
                    .push(super::ScopedEntry {
                        index: format!("coverage[{cmd_idx}].args[{arg_idx}]"),
                        scope_kind: scope_kind(arg.scope.as_ref()),
                        targets: effective,
                    });
            }
        }
        for ((cmd_path, name), entries) in by_arg {
            detect_overlaps(
                ctx,
                violations,
                path,
                "WRAPPER_SCOPE_OVERLAP",
                "arg",
                &cmd_path,
                Some(name),
                entries,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn detect_overlaps(
    ctx: &ValidateCtx,
    violations: &mut Vec<Violation>,
    path: &Path,
    code: &'static str,
    unit: &'static str,
    cmd_path: &str,
    key_or_name: Option<String>,
    entries: Vec<super::ScopedEntry>,
) {
    if entries.len() <= 1 {
        return;
    }
    for target in &ctx.expected_targets {
        let matching = entries
            .iter()
            .filter(|e| e.targets.contains(target))
            .collect::<Vec<_>>();
        if matching.len() <= 1 {
            continue;
        }
        let indexes = matching
            .iter()
            .map(|e| e.index.as_str())
            .collect::<Vec<_>>()
            .join(",");
        let mut msg = format!(
            "overlapping wrapper_coverage scopes are manifest-invalid (unit={unit} command_path={cmd_path} target_triple={target} matching_entry_indexes=[{indexes}])"
        );
        if let Some(k) = key_or_name.as_deref() {
            msg.push_str(&format!(" key_or_name={k}"));
        }
        violations.push(Violation {
            code,
            path: rel_path(&ctx.root, path),
            json_pointer: None,
            message: msg,
            unit: Some("wrapper_coverage"),
            command_path: Some(cmd_path.to_string()),
            key_or_name: key_or_name.clone(),
            field: Some("scope"),
            target_triple: Some(target.to_string()),
            details: Some(json!({
                "matching_entry_indexes": matching.iter().map(|e| e.index.clone()).collect::<Vec<_>>(),
                "matching_entry_scope_kinds": matching.iter().map(|e| e.scope_kind).collect::<Vec<_>>(),
            })),
        });
    }
}

fn scope_kind(scope: Option<&WrapperScope>) -> &'static str {
    let Some(scope) = scope else {
        return "no_scope";
    };
    if scope.target_triples.as_ref().is_some_and(|v| !v.is_empty()) {
        return "target_triples";
    }
    if scope.platforms.as_ref().is_some_and(|v| !v.is_empty()) {
        return "platforms";
    }
    "no_scope"
}

fn scope_to_targets(
    ctx: &ValidateCtx,
    expected: &BTreeSet<String>,
    scope: Option<&WrapperScope>,
) -> BTreeSet<String> {
    let Some(scope) = scope else {
        return expected.clone();
    };

    let mut out = BTreeSet::<String>::new();
    if let Some(tt) = scope.target_triples.as_ref() {
        for t in tt {
            if expected.contains(t) {
                out.insert(t.clone());
            }
        }
    }
    if let Some(platforms) = scope.platforms.as_ref() {
        if ctx
            .wrapper_rules
            .scope_semantics
            .platforms_expand_to_expected_targets
        {
            for t in expected {
                if let Some(p) = ctx.platform_mapping.get(t) {
                    if platforms.iter().any(|pl| pl == p) {
                        out.insert(t.clone());
                    }
                }
            }
        }
    }

    if out.is_empty() {
        // Treat an empty/unknown scope as applying to no expected targets.
        return BTreeSet::new();
    }
    out
}

fn intersect(a: &BTreeSet<String>, b: &BTreeSet<String>) -> BTreeSet<String> {
    a.intersection(b).cloned().collect()
}

fn validate_wrapper_coverage_exclusions(
    ctx: &ValidateCtx,
    violations: &mut Vec<Violation>,
    wc: &WrapperCoverageFile,
    wc_path: &Path,
) {
    let Some(index) = ctx.parity_exclusions.as_ref() else {
        return;
    };

    for (cmd_idx, cmd) in wc.coverage.iter().enumerate() {
        if let Some(ex) = index.commands.get(&cmd.path) {
            violations.push(Violation {
                code: "WRAPPER_COVERAGE_INCLUDES_EXCLUDED",
                path: rel_path(&ctx.root, wc_path),
                json_pointer: Some(format!("/coverage/{cmd_idx}")),
                message: format!(
                    "wrapper_coverage includes excluded command_path={} (note={})",
                    format_command_path(&cmd.path),
                    ex.note
                ),
                unit: Some("wrapper_command"),
                command_path: Some(format_command_path(&cmd.path)),
                key_or_name: None,
                field: Some("coverage"),
                target_triple: None,
                details: None,
            });
        }

        for (flag_idx, flag) in cmd.flags.as_deref().unwrap_or(&[]).iter().enumerate() {
            if let Some(ex) = index.flags.get(&(cmd.path.clone(), flag.key.clone())) {
                violations.push(Violation {
                    code: "WRAPPER_COVERAGE_INCLUDES_EXCLUDED",
                    path: rel_path(&ctx.root, wc_path),
                    json_pointer: Some(format!("/coverage/{cmd_idx}/flags/{flag_idx}")),
                    message: format!(
                        "wrapper_coverage includes excluded flag (command_path={} key={} note={})",
                        format_command_path(&cmd.path),
                        flag.key,
                        ex.note
                    ),
                    unit: Some("wrapper_flag"),
                    command_path: Some(format_command_path(&cmd.path)),
                    key_or_name: Some(flag.key.clone()),
                    field: Some("flags"),
                    target_triple: None,
                    details: None,
                });
            }
        }

        for (arg_idx, arg) in cmd.args.as_deref().unwrap_or(&[]).iter().enumerate() {
            if let Some(ex) = index.args.get(&(cmd.path.clone(), arg.name.clone())) {
                violations.push(Violation {
                    code: "WRAPPER_COVERAGE_INCLUDES_EXCLUDED",
                    path: rel_path(&ctx.root, wc_path),
                    json_pointer: Some(format!("/coverage/{cmd_idx}/args/{arg_idx}")),
                    message: format!(
                        "wrapper_coverage includes excluded arg (command_path={} name={} note={})",
                        format_command_path(&cmd.path),
                        arg.name,
                        ex.note
                    ),
                    unit: Some("wrapper_arg"),
                    command_path: Some(format_command_path(&cmd.path)),
                    key_or_name: Some(arg.name.clone()),
                    field: Some("args"),
                    target_triple: None,
                    details: None,
                });
            }
        }
    }
}
