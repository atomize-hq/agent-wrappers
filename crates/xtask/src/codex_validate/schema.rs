use std::{fs, io, path::Path};

use jsonschema::JSONSchema;
use serde_json::Value;

use super::{rel_path, FatalError, ValidateCtx, Violation};

pub(super) fn absolutize_schema_id(
    schema: &mut Value,
    schema_path: &Path,
) -> Result<(), FatalError> {
    let Some(id) = schema.get("$id").and_then(|v| v.as_str()) else {
        return Ok(());
    };

    // `jsonschema` expects `$id` to be an absolute URI. Our committed schemas use
    // repo-relative `$id` values (e.g. `cli_manifests/codex/SCHEMA.json`) for
    // readability, so rewrite them to a file URI at runtime.
    if id.contains("://") {
        return Ok(());
    }

    let abs = fs::canonicalize(schema_path)?;
    let abs_str = abs.to_string_lossy();
    let file_uri = if abs_str.starts_with('/') {
        format!("file://{abs_str}")
    } else {
        format!("file:///{abs_str}")
    };

    if let Some(obj) = schema.as_object_mut() {
        obj.insert("$id".to_string(), Value::String(file_uri));
    }

    Ok(())
}

pub(super) fn schema_validate(
    ctx: &ValidateCtx,
    violations: &mut Vec<Violation>,
    schema: &JSONSchema,
    instance: &Value,
    path: &Path,
    code: &'static str,
) {
    if let Err(errors) = schema.validate(instance) {
        let mut errs = errors
            .map(|e| {
                let ptr = e.instance_path.to_string();
                (ptr, e.to_string())
            })
            .collect::<Vec<_>>();
        errs.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        for (ptr, msg) in errs {
            violations.push(Violation {
                code,
                path: rel_path(&ctx.root, path),
                json_pointer: if ptr.is_empty() { None } else { Some(ptr) },
                message: msg,
                unit: Some("schemas"),
                command_path: None,
                key_or_name: None,
                field: Some("schema"),
                target_triple: None,
                details: None,
            });
        }
    }
}

pub(super) fn read_json_file(
    root: &Path,
    path: &Path,
    violations: &mut Vec<Violation>,
    code_invalid_json: &'static str,
) -> Option<Value> {
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return None,
        Err(e) => {
            violations.push(Violation {
                code: "FILE_UNREADABLE",
                path: rel_path(root, path),
                json_pointer: None,
                message: format!("failed to read file: {e}"),
                unit: None,
                command_path: None,
                key_or_name: None,
                field: None,
                target_triple: None,
                details: None,
            });
            return None;
        }
    };
    match serde_json::from_slice::<Value>(&bytes) {
        Ok(v) => Some(v),
        Err(e) => {
            violations.push(Violation {
                code: code_invalid_json,
                path: rel_path(root, path),
                json_pointer: None,
                message: format!("invalid JSON: {e}"),
                unit: Some("schemas"),
                command_path: None,
                key_or_name: None,
                field: Some("json"),
                target_triple: None,
                details: None,
            });
            None
        }
    }
}
