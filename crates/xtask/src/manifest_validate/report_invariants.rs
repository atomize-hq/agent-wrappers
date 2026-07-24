use std::path::Path;

use serde_json::Value;

use super::{format_command_path, rel_path, schema, IuSortKey, ValidateCtx, Violation};

pub(super) fn require_report(
    ctx: &ValidateCtx,
    violations: &mut Vec<Violation>,
    version: &str,
    kind: &'static str,
    target: Option<&str>,
    path: &Path,
) {
    match schema::read_json_file(&ctx.root, path, violations, "REPORT_INVALID_JSON") {
        Some(value) => {
            schema::schema_validate(
                ctx,
                violations,
                &ctx.schema,
                &value,
                path,
                "REPORT_SCHEMA_INVALID",
            );
            validate_report_exclusions(ctx, violations, &value, path);
            validate_report_intentionally_unsupported(ctx, violations, &value, path);
        }
        None => {
            if path.exists() {
                return;
            }
            violations.push(Violation {
                code: "REPORT_MISSING",
                path: rel_path(&ctx.root, path),
                json_pointer: None,
                message: match target {
                    Some(t) => format!(
                        "missing required file: reports/{version}/{} (kind={kind} target_triple={t})",
                        path.file_name().and_then(|x| x.to_str()).unwrap_or("<unknown>")
                    ),
                    None => format!(
                        "missing required file: reports/{version}/{} (kind={kind})",
                        path.file_name().and_then(|x| x.to_str()).unwrap_or("<unknown>")
                    ),
                },
                unit: Some("reports"),
                command_path: None,
                key_or_name: target.map(|t| t.to_string()),
                field: Some("reports"),
                target_triple: target.map(|t| t.to_string()),
                details: None,
            });
        }
    }
}

pub(super) fn validate_report_if_present(
    ctx: &ValidateCtx,
    violations: &mut Vec<Violation>,
    path: &Path,
) {
    let Some(value) = schema::read_json_file(&ctx.root, path, violations, "REPORT_INVALID_JSON")
    else {
        return;
    };

    schema::schema_validate(
        ctx,
        violations,
        &ctx.schema,
        &value,
        path,
        "REPORT_SCHEMA_INVALID",
    );
    validate_report_exclusions(ctx, violations, &value, path);
    validate_report_intentionally_unsupported(ctx, violations, &value, path);
}

fn validate_report_exclusions(
    ctx: &ValidateCtx,
    violations: &mut Vec<Violation>,
    report: &Value,
    report_path: &Path,
) {
    let Some(index) = ctx.parity_exclusions.as_ref() else {
        return;
    };
    let Some(deltas) = report.get("deltas") else {
        return;
    };

    let missing_commands = deltas.get("missing_commands").and_then(Value::as_array);
    let missing_flags = deltas.get("missing_flags").and_then(Value::as_array);
    let missing_args = deltas.get("missing_args").and_then(Value::as_array);

    if let Some(items) = missing_commands {
        for (i, item) in items.iter().enumerate() {
            let path = item.get("path").and_then(Value::as_array).map(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            });
            let Some(path) = path else { continue };
            if index.commands.contains_key(&path) {
                violations.push(Violation {
                    code: "REPORT_MISSING_INCLUDES_EXCLUDED",
                    path: rel_path(&ctx.root, report_path),
                    json_pointer: Some(format!("/deltas/missing_commands/{i}")),
                    message: format!(
                        "report missing_commands includes excluded command_path={}",
                        format_command_path(&path)
                    ),
                    unit: Some("reports"),
                    command_path: Some(format_command_path(&path)),
                    key_or_name: None,
                    field: Some("missing_commands"),
                    target_triple: None,
                    details: None,
                });
            }
        }
    }

    if let Some(items) = missing_flags {
        for (i, item) in items.iter().enumerate() {
            let path = item.get("path").and_then(Value::as_array).map(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            });
            let key = item.get("key").and_then(Value::as_str).map(str::to_string);
            let (Some(path), Some(key)) = (path, key) else {
                continue;
            };
            if let Some(ex) = index.flags.get(&(path.clone(), key.clone())) {
                violations.push(Violation {
                    code: "REPORT_MISSING_INCLUDES_EXCLUDED",
                    path: rel_path(&ctx.root, report_path),
                    json_pointer: Some(format!("/deltas/missing_flags/{i}")),
                    message: format!(
                        "report missing_flags includes excluded flag (command_path={} key={} category={})",
                        format_command_path(&path),
                        key,
                        ex.category.clone().unwrap_or_else(|| "<missing>".to_string())
                    ),
                    unit: Some("reports"),
                    command_path: Some(format_command_path(&path)),
                    key_or_name: Some(key),
                    field: Some("missing_flags"),
                    target_triple: None,
                    details: None,
                });
            }
        }
    }

    if let Some(items) = missing_args {
        for (i, item) in items.iter().enumerate() {
            let path = item.get("path").and_then(Value::as_array).map(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            });
            let name = item.get("name").and_then(Value::as_str).map(str::to_string);
            let (Some(path), Some(name)) = (path, name) else {
                continue;
            };
            if let Some(ex) = index.args.get(&(path.clone(), name.clone())) {
                violations.push(Violation {
                    code: "REPORT_MISSING_INCLUDES_EXCLUDED",
                    path: rel_path(&ctx.root, report_path),
                    json_pointer: Some(format!("/deltas/missing_args/{i}")),
                    message: format!(
                        "report missing_args includes excluded arg (command_path={} name={} category={})",
                        format_command_path(&path),
                        name,
                        ex.category.clone().unwrap_or_else(|| "<missing>".to_string())
                    ),
                    unit: Some("reports"),
                    command_path: Some(format_command_path(&path)),
                    key_or_name: Some(name),
                    field: Some("missing_args"),
                    target_triple: None,
                    details: None,
                });
            }
        }
    }
}

fn cmp_path_tokens(a: &[String], b: &[String]) -> std::cmp::Ordering {
    let mut i = 0usize;
    while i < a.len() && i < b.len() {
        match a[i].cmp(&b[i]) {
            std::cmp::Ordering::Equal => i += 1,
            non_eq => return non_eq,
        }
    }
    a.len().cmp(&b.len())
}

fn cmp_iu_sort_key(a: &IuSortKey, b: &IuSortKey) -> std::cmp::Ordering {
    a.kind_rank
        .cmp(&b.kind_rank)
        .then_with(|| cmp_path_tokens(&a.path, &b.path))
        .then_with(|| a.key_or_name.cmp(&b.key_or_name))
}

fn validate_report_intentionally_unsupported(
    ctx: &ValidateCtx,
    violations: &mut Vec<Violation>,
    report: &Value,
    report_path: &Path,
) {
    let Some(deltas) = report.get("deltas") else {
        return;
    };

    for (list_name, items) in [
        (
            "missing_commands",
            deltas.get("missing_commands").and_then(Value::as_array),
        ),
        (
            "missing_flags",
            deltas.get("missing_flags").and_then(Value::as_array),
        ),
        (
            "missing_args",
            deltas.get("missing_args").and_then(Value::as_array),
        ),
    ] {
        let Some(items) = items else { continue };
        for (i, item) in items.iter().enumerate() {
            if item
                .get("wrapper_level")
                .and_then(Value::as_str)
                .is_some_and(|s| s == "intentionally_unsupported")
            {
                violations.push(Violation {
                    code: "REPORT_MISSING_INCLUDES_INTENTIONALLY_UNSUPPORTED",
                    path: rel_path(&ctx.root, report_path),
                    json_pointer: Some(format!("/deltas/{list_name}/{i}")),
                    message: format!(
                        "report {list_name} includes wrapper_level=intentionally_unsupported"
                    ),
                    unit: Some("reports"),
                    command_path: item.get("path").and_then(Value::as_array).map(|arr| {
                        arr.iter()
                            .filter_map(Value::as_str)
                            .collect::<Vec<_>>()
                            .join(" ")
                    }),
                    key_or_name: item
                        .get("key")
                        .and_then(Value::as_str)
                        .or_else(|| item.get("name").and_then(Value::as_str))
                        .map(|s| s.to_string()),
                    field: Some(list_name),
                    target_triple: None,
                    details: None,
                });
            }
        }
    }

    let Some(iu_items) = deltas
        .get("intentionally_unsupported")
        .and_then(Value::as_array)
    else {
        return;
    };

    let mut keys = Vec::new();
    for (i, item) in iu_items.iter().enumerate() {
        let path = item.get("path").and_then(Value::as_array).map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>()
        });
        let Some(path) = path else { continue };

        let kind_rank = if item.get("key").is_some() {
            1u8
        } else if item.get("name").is_some() {
            2u8
        } else {
            0u8
        };

        let key_or_name = item
            .get("key")
            .and_then(Value::as_str)
            .or_else(|| item.get("name").and_then(Value::as_str))
            .unwrap_or("")
            .to_string();

        if item.get("wrapper_level").and_then(Value::as_str) != Some("intentionally_unsupported") {
            violations.push(Violation {
                code: "REPORT_IU_NOTE_MISSING",
                path: rel_path(&ctx.root, report_path),
                json_pointer: Some(format!("/deltas/intentionally_unsupported/{i}/wrapper_level")),
                message:
                    "intentionally_unsupported entry must have wrapper_level=intentionally_unsupported"
                        .to_string(),
                unit: Some("reports"),
                command_path: Some(format_command_path(&path)),
                key_or_name: if key_or_name.is_empty() {
                    None
                } else {
                    Some(key_or_name.clone())
                },
                field: Some("intentionally_unsupported"),
                target_triple: None,
                details: None,
            });
        }

        if item
            .get("note")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .is_empty()
        {
            violations.push(Violation {
                code: "REPORT_IU_NOTE_MISSING",
                path: rel_path(&ctx.root, report_path),
                json_pointer: Some(format!("/deltas/intentionally_unsupported/{i}/note")),
                message: "intentionally_unsupported entry requires non-empty note".to_string(),
                unit: Some("reports"),
                command_path: Some(format_command_path(&path)),
                key_or_name: if key_or_name.is_empty() {
                    None
                } else {
                    Some(key_or_name.clone())
                },
                field: Some("note"),
                target_triple: None,
                details: None,
            });
        }

        keys.push(IuSortKey {
            kind_rank,
            path,
            key_or_name,
        });
    }

    let mut sorted = keys.clone();
    sorted.sort_by(cmp_iu_sort_key);
    if keys != sorted {
        violations.push(Violation {
            code: "REPORT_IU_NOT_SORTED",
            path: rel_path(&ctx.root, report_path),
            json_pointer: Some("/deltas/intentionally_unsupported".to_string()),
            message:
                "deltas.intentionally_unsupported must be stable-sorted by (kind,path,key_or_name)"
                    .to_string(),
            unit: Some("reports"),
            command_path: None,
            key_or_name: None,
            field: Some("intentionally_unsupported"),
            target_triple: None,
            details: None,
        });
    }
}
