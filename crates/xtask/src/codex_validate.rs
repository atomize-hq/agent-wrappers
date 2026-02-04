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

mod models;
mod pointers;
mod report_invariants;
mod schema;
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
        apply_fix_mode(&ctx)?;
    }

    let mut violations = Vec::<Violation>::new();

    validate_parity_exclusions_config(&mut ctx, &mut violations);

    // 1) Pointer files.
    let pointer_values = pointers::validate_pointers(&mut ctx, &mut violations);

    // 2) Version set to validate.
    let versions_to_validate =
        compute_versions_to_validate(&mut ctx, &mut violations, &pointer_values);

    // 3) Per-version required files (+ schemas).
    let mut version_metadata = BTreeMap::<String, Value>::new();
    for version in &versions_to_validate {
        validate_version_bundle(&mut ctx, &mut violations, version, &mut version_metadata);
    }

    // 4) current.json invariants.
    validate_current_json(
        &mut ctx,
        &mut violations,
        pointer_values.latest_validated.as_deref(),
    );

    // 5) wrapper_coverage.json and semantic invariants.
    wrapper_coverage::validate_wrapper_coverage(&mut ctx, &mut violations);

    // 6) Pointer → version metadata consistency (requires parsed metadata).
    validate_pointer_consistency(&ctx, &mut violations, &pointer_values, &version_metadata);

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

fn apply_fix_mode(ctx: &ValidateCtx) -> Result<(), FatalError> {
    // 1) Create missing pointer files under pointers/ for every expected target.
    for target in &ctx.expected_targets {
        for dir in ["pointers/latest_supported", "pointers/latest_validated"] {
            let path = ctx.root.join(dir).join(format!("{target}.txt"));
            if path.exists() {
                continue;
            }
            fs::create_dir_all(path.parent().unwrap_or(&ctx.root))?;
            fs::write(&path, b"none\n")?;
        }
    }

    // 2) Normalize pointer formatting (single line + trailing newline).
    for target in &ctx.expected_targets {
        for dir in ["pointers/latest_supported", "pointers/latest_validated"] {
            let path = ctx.root.join(dir).join(format!("{target}.txt"));
            pointers::normalize_single_line_file(&path)?;
        }
    }
    pointers::normalize_single_line_file(&ctx.root.join("latest_validated.txt"))?;
    pointers::normalize_single_line_file(&ctx.root.join("min_supported.txt"))?;

    // 3) Normalize current.json to match snapshots/<latest_validated>/union.json (if possible).
    let latest_validated = match pointers::read_pointer_file(
        &ctx.root.join("latest_validated.txt"),
        &ctx.stable_semver_re,
        false,
    ) {
        Ok(PointerRead::Value(PointerValue::Version(ver))) => Some(ver.to_string()),
        _ => None,
    };

    if let Some(version) = latest_validated {
        let union_path = ctx.root.join("snapshots").join(&version).join("union.json");
        if union_path.is_file() {
            let bytes = fs::read(&union_path)?;
            fs::write(ctx.root.join("current.json"), bytes)?;
        }
    }

    Ok(())
}

fn compute_versions_to_validate(
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

fn validate_version_bundle(
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

fn validate_current_json(
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

fn validate_pointer_consistency(
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
