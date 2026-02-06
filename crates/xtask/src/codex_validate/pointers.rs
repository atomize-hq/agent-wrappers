use std::{fs, io, path::Path};

use regex::Regex;

use super::{
    parse_stable_version, rel_path, FatalError, PointerRead, PointerValue, PointerValues,
    ValidateCtx, Violation,
};

pub(super) fn normalize_single_line_file(path: &Path) -> Result<(), FatalError> {
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e.into()),
    };
    let content =
        String::from_utf8(bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let line = content
        .split('\n')
        .next()
        .unwrap_or("")
        .trim_end_matches('\r');
    fs::write(path, format!("{line}\n"))?;
    Ok(())
}

pub(super) fn validate_pointers(
    ctx: &mut ValidateCtx,
    violations: &mut Vec<Violation>,
) -> PointerValues {
    let mut out = PointerValues::default();

    let min_supported_path = ctx.root.join("min_supported.txt");
    match read_pointer_file(&min_supported_path, &ctx.stable_semver_re, false) {
        Ok(PointerRead::Value(PointerValue::Version(ver))) => {
            out.min_supported = Some(ver.to_string());
        }
        Ok(PointerRead::Missing) => violations.push(pointer_violation(
            ctx,
            "POINTER_MISSING_FILE",
            &min_supported_path,
            "missing required file: min_supported.txt",
        )),
        Ok(PointerRead::InvalidFormat { reason }) => violations.push(pointer_violation(
            ctx,
            "POINTER_INVALID_FORMAT",
            &min_supported_path,
            &format!(
                "invalid pointer file format ({reason}); expected single line + trailing newline"
            ),
        )),
        Ok(PointerRead::InvalidValue { raw }) => violations.push(pointer_violation(
            ctx,
            "POINTER_INVALID_VALUE",
            &min_supported_path,
            &format!(
                "invalid pointer value (got {raw}); expected strict stable semver MAJOR.MINOR.PATCH"
            ),
        )),
        Ok(PointerRead::Value(PointerValue::None)) => violations.push(pointer_violation(
            ctx,
            "POINTER_INVALID_VALUE",
            &min_supported_path,
            "invalid pointer value (got none); expected strict stable semver MAJOR.MINOR.PATCH",
        )),
        Err(e) => violations.push(pointer_violation(
            ctx,
            "POINTER_UNREADABLE",
            &min_supported_path,
            &format!("failed to read pointer file: {e}"),
        )),
    }

    let latest_validated_path = ctx.root.join("latest_validated.txt");
    match read_pointer_file(&latest_validated_path, &ctx.stable_semver_re, false) {
        Ok(PointerRead::Value(PointerValue::Version(ver))) => {
            out.latest_validated = Some(ver.to_string());
        }
        Ok(PointerRead::Missing) => violations.push(pointer_violation(
            ctx,
            "POINTER_MISSING_FILE",
            &latest_validated_path,
            "missing required file: latest_validated.txt",
        )),
        Ok(PointerRead::InvalidFormat { reason }) => violations.push(pointer_violation(
            ctx,
            "POINTER_INVALID_FORMAT",
            &latest_validated_path,
            &format!(
                "invalid pointer file format ({reason}); expected single line + trailing newline"
            ),
        )),
        Ok(PointerRead::InvalidValue { raw }) => violations.push(pointer_violation(
            ctx,
            "POINTER_INVALID_VALUE",
            &latest_validated_path,
            &format!(
                "invalid pointer value (got {raw}); expected strict stable semver MAJOR.MINOR.PATCH"
            ),
        )),
        Ok(PointerRead::Value(PointerValue::None)) => violations.push(pointer_violation(
            ctx,
            "POINTER_INVALID_VALUE",
            &latest_validated_path,
            "invalid pointer value (got none); expected strict stable semver MAJOR.MINOR.PATCH",
        )),
        Err(e) => violations.push(pointer_violation(
            ctx,
            "POINTER_UNREADABLE",
            &latest_validated_path,
            &format!("failed to read pointer file: {e}"),
        )),
    }

    for target in ctx.expected_targets.clone() {
        let supported_path = ctx
            .root
            .join("pointers/latest_supported")
            .join(format!("{target}.txt"));
        let validated_path = ctx
            .root
            .join("pointers/latest_validated")
            .join(format!("{target}.txt"));

        let supported = match read_pointer_file(&supported_path, &ctx.stable_semver_re, true) {
            Ok(PointerRead::Value(PointerValue::None)) => None,
            Ok(PointerRead::Value(PointerValue::Version(ver))) => Some(ver.to_string()),
            Ok(PointerRead::Missing) => {
                violations.push(Violation {
                    code: "POINTER_MISSING_FILE",
                    path: rel_path(&ctx.root, &supported_path),
                    json_pointer: None,
                    message: format!(
                        "missing pointer file for target_triple={target} kind=latest_supported"
                    ),
                    unit: Some("pointers"),
                    command_path: None,
                    key_or_name: Some(target.clone()),
                    field: Some("latest_supported"),
                    target_triple: Some(target.clone()),
                    details: None,
                });
                None
            }
            Ok(PointerRead::InvalidFormat { reason }) => {
                violations.push(Violation {
                    code: "POINTER_INVALID_FORMAT",
                    path: rel_path(&ctx.root, &supported_path),
                    json_pointer: None,
                    message: format!(
                        "invalid pointer file format ({reason}); expected single line + trailing newline (target_triple={target} kind=latest_supported)"
                    ),
                    unit: Some("pointers"),
                    command_path: None,
                    key_or_name: Some(target.clone()),
                    field: Some("latest_supported"),
                    target_triple: Some(target.clone()),
                    details: None,
                });
                None
            }
            Ok(PointerRead::InvalidValue { raw }) => {
                violations.push(Violation {
                    code: "POINTER_INVALID_VALUE",
                    path: rel_path(&ctx.root, &supported_path),
                    json_pointer: None,
                    message: format!(
                        "invalid pointer value (got {raw}); expected none or strict stable semver MAJOR.MINOR.PATCH (target_triple={target} kind=latest_supported)"
                    ),
                    unit: Some("pointers"),
                    command_path: None,
                    key_or_name: Some(target.clone()),
                    field: Some("latest_supported"),
                    target_triple: Some(target.clone()),
                    details: None,
                });
                None
            }
            Err(e) => {
                violations.push(Violation {
                    code: "POINTER_UNREADABLE",
                    path: rel_path(&ctx.root, &supported_path),
                    json_pointer: None,
                    message: format!(
                        "failed to read pointer file: {e} (target_triple={target} kind=latest_supported)"
                    ),
                    unit: Some("pointers"),
                    command_path: None,
                    key_or_name: Some(target.clone()),
                    field: Some("latest_supported"),
                    target_triple: Some(target.clone()),
                    details: None,
                });
                None
            }
        };
        out.by_target_latest_supported
            .insert(target.clone(), supported);

        let validated = match read_pointer_file(&validated_path, &ctx.stable_semver_re, true) {
            Ok(PointerRead::Value(PointerValue::None)) => None,
            Ok(PointerRead::Value(PointerValue::Version(ver))) => Some(ver.to_string()),
            Ok(PointerRead::Missing) => {
                violations.push(Violation {
                    code: "POINTER_MISSING_FILE",
                    path: rel_path(&ctx.root, &validated_path),
                    json_pointer: None,
                    message: format!(
                        "missing pointer file for target_triple={target} kind=latest_validated"
                    ),
                    unit: Some("pointers"),
                    command_path: None,
                    key_or_name: Some(target.clone()),
                    field: Some("latest_validated"),
                    target_triple: Some(target.clone()),
                    details: None,
                });
                None
            }
            Ok(PointerRead::InvalidFormat { reason }) => {
                violations.push(Violation {
                    code: "POINTER_INVALID_FORMAT",
                    path: rel_path(&ctx.root, &validated_path),
                    json_pointer: None,
                    message: format!(
                        "invalid pointer file format ({reason}); expected single line + trailing newline (target_triple={target} kind=latest_validated)"
                    ),
                    unit: Some("pointers"),
                    command_path: None,
                    key_or_name: Some(target.clone()),
                    field: Some("latest_validated"),
                    target_triple: Some(target.clone()),
                    details: None,
                });
                None
            }
            Ok(PointerRead::InvalidValue { raw }) => {
                violations.push(Violation {
                    code: "POINTER_INVALID_VALUE",
                    path: rel_path(&ctx.root, &validated_path),
                    json_pointer: None,
                    message: format!(
                        "invalid pointer value (got {raw}); expected none or strict stable semver MAJOR.MINOR.PATCH (target_triple={target} kind=latest_validated)"
                    ),
                    unit: Some("pointers"),
                    command_path: None,
                    key_or_name: Some(target.clone()),
                    field: Some("latest_validated"),
                    target_triple: Some(target.clone()),
                    details: None,
                });
                None
            }
            Err(e) => {
                violations.push(Violation {
                    code: "POINTER_UNREADABLE",
                    path: rel_path(&ctx.root, &validated_path),
                    json_pointer: None,
                    message: format!(
                        "failed to read pointer file: {e} (target_triple={target} kind=latest_validated)"
                    ),
                    unit: Some("pointers"),
                    command_path: None,
                    key_or_name: Some(target.clone()),
                    field: Some("latest_validated"),
                    target_triple: Some(target.clone()),
                    details: None,
                });
                None
            }
        };
        out.by_target_latest_validated
            .insert(target.clone(), validated);
    }

    // latest_validated.txt must equal pointers/latest_validated/<required_target>.txt and must not be none.
    if let Some(latest_validated) = out.latest_validated.clone() {
        let required_ptr = ctx
            .root
            .join("pointers/latest_validated")
            .join(format!("{}.txt", ctx.required_target));
        let required_value = match read_pointer_file(&required_ptr, &ctx.stable_semver_re, true) {
            Ok(PointerRead::Value(PointerValue::Version(ver))) => Some(ver.to_string()),
            _ => None,
        };
        if required_value.as_deref() != Some(latest_validated.as_str()) {
            violations.push(Violation {
                code: "POINTER_LATEST_VALIDATED_MISMATCH",
                path: rel_path(&ctx.root, &latest_validated_path),
                json_pointer: None,
                message: format!(
                    "latest_validated.txt must equal pointers/latest_validated/{}.txt and must not be none (latest_validated={}, required_target_value={})",
                    ctx.required_target,
                    latest_validated,
                    required_value.unwrap_or_else(|| "none".to_string())
                ),
                unit: Some("pointers"),
                command_path: None,
                key_or_name: Some(ctx.required_target.clone()),
                field: Some("latest_validated"),
                target_triple: Some(ctx.required_target.clone()),
                details: None,
            });
        }
    }

    out
}

fn pointer_violation(
    ctx: &ValidateCtx,
    code: &'static str,
    path: &Path,
    message: &str,
) -> Violation {
    Violation {
        code,
        path: rel_path(&ctx.root, path),
        json_pointer: None,
        message: message.to_string(),
        unit: Some("pointers"),
        command_path: None,
        key_or_name: None,
        field: None,
        target_triple: None,
        details: None,
    }
}

pub(super) fn read_pointer_file(
    path: &Path,
    stable_semver_re: &Regex,
    allow_none: bool,
) -> Result<PointerRead, FatalError> {
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(PointerRead::Missing),
        Err(e) => return Err(e.into()),
    };

    let content =
        std::str::from_utf8(&bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    if !content.ends_with('\n') {
        return Ok(PointerRead::InvalidFormat {
            reason: "missing trailing newline",
        });
    }

    let without_nl = &content[..content.len() - 1];
    if without_nl.contains('\n') {
        return Ok(PointerRead::InvalidFormat {
            reason: "multiple lines",
        });
    }
    if without_nl.contains('\r') {
        return Ok(PointerRead::InvalidFormat {
            reason: "contains CR character",
        });
    }
    if without_nl != without_nl.trim() {
        return Ok(PointerRead::InvalidFormat {
            reason: "contains leading/trailing whitespace",
        });
    }

    if without_nl == "none" {
        if allow_none {
            return Ok(PointerRead::Value(PointerValue::None));
        }
        return Ok(PointerRead::InvalidValue {
            raw: "none".to_string(),
        });
    }

    let Some(ver) = parse_stable_version(without_nl, stable_semver_re) else {
        return Ok(PointerRead::InvalidValue {
            raw: without_nl.to_string(),
        });
    };
    Ok(PointerRead::Value(PointerValue::Version(ver)))
}
