use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use super::{rel_path, PointerValues, ValidateCtx, Violation};

pub(super) fn validate_pointer_consistency(
    ctx: &ValidateCtx,
    violations: &mut Vec<Violation>,
    pointers: &PointerValues,
    version_metadata: &BTreeMap<String, Value>,
) {
    for (target, v) in &pointers.by_target_latest_supported {
        let Some(version) = v.as_deref() else {
            continue;
        };
        let meta = version_metadata.get(version);
        if meta.is_none() {
            continue;
        }
        let supported_targets = meta
            .and_then(|m| m.get("coverage"))
            .and_then(|c| c.get("supported_targets"))
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .map(|s| s.to_string())
                    .collect::<BTreeSet<_>>()
            })
            .unwrap_or_default();
        if !supported_targets.contains(target) {
            violations.push(Violation {
                code: "POINTER_INCONSISTENT_WITH_VERSION_METADATA",
                path: rel_path(&ctx.root, &ctx.root.join("versions").join(format!("{version}.json"))),
                json_pointer: Some("/coverage/supported_targets".to_string()),
                message: format!(
                    "pointers/latest_supported/{target}.txt={version} requires versions/{version}.json.coverage.supported_targets to include target_triple={target}"
                ),
                unit: Some("pointers"),
                command_path: None,
                key_or_name: Some(target.clone()),
                field: Some("latest_supported"),
                target_triple: Some(target.clone()),
                details: None,
            });
        }
    }

    for (target, v) in &pointers.by_target_latest_validated {
        let Some(version) = v.as_deref() else {
            continue;
        };
        let meta = version_metadata.get(version);
        if meta.is_none() {
            continue;
        }
        let supported_targets = meta
            .and_then(|m| m.get("coverage"))
            .and_then(|c| c.get("supported_targets"))
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .map(|s| s.to_string())
                    .collect::<BTreeSet<_>>()
            })
            .unwrap_or_default();
        let passed_targets = meta
            .and_then(|m| m.get("validation"))
            .and_then(|v| v.get("passed_targets"))
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .map(|s| s.to_string())
                    .collect::<BTreeSet<_>>()
            })
            .unwrap_or_default();

        if !supported_targets.contains(target) || !passed_targets.contains(target) {
            violations.push(Violation {
                code: "POINTER_INCONSISTENT_WITH_VERSION_METADATA",
                path: rel_path(&ctx.root, &ctx.root.join("versions").join(format!("{version}.json"))),
                json_pointer: Some("/validation/passed_targets".to_string()),
                message: format!(
                    "pointers/latest_validated/{target}.txt={version} requires versions/{version}.json.coverage.supported_targets and versions/{version}.json.validation.passed_targets to include target_triple={target}"
                ),
                unit: Some("pointers"),
                command_path: None,
                key_or_name: Some(target.clone()),
                field: Some("latest_validated"),
                target_triple: Some(target.clone()),
                details: None,
            });
        }
    }
}
