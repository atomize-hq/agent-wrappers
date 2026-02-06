use std::fs;

use serde_json::Value;

use super::{is_union_snapshot, rel_path, schema, ValidateCtx, Violation};

pub(super) fn validate_current_json(
    ctx: &mut ValidateCtx,
    violations: &mut Vec<Violation>,
    latest_validated: Option<&str>,
) {
    let current_path = ctx.root.join("current.json");
    let current_value = match schema::read_json_file(
        &ctx.root,
        &current_path,
        violations,
        "CURRENT_INVALID_JSON",
    ) {
        Some(v) => {
            schema::schema_validate(
                ctx,
                violations,
                &ctx.schema,
                &v,
                &current_path,
                "CURRENT_SCHEMA_INVALID",
            );
            if !is_union_snapshot(&v) {
                violations.push(Violation {
                    code: "CURRENT_WRONG_KIND",
                    path: rel_path(&ctx.root, &current_path),
                    json_pointer: Some("/snapshot_schema_version".to_string()),
                    message: "current.json must be an UpstreamSnapshotUnionV2 (snapshot_schema_version=2, mode=union)".to_string(),
                    unit: Some("current_json"),
                    command_path: None,
                    key_or_name: None,
                    field: Some("current"),
                    target_triple: None,
                    details: None,
                });
            }
            Some(v)
        }
        None => {
            if current_path.exists() {
                return;
            }
            violations.push(Violation {
                code: "CURRENT_MISSING",
                path: rel_path(&ctx.root, &current_path),
                json_pointer: None,
                message: "missing required file: current.json".to_string(),
                unit: Some("current_json"),
                command_path: None,
                key_or_name: None,
                field: Some("current"),
                target_triple: None,
                details: None,
            });
            None
        }
    };

    let Some(latest_validated) = latest_validated else {
        return;
    };
    let union_path = ctx
        .root
        .join("snapshots")
        .join(latest_validated)
        .join("union.json");

    if current_path.is_file() && union_path.is_file() {
        if let (Ok(a), Ok(b)) = (fs::read(&current_path), fs::read(&union_path)) {
            if a != b {
                violations.push(Violation {
                    code: "CURRENT_JSON_NOT_EQUAL_UNION",
                    path: rel_path(&ctx.root, &current_path),
                    json_pointer: None,
                    message: format!(
                        "current.json must be byte-for-byte identical to snapshots/{latest_validated}/union.json"
                    ),
                    unit: Some("current_json"),
                    command_path: None,
                    key_or_name: Some(latest_validated.to_string()),
                    field: Some("identity"),
                    target_triple: None,
                    details: None,
                });
            }
        }
    }

    // current.json semantic version invariants use the required target's input.binary.semantic_version.
    let Some(current_value) = current_value else {
        return;
    };
    let required_target = ctx.required_target.clone();
    let required_input = current_value
        .get("inputs")
        .and_then(Value::as_array)
        .and_then(|inputs| {
            inputs.iter().find(|i| {
                i.get("target_triple")
                    .and_then(Value::as_str)
                    .is_some_and(|t| t == required_target.as_str())
            })
        });
    let Some(required_input) = required_input else {
        violations.push(Violation {
            code: "CURRENT_JSON_MISSING_REQUIRED_TARGET",
            path: rel_path(&ctx.root, &current_path),
            json_pointer: Some("/inputs".to_string()),
            message: format!("current.json.inputs[] missing required_target={required_target}"),
            unit: Some("current_json"),
            command_path: None,
            key_or_name: Some(required_target.clone()),
            field: Some("inputs"),
            target_triple: Some(required_target),
            details: None,
        });
        return;
    };
    let semantic_version = required_input
        .get("binary")
        .and_then(|b| b.get("semantic_version"))
        .and_then(Value::as_str);
    if semantic_version != Some(latest_validated) {
        violations.push(Violation {
            code: "CURRENT_JSON_SEMVER_MISMATCH",
            path: rel_path(&ctx.root, &current_path),
            json_pointer: Some("/inputs/*/binary/semantic_version".to_string()),
            message: format!(
                "current.json required_target binary.semantic_version must equal latest_validated.txt (expected {latest_validated}, got {})",
                semantic_version.unwrap_or("<missing>")
            ),
            unit: Some("current_json"),
            command_path: None,
            key_or_name: Some(required_target.clone()),
            field: Some("semantic_version"),
            target_triple: Some(required_target),
            details: None,
        });
    }
}
