use std::{
    collections::{BTreeMap, BTreeSet},
    fs, io,
    path::Path,
};

use semver::Version;
use serde_json::{json, Value};

use super::{
    is_per_target_snapshot, is_union_snapshot, parse_stable_version, rel_path, report_invariants,
    schema, PointerValues, ValidateCtx, Violation,
};

pub(super) fn compute_versions_to_validate(
    ctx: &mut ValidateCtx,
    violations: &mut Vec<Violation>,
    pointers: &PointerValues,
) -> Vec<String> {
    let mut versions = BTreeSet::<Version>::new();

    for v in pointers
        .min_supported
        .iter()
        .chain(pointers.latest_validated.iter())
    {
        if let Some(ver) = parse_stable_version(v, &ctx.stable_semver_re) {
            versions.insert(ver);
        }
    }
    for (_target, v) in pointers
        .by_target_latest_supported
        .iter()
        .chain(pointers.by_target_latest_validated.iter())
    {
        if let Some(v) = v {
            if let Some(ver) = parse_stable_version(v, &ctx.stable_semver_re) {
                versions.insert(ver);
            }
        }
    }

    let versions_dir = ctx.root.join("versions");
    match fs::read_dir(&versions_dir) {
        Ok(read_dir) => {
            let mut entries = read_dir
                .filter_map(|e| e.ok())
                .filter_map(|e| {
                    let path = e.path();
                    if path.extension().and_then(|x| x.to_str()) != Some("json") {
                        return None;
                    }
                    let stem = path.file_stem()?.to_str()?.to_string();
                    Some((stem, path))
                })
                .collect::<Vec<_>>();
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            for (stem, path) in entries {
                match parse_stable_version(&stem, &ctx.stable_semver_re) {
                    Some(ver) => {
                        versions.insert(ver);
                    }
                    None => violations.push(Violation {
                        code: "VERSION_FILE_INVALID_NAME",
                        path: rel_path(&ctx.root, &path),
                        json_pointer: None,
                        message: format!(
                            "versions/<version>.json filename must be a strict stable semver (got {stem})"
                        ),
                        unit: Some("versions"),
                        command_path: None,
                        key_or_name: Some(stem),
                        field: Some("filename"),
                        target_triple: None,
                        details: None,
                    }),
                }
            }
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => {
            violations.push(Violation {
                code: "VERSIONS_DIR_UNREADABLE",
                path: rel_path(&ctx.root, &versions_dir),
                json_pointer: None,
                message: format!("failed to read versions directory: {e}"),
                unit: Some("versions"),
                command_path: None,
                key_or_name: None,
                field: None,
                target_triple: None,
                details: None,
            });
        }
    }

    versions.into_iter().map(|v| v.to_string()).collect()
}

pub(super) fn validate_version_bundle(
    ctx: &mut ValidateCtx,
    violations: &mut Vec<Violation>,
    version: &str,
    version_metadata: &mut BTreeMap<String, Value>,
) {
    let version_path = ctx.root.join("versions").join(format!("{version}.json"));
    match schema::read_json_file(
        &ctx.root,
        &version_path,
        violations,
        "VERSION_METADATA_INVALID_JSON",
    ) {
        Some(value) => {
            schema::schema_validate(
                ctx,
                violations,
                &ctx.version_schema,
                &value,
                &version_path,
                "VERSION_METADATA_SCHEMA_INVALID",
            );
            validate_version_metadata_validation_sets(
                ctx,
                violations,
                version,
                &value,
                &version_path,
            );
            version_metadata.insert(version.to_string(), value);
        }
        None => {
            if !version_path.exists() {
                violations.push(Violation {
                    code: "VERSION_METADATA_MISSING",
                    path: rel_path(&ctx.root, &version_path),
                    json_pointer: None,
                    message: format!("missing required file: versions/{version}.json"),
                    unit: Some("versions"),
                    command_path: None,
                    key_or_name: Some(version.to_string()),
                    field: Some("versions"),
                    target_triple: None,
                    details: None,
                });
            }
        }
    }

    let union_path = ctx.root.join("snapshots").join(version).join("union.json");
    let union_value = match schema::read_json_file(
        &ctx.root,
        &union_path,
        violations,
        "UNION_INVALID_JSON",
    ) {
        Some(value) => {
            schema::schema_validate(
                ctx,
                violations,
                &ctx.schema,
                &value,
                &union_path,
                "UNION_SCHEMA_INVALID",
            );
            if !is_union_snapshot(&value) {
                violations.push(Violation {
                    code: "UNION_WRONG_KIND",
                    path: rel_path(&ctx.root, &union_path),
                    json_pointer: Some("/snapshot_schema_version".to_string()),
                    message: "snapshots/<version>/union.json must be an UpstreamSnapshotUnionV2 (snapshot_schema_version=2, mode=union)".to_string(),
                    unit: Some("snapshots"),
                    command_path: None,
                    key_or_name: Some(version.to_string()),
                    field: Some("union"),
                    target_triple: None,
                    details: None,
                });
            }
            Some(value)
        }
        None => {
            if !union_path.exists() {
                violations.push(Violation {
                    code: "UNION_MISSING",
                    path: rel_path(&ctx.root, &union_path),
                    json_pointer: None,
                    message: format!("missing required file: snapshots/{version}/union.json"),
                    unit: Some("snapshots"),
                    command_path: None,
                    key_or_name: Some(version.to_string()),
                    field: Some("union"),
                    target_triple: None,
                    details: None,
                });
            }
            None
        }
    };

    let inputs = union_value
        .as_ref()
        .and_then(|u| u.get("inputs"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut input_targets = Vec::<String>::new();
    for input in &inputs {
        if let Some(t) = input.get("target_triple").and_then(Value::as_str) {
            input_targets.push(t.to_string());
        }
    }

    for target in &input_targets {
        let per_target_path = ctx
            .root
            .join("snapshots")
            .join(version)
            .join(format!("{target}.json"));
        match schema::read_json_file(
            &ctx.root,
            &per_target_path,
            violations,
            "SNAPSHOT_INVALID_JSON",
        ) {
            Some(value) => {
                schema::schema_validate(
                    ctx,
                    violations,
                    &ctx.schema,
                    &value,
                    &per_target_path,
                    "SNAPSHOT_SCHEMA_INVALID",
                );
                if !is_per_target_snapshot(&value) {
                    violations.push(Violation {
                        code: "SNAPSHOT_WRONG_KIND",
                        path: rel_path(&ctx.root, &per_target_path),
                        json_pointer: Some("/snapshot_schema_version".to_string()),
                        message: "snapshots/<version>/<target_triple>.json must be an UpstreamSnapshotV1 (snapshot_schema_version=1)".to_string(),
                        unit: Some("snapshots"),
                        command_path: None,
                        key_or_name: Some(target.to_string()),
                        field: Some("per_target"),
                        target_triple: Some(target.to_string()),
                        details: None,
                    });
                }
            }
            None => {
                if per_target_path.exists() {
                    continue;
                }
                violations.push(Violation {
                    code: "SNAPSHOT_MISSING",
                    path: rel_path(&ctx.root, &per_target_path),
                    json_pointer: None,
                    message: format!(
                        "missing required file: snapshots/{version}/{target}.json (referenced by union.inputs[])"
                    ),
                    unit: Some("snapshots"),
                    command_path: None,
                    key_or_name: Some(target.to_string()),
                    field: Some("per_target"),
                    target_triple: Some(target.to_string()),
                    details: None,
                });
            }
        }
    }

    // Reports are required depending on version status.
    let status = version_metadata
        .get(version)
        .and_then(|v| v.get("status"))
        .and_then(Value::as_str)
        .unwrap_or("unknown");

    let require_reports = matches!(status, "reported" | "validated" | "supported");
    let reports_dir = ctx.root.join("reports").join(version);
    let any_report = reports_dir.join("coverage.any.json");
    if require_reports {
        report_invariants::require_report(ctx, violations, version, "any", None, &any_report);
    } else {
        report_invariants::validate_report_if_present(ctx, violations, &any_report);
    }

    for target in &input_targets {
        let per_target = reports_dir.join(format!("coverage.{target}.json"));
        if require_reports {
            report_invariants::require_report(
                ctx,
                violations,
                version,
                "per_target",
                Some(target.as_str()),
                &per_target,
            );
        } else {
            report_invariants::validate_report_if_present(ctx, violations, &per_target);
        }
    }

    let complete = union_value
        .as_ref()
        .and_then(|u| u.get("complete"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if complete {
        let all_report = reports_dir.join("coverage.all.json");
        if require_reports {
            report_invariants::require_report(ctx, violations, version, "all", None, &all_report);
        } else {
            report_invariants::validate_report_if_present(ctx, violations, &all_report);
        }
    }
}

fn intersect(a: &BTreeSet<String>, b: &BTreeSet<String>) -> BTreeSet<String> {
    a.intersection(b).cloned().collect()
}

fn validate_version_metadata_validation_sets(
    ctx: &ValidateCtx,
    violations: &mut Vec<Violation>,
    version: &str,
    meta: &Value,
    path: &Path,
) {
    let Some(validation) = meta.get("validation") else {
        return;
    };

    let expected = ctx
        .expected_targets
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();

    let passed = validation
        .get("passed_targets")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .map(|s| s.to_string())
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();
    let failed = validation
        .get("failed_targets")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .map(|s| s.to_string())
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();
    let skipped = validation
        .get("skipped_targets")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .map(|s| s.to_string())
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();

    let overlaps = [
        (
            "passed_targets",
            "failed_targets",
            intersect(&passed, &failed),
        ),
        (
            "passed_targets",
            "skipped_targets",
            intersect(&passed, &skipped),
        ),
        (
            "failed_targets",
            "skipped_targets",
            intersect(&failed, &skipped),
        ),
    ];
    for (a, b, inter) in overlaps {
        if inter.is_empty() {
            continue;
        }
        violations.push(Violation {
            code: "VALIDATION_TARGET_SETS_OVERLAP",
            path: rel_path(&ctx.root, path),
            json_pointer: Some("/validation".to_string()),
            message: format!(
                "versions/{version}.json validation target sets overlap ({a} ∩ {b} = {:?})",
                inter.iter().collect::<Vec<_>>()
            ),
            unit: Some("versions"),
            command_path: None,
            key_or_name: Some(version.to_string()),
            field: Some("validation"),
            target_triple: None,
            details: Some(json!({
                "overlap": inter.into_iter().collect::<Vec<_>>(),
                "a": a,
                "b": b,
            })),
        });
    }

    for t in passed.iter().chain(failed.iter()).chain(skipped.iter()) {
        if expected.contains(t) {
            continue;
        }
        violations.push(Violation {
            code: "VALIDATION_TARGET_NOT_EXPECTED",
            path: rel_path(&ctx.root, path),
            json_pointer: Some("/validation".to_string()),
            message: format!(
                "versions/{version}.json validation includes unexpected target_triple={t} (not in RULES.json.union.expected_targets)"
            ),
            unit: Some("versions"),
            command_path: None,
            key_or_name: Some(version.to_string()),
            field: Some("validation"),
            target_triple: Some(t.to_string()),
            details: None,
        });
    }

    let required = ctx.required_target.as_str();
    let count = (passed.contains(required) as u8)
        + (failed.contains(required) as u8)
        + (skipped.contains(required) as u8);
    if count != 1 {
        violations.push(Violation {
            code: "VALIDATION_REQUIRED_TARGET_NOT_EXPLICIT",
            path: rel_path(&ctx.root, path),
            json_pointer: Some("/validation".to_string()),
            message: format!(
                "versions/{version}.json validation must include required_target={} in exactly one of passed_targets/failed_targets/skipped_targets",
                ctx.required_target
            ),
            unit: Some("versions"),
            command_path: None,
            key_or_name: Some(version.to_string()),
            field: Some("validation"),
            target_triple: Some(ctx.required_target.clone()),
            details: Some(json!({
                "required_target": ctx.required_target,
                "passed": passed.contains(required),
                "failed": failed.contains(required),
                "skipped": skipped.contains(required),
            })),
        });
    }
}
