use std::{
    collections::{BTreeMap, BTreeSet},
    fs, io,
    path::{Path, PathBuf},
};

use clap::{Parser, ValueEnum};
use jsonschema::{Draft, JSONSchema};
use regex::Regex;
use semver::Version;
use serde_json::{json, Value};
use thiserror::Error;

mod current;
mod fix_mode;
mod models;
mod pointer_consistency;
mod pointers;
mod report_invariants;
mod schema;
mod versions;
mod wrapper_coverage;
use models::{
    IuSortKey, ParityExclusionUnit, ParityExclusionsIndex, PointerRead, PointerValue,
    PointerValues, Rules, RulesWrapperCoverage, ScopedEntry, Violation, WrapperCoverageFile,
    WrapperScope,
};

#[derive(Debug, Parser)]
pub struct Args {
    /// Root directory containing `SCHEMA.json`, `RULES.json`, pointer files, snapshots, reports,
    /// and version metadata.
    #[arg(long, default_value = "cli_manifests/codex", alias = "codex-dir")]
    pub root: PathBuf,

    /// Path to `RULES.json`.
    #[arg(long)]
    pub rules: Option<PathBuf>,

    /// Path to `SCHEMA.json`.
    #[arg(long)]
    pub schema: Option<PathBuf>,

    /// Path to `VERSION_METADATA_SCHEMA.json`.
    #[arg(long, alias = "version-metadata-schema")]
    pub version_schema: Option<PathBuf>,

    /// Validation mode.
    #[arg(long, value_enum, default_value_t = Mode::Check)]
    pub mode: Mode,

    /// Emit a machine-readable JSON report to stdout in addition to human text.
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Copy, Clone, ValueEnum)]
pub enum Mode {
    Check,
    Fix,
}

#[derive(Debug, Error)]
pub enum FatalError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("failed to parse JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("failed to compile JSON Schema: {0}")]
    SchemaCompile(String),
    #[error("invalid RULES.json: {0}")]
    Rules(String),
}

#[derive(Debug)]
struct ValidateCtx {
    root: PathBuf,
    required_target: String,
    expected_targets: Vec<String>,
    platform_mapping: BTreeMap<String, String>,
    stable_semver_re: Regex,
    schema: JSONSchema,
    version_schema: JSONSchema,
    wrapper_rules: RulesWrapperCoverage,
    parity_exclusions_schema_version: Option<u32>,
    parity_exclusions_raw: Option<Vec<ParityExclusionUnit>>,
    parity_exclusions: Option<ParityExclusionsIndex>,
}

pub fn run(args: Args) -> i32 {
    let json_out = args.json;
    match run_inner(args) {
        Ok(violations) => {
            if json_out {
                let out = json!({
                    "ok": violations.is_empty(),
                    "violations": violations.iter().map(Violation::to_json).collect::<Vec<_>>(),
                });
                println!(
                    "{}",
                    serde_json::to_string_pretty(&out).unwrap_or_else(|_| "{}".to_string())
                );
            }

            if violations.is_empty() {
                if json_out {
                    eprintln!("OK: codex-validate");
                } else {
                    println!("OK: codex-validate");
                }
                0
            } else {
                eprintln!("FAIL: {} violations", violations.len());
                for v in &violations {
                    eprintln!("{}", v.to_human_line());
                }
                2
            }
        }
        Err(err) => {
            eprintln!("FAIL: codex-validate ({err})");
            3
        }
    }
}

fn run_inner(args: Args) -> Result<Vec<Violation>, FatalError> {
    let root = args.root;
    let rules_path = args.rules.unwrap_or_else(|| root.join("RULES.json"));
    let schema_path = args.schema.unwrap_or_else(|| root.join("SCHEMA.json"));
    let version_schema_path = args
        .version_schema
        .unwrap_or_else(|| root.join("VERSION_METADATA_SCHEMA.json"));

    let rules: Rules = serde_json::from_slice(&fs::read(&rules_path)?)?;
    let stable_semver_re =
        Regex::new(&rules.versioning.pointers.stable_semver_pattern).map_err(|e| {
            FatalError::Rules(format!(
                "invalid versioning.pointers.stable_semver_pattern: {e}"
            ))
        })?;

    // Guardrails: wrapper rules are designed around expanding platform labels into target triples
    // using the union's platform mapping.
    if rules
        .wrapper_coverage
        .scope_semantics
        .platforms_expand_using
        != "union.platform_mapping"
    {
        return Err(FatalError::Rules(format!(
            "unsupported wrapper_coverage.scope_semantics.platforms_expand_using={} (expected union.platform_mapping)",
            rules.wrapper_coverage.scope_semantics.platforms_expand_using
        )));
    }
    if rules
        .wrapper_coverage
        .scope_semantics
        .defaults
        .no_scope_means
        != "all_expected_targets"
    {
        return Err(FatalError::Rules(format!(
            "unsupported wrapper_coverage.scope_semantics.defaults.no_scope_means={} (expected all_expected_targets)",
            rules.wrapper_coverage.scope_semantics.defaults.no_scope_means
        )));
    }
    if rules
        .wrapper_coverage
        .scope_semantics
        .scope_set_resolution
        .mode
        != "union"
    {
        return Err(FatalError::Rules(format!(
            "unsupported wrapper_coverage.scope_semantics.scope_set_resolution.mode={} (expected union)",
            rules.wrapper_coverage.scope_semantics.scope_set_resolution.mode
        )));
    }

    let mut schema_value: Value = serde_json::from_slice(&fs::read(&schema_path)?)?;
    let mut version_schema_value: Value = serde_json::from_slice(&fs::read(&version_schema_path)?)?;

    schema::absolutize_schema_id(&mut schema_value, &schema_path)?;
    schema::absolutize_schema_id(&mut version_schema_value, &version_schema_path)?;

    let schema = JSONSchema::options()
        .with_draft(Draft::Draft202012)
        .compile(&schema_value)
        .map_err(|e| FatalError::SchemaCompile(e.to_string()))?;
    let version_schema = JSONSchema::options()
        .with_draft(Draft::Draft202012)
        .compile(&version_schema_value)
        .map_err(|e| FatalError::SchemaCompile(e.to_string()))?;

    let parity_exclusions_schema_version =
        rules.parity_exclusions.as_ref().map(|ex| ex.schema_version);
    let parity_exclusions_raw = rules.parity_exclusions.as_ref().map(|ex| ex.units.clone());
    let parity_exclusions = rules
        .parity_exclusions
        .as_ref()
        .filter(|ex| ex.schema_version == 1)
        .map(|ex| build_parity_exclusions_index(&ex.units));

    let mut ctx = ValidateCtx {
        root,
        required_target: rules.union.required_target,
        expected_targets: rules.union.expected_targets,
        platform_mapping: rules.union.platform_mapping,
        stable_semver_re,
        schema,
        version_schema,
        wrapper_rules: rules.wrapper_coverage,
        parity_exclusions_schema_version,
        parity_exclusions_raw,
        parity_exclusions,
    };

    if matches!(args.mode, Mode::Fix) {
        fix_mode::apply_fix_mode(&ctx)?;
    }

    let mut violations = Vec::<Violation>::new();

    validate_parity_exclusions_config(&mut ctx, &mut violations);

    // 1) Pointer files.
    let pointer_values = pointers::validate_pointers(&mut ctx, &mut violations);

    // 2) Version set to validate.
    let versions_to_validate =
        versions::compute_versions_to_validate(&mut ctx, &mut violations, &pointer_values);

    // 3) Per-version required files (+ schemas).
    let mut version_metadata = BTreeMap::<String, Value>::new();
    for version in &versions_to_validate {
        versions::validate_version_bundle(
            &mut ctx,
            &mut violations,
            version,
            &mut version_metadata,
        );
    }

    // 4) current.json invariants.
    current::validate_current_json(
        &mut ctx,
        &mut violations,
        pointer_values.latest_validated.as_deref(),
    );

    // 5) wrapper_coverage.json and semantic invariants.
    wrapper_coverage::validate_wrapper_coverage(&mut ctx, &mut violations);

    // 6) Pointer → version metadata consistency (requires parsed metadata).
    pointer_consistency::validate_pointer_consistency(
        &ctx,
        &mut violations,
        &pointer_values,
        &version_metadata,
    );

    violations.sort_by(|a, b| {
        a.sort_key()
            .cmp(&b.sort_key())
            .then_with(|| a.target_triple.cmp(&b.target_triple))
            .then_with(|| a.json_pointer.cmp(&b.json_pointer))
            .then_with(|| a.code.cmp(b.code))
            .then_with(|| a.message.cmp(&b.message))
    });

    Ok(violations)
}

fn parse_stable_version(s: &str, stable_semver_re: &Regex) -> Option<Version> {
    models::parse_stable_version(s, stable_semver_re)
}

fn build_parity_exclusions_index(units: &[ParityExclusionUnit]) -> ParityExclusionsIndex {
    let mut commands = BTreeMap::new();
    let mut flags = BTreeMap::new();
    let mut args = BTreeMap::new();

    for unit in units {
        match unit.unit.as_str() {
            "command" => {
                commands.insert(unit.path.clone(), unit.clone());
            }
            "flag" => {
                if let Some(key) = unit.key.as_ref() {
                    flags.insert((unit.path.clone(), key.clone()), unit.clone());
                }
            }
            "arg" => {
                if let Some(name) = unit.name.as_ref() {
                    args.insert((unit.path.clone(), name.clone()), unit.clone());
                }
            }
            _ => {}
        }
    }

    ParityExclusionsIndex {
        commands,
        flags,
        args,
    }
}

fn validate_parity_exclusions_config(ctx: &mut ValidateCtx, violations: &mut Vec<Violation>) {
    let Some(schema_version) = ctx.parity_exclusions_schema_version else {
        return;
    };
    if schema_version != 1 {
        violations.push(Violation {
            code: "PARITY_EXCLUSIONS_SCHEMA_VERSION",
            path: rel_path(&ctx.root, &ctx.root.join("RULES.json")),
            json_pointer: Some("/parity_exclusions/schema_version".to_string()),
            message: format!("parity_exclusions.schema_version must be 1 (got {schema_version})"),
            unit: Some("rules"),
            command_path: None,
            key_or_name: None,
            field: Some("parity_exclusions"),
            target_triple: None,
            details: None,
        });
        return;
    }

    let Some(units) = ctx.parity_exclusions_raw.as_ref() else {
        violations.push(Violation {
            code: "PARITY_EXCLUSIONS_MISSING_UNITS",
            path: rel_path(&ctx.root, &ctx.root.join("RULES.json")),
            json_pointer: Some("/parity_exclusions/units".to_string()),
            message: "parity_exclusions.units must exist".to_string(),
            unit: Some("rules"),
            command_path: None,
            key_or_name: None,
            field: Some("parity_exclusions"),
            target_triple: None,
            details: None,
        });
        return;
    };

    let mut keys = Vec::new();
    let mut seen = BTreeSet::new();

    for (idx, unit) in units.iter().enumerate() {
        if unit.note.trim().is_empty() {
            violations.push(Violation {
                code: "PARITY_EXCLUSIONS_NOTE_MISSING",
                path: rel_path(&ctx.root, &ctx.root.join("RULES.json")),
                json_pointer: Some(format!("/parity_exclusions/units/{idx}/note")),
                message: "parity_exclusions entry requires non-empty note".to_string(),
                unit: Some("rules"),
                command_path: Some(format_command_path(&unit.path)),
                key_or_name: unit
                    .key
                    .clone()
                    .or_else(|| unit.name.clone())
                    .or_else(|| Some(unit.unit.clone())),
                field: Some("parity_exclusions"),
                target_triple: None,
                details: None,
            });
        }

        let (kind, key_or_name) = match unit.unit.as_str() {
            "command" => {
                if unit.key.is_some() || unit.name.is_some() {
                    violations.push(Violation {
                        code: "PARITY_EXCLUSIONS_INVALID_ENTRY",
                        path: rel_path(&ctx.root, &ctx.root.join("RULES.json")),
                        json_pointer: Some(format!("/parity_exclusions/units/{idx}")),
                        message: "parity_exclusions command entry must not include key or name"
                            .to_string(),
                        unit: Some("rules"),
                        command_path: Some(format_command_path(&unit.path)),
                        key_or_name: None,
                        field: Some("parity_exclusions"),
                        target_triple: None,
                        details: None,
                    });
                }
                ("command".to_string(), "".to_string())
            }
            "flag" => {
                let Some(key) = unit.key.as_ref().filter(|s| !s.trim().is_empty()) else {
                    violations.push(Violation {
                        code: "PARITY_EXCLUSIONS_INVALID_ENTRY",
                        path: rel_path(&ctx.root, &ctx.root.join("RULES.json")),
                        json_pointer: Some(format!("/parity_exclusions/units/{idx}/key")),
                        message: "parity_exclusions flag entry requires key".to_string(),
                        unit: Some("rules"),
                        command_path: Some(format_command_path(&unit.path)),
                        key_or_name: None,
                        field: Some("parity_exclusions"),
                        target_triple: None,
                        details: None,
                    });
                    continue;
                };
                if unit.name.is_some() {
                    violations.push(Violation {
                        code: "PARITY_EXCLUSIONS_INVALID_ENTRY",
                        path: rel_path(&ctx.root, &ctx.root.join("RULES.json")),
                        json_pointer: Some(format!("/parity_exclusions/units/{idx}/name")),
                        message: "parity_exclusions flag entry must not include name".to_string(),
                        unit: Some("rules"),
                        command_path: Some(format_command_path(&unit.path)),
                        key_or_name: Some(key.clone()),
                        field: Some("parity_exclusions"),
                        target_triple: None,
                        details: None,
                    });
                }
                ("flag".to_string(), key.clone())
            }
            "arg" => {
                let Some(name) = unit.name.as_ref().filter(|s| !s.trim().is_empty()) else {
                    violations.push(Violation {
                        code: "PARITY_EXCLUSIONS_INVALID_ENTRY",
                        path: rel_path(&ctx.root, &ctx.root.join("RULES.json")),
                        json_pointer: Some(format!("/parity_exclusions/units/{idx}/name")),
                        message: "parity_exclusions arg entry requires name".to_string(),
                        unit: Some("rules"),
                        command_path: Some(format_command_path(&unit.path)),
                        key_or_name: None,
                        field: Some("parity_exclusions"),
                        target_triple: None,
                        details: None,
                    });
                    continue;
                };
                if unit.key.is_some() {
                    violations.push(Violation {
                        code: "PARITY_EXCLUSIONS_INVALID_ENTRY",
                        path: rel_path(&ctx.root, &ctx.root.join("RULES.json")),
                        json_pointer: Some(format!("/parity_exclusions/units/{idx}/key")),
                        message: "parity_exclusions arg entry must not include key".to_string(),
                        unit: Some("rules"),
                        command_path: Some(format_command_path(&unit.path)),
                        key_or_name: Some(name.clone()),
                        field: Some("parity_exclusions"),
                        target_triple: None,
                        details: None,
                    });
                }
                ("arg".to_string(), name.clone())
            }
            other => {
                violations.push(Violation {
                    code: "PARITY_EXCLUSIONS_INVALID_ENTRY",
                    path: rel_path(&ctx.root, &ctx.root.join("RULES.json")),
                    json_pointer: Some(format!("/parity_exclusions/units/{idx}/unit")),
                    message: format!(
                        "parity_exclusions entry unit must be one of command|flag|arg (got {other})"
                    ),
                    unit: Some("rules"),
                    command_path: Some(format_command_path(&unit.path)),
                    key_or_name: None,
                    field: Some("parity_exclusions"),
                    target_triple: None,
                    details: None,
                });
                continue;
            }
        };

        let identity = (kind.clone(), unit.path.clone(), key_or_name.clone());
        keys.push(identity.clone());
        if !seen.insert(identity.clone()) {
            violations.push(Violation {
                code: "PARITY_EXCLUSIONS_DUPLICATE",
                path: rel_path(&ctx.root, &ctx.root.join("RULES.json")),
                json_pointer: Some("/parity_exclusions/units".to_string()),
                message: format!(
                    "duplicate parity_exclusions identity (unit={kind} command_path={} key_or_name={})",
                    format_command_path(&unit.path),
                    key_or_name
                ),
                unit: Some("rules"),
                command_path: Some(format_command_path(&unit.path)),
                key_or_name: Some(key_or_name),
                field: Some("parity_exclusions"),
                target_triple: None,
                details: None,
            });
        }
    }

    let mut sorted = keys.clone();
    sorted.sort();
    if keys != sorted {
        violations.push(Violation {
            code: "PARITY_EXCLUSIONS_NOT_SORTED",
            path: rel_path(&ctx.root, &ctx.root.join("RULES.json")),
            json_pointer: Some("/parity_exclusions/units".to_string()),
            message: "parity_exclusions.units must be stable-sorted by (unit,path,key_or_name)"
                .to_string(),
            unit: Some("rules"),
            command_path: None,
            key_or_name: None,
            field: Some("parity_exclusions"),
            target_triple: None,
            details: None,
        });
    }
}

fn is_union_snapshot(v: &Value) -> bool {
    v.get("snapshot_schema_version")
        .and_then(Value::as_i64)
        .is_some_and(|x| x == 2)
        && v.get("mode")
            .and_then(Value::as_str)
            .is_some_and(|x| x == "union")
}

fn is_per_target_snapshot(v: &Value) -> bool {
    v.get("snapshot_schema_version")
        .and_then(Value::as_i64)
        .is_some_and(|x| x == 1)
}

fn rel_path(root: &Path, path: &Path) -> String {
    let rel = path.strip_prefix(root).unwrap_or(path);
    rel.to_string_lossy().replace('\\', "/")
}

fn format_command_path(path: &[String]) -> String {
    if path.is_empty() {
        "[]".to_string()
    } else {
        path.join("/")
    }
}
