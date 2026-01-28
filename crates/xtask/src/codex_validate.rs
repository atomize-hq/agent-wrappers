use std::{
    collections::{BTreeMap, BTreeSet},
    fs, io,
    path::{Path, PathBuf},
};

use clap::{Parser, ValueEnum};
use jsonschema::{Draft, JSONSchema};
use regex::Regex;
use semver::Version;
use serde::Deserialize;
use serde_json::{json, Value};
use thiserror::Error;

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

#[derive(Debug, Clone)]
struct Violation {
    code: &'static str,
    path: String,
    json_pointer: Option<String>,
    message: String,

    unit: Option<&'static str>,
    command_path: Option<String>,
    key_or_name: Option<String>,
    field: Option<&'static str>,
    target_triple: Option<String>,

    details: Option<Value>,
}

impl Violation {
    fn sort_key(&self) -> (&str, &str, &str, &str, &str) {
        (
            self.path.as_str(),
            self.unit.unwrap_or(""),
            self.command_path.as_deref().unwrap_or(""),
            self.key_or_name.as_deref().unwrap_or(""),
            self.field.unwrap_or(""),
        )
    }

    fn to_human_line(&self) -> String {
        // Keep this stable and single-line for CI logs.
        let mut parts = vec![self.code, "error", self.path.as_str()];
        if let Some(ptr) = self.json_pointer.as_deref() {
            if !ptr.is_empty() {
                parts.push(ptr);
            }
        }
        parts.push(self.message.as_str());
        parts.join(" ")
    }

    fn to_json(&self) -> Value {
        let mut out = json!({
            "code": self.code,
            "severity": "error",
            "path": self.path,
            "message": self.message,
        });
        if let Some(ptr) = self.json_pointer.clone() {
            out["json_pointer"] = Value::String(ptr);
        }
        if let Some(details) = self.details.clone() {
            out["details"] = details;
        }
        out
    }
}

#[derive(Debug, Deserialize)]
struct Rules {
    union: RulesUnion,
    versioning: RulesVersioning,
    wrapper_coverage: RulesWrapperCoverage,
}

#[derive(Debug, Deserialize)]
struct RulesUnion {
    required_target: String,
    expected_targets: Vec<String>,
    platform_mapping: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct RulesVersioning {
    pointers: RulesPointers,
}

#[derive(Debug, Deserialize)]
struct RulesPointers {
    stable_semver_pattern: String,
}

#[derive(Debug, Deserialize)]
struct RulesWrapperCoverage {
    scope_semantics: RulesWrapperScopeSemantics,
    validation: RulesWrapperValidation,
}

#[derive(Debug, Deserialize)]
struct RulesWrapperScopeSemantics {
    defaults: RulesWrapperScopeDefaults,
    platforms_expand_to_expected_targets: bool,
    platforms_expand_using: String,
    scope_set_resolution: RulesWrapperScopeSetResolution,
}

#[derive(Debug, Deserialize)]
struct RulesWrapperScopeDefaults {
    no_scope_means: String,
}

#[derive(Debug, Deserialize)]
struct RulesWrapperScopeSetResolution {
    mode: String,
}

#[derive(Debug, Deserialize)]
struct RulesWrapperValidation {
    disallow_overlapping_scopes: bool,
    overlap_units: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct WrapperCoverageFile {
    schema_version: u32,
    coverage: Vec<WrapperCommandCoverage>,
}

#[derive(Debug, Deserialize)]
struct WrapperCommandCoverage {
    path: Vec<String>,
    level: String,
    note: Option<String>,
    scope: Option<WrapperScope>,
    flags: Option<Vec<WrapperFlagCoverage>>,
    args: Option<Vec<WrapperArgCoverage>>,
}

#[derive(Debug, Deserialize, Clone)]
struct WrapperScope {
    platforms: Option<Vec<String>>,
    target_triples: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct WrapperFlagCoverage {
    key: String,
    level: String,
    note: Option<String>,
    scope: Option<WrapperScope>,
}

#[derive(Debug, Deserialize)]
struct WrapperArgCoverage {
    name: String,
    level: String,
    note: Option<String>,
    scope: Option<WrapperScope>,
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

    let schema_value: Value = serde_json::from_slice(&fs::read(&schema_path)?)?;
    let version_schema_value: Value = serde_json::from_slice(&fs::read(&version_schema_path)?)?;

    let schema = JSONSchema::options()
        .with_draft(Draft::Draft202012)
        .compile(&schema_value)
        .map_err(|e| FatalError::SchemaCompile(e.to_string()))?;
    let version_schema = JSONSchema::options()
        .with_draft(Draft::Draft202012)
        .compile(&version_schema_value)
        .map_err(|e| FatalError::SchemaCompile(e.to_string()))?;

    let mut ctx = ValidateCtx {
        root,
        required_target: rules.union.required_target,
        expected_targets: rules.union.expected_targets,
        platform_mapping: rules.union.platform_mapping,
        stable_semver_re,
        schema,
        version_schema,
        wrapper_rules: rules.wrapper_coverage,
    };

    if matches!(args.mode, Mode::Fix) {
        apply_fix_mode(&ctx)?;
    }

    let mut violations = Vec::<Violation>::new();

    // 1) Pointer files.
    let pointer_values = validate_pointers(&mut ctx, &mut violations);

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
    validate_wrapper_coverage(&mut ctx, &mut violations);

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
            normalize_single_line_file(&path)?;
        }
    }
    normalize_single_line_file(&ctx.root.join("latest_validated.txt"))?;
    normalize_single_line_file(&ctx.root.join("min_supported.txt"))?;

    // 3) Normalize current.json to match snapshots/<latest_validated>/union.json (if possible).
    let latest_validated = match read_pointer_file(
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

fn normalize_single_line_file(path: &Path) -> Result<(), FatalError> {
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

#[derive(Debug, Clone)]
enum PointerValue {
    None,
    Version(Version),
}

#[derive(Debug)]
enum PointerRead {
    Missing,
    InvalidFormat { reason: &'static str },
    InvalidValue { raw: String },
    Value(PointerValue),
}

#[derive(Debug, Default)]
struct PointerValues {
    min_supported: Option<String>,
    latest_validated: Option<String>,
    by_target_latest_supported: BTreeMap<String, Option<String>>,
    by_target_latest_validated: BTreeMap<String, Option<String>>,
}

fn validate_pointers(ctx: &mut ValidateCtx, violations: &mut Vec<Violation>) -> PointerValues {
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
            &format!("invalid pointer file format ({reason}); expected single line + trailing newline"),
        )),
        Ok(PointerRead::InvalidValue { raw }) => violations.push(pointer_violation(
            ctx,
            "POINTER_INVALID_VALUE",
            &min_supported_path,
            &format!("invalid pointer value (got {raw}); expected strict stable semver MAJOR.MINOR.PATCH"),
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
            &format!("invalid pointer file format ({reason}); expected single line + trailing newline"),
        )),
        Ok(PointerRead::InvalidValue { raw }) => violations.push(pointer_violation(
            ctx,
            "POINTER_INVALID_VALUE",
            &latest_validated_path,
            &format!("invalid pointer value (got {raw}); expected strict stable semver MAJOR.MINOR.PATCH"),
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
    match read_json_file(
        &ctx.root,
        &version_path,
        violations,
        "VERSION_METADATA_INVALID_JSON",
    ) {
        Some(value) => {
            schema_validate(
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
    let union_value = match read_json_file(&ctx.root, &union_path, violations, "UNION_INVALID_JSON")
    {
        Some(value) => {
            schema_validate(
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
        match read_json_file(
            &ctx.root,
            &per_target_path,
            violations,
            "SNAPSHOT_INVALID_JSON",
        ) {
            Some(value) => {
                schema_validate(
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
    if !require_reports {
        return;
    }

    let reports_dir = ctx.root.join("reports").join(version);
    let any_report = reports_dir.join("coverage.any.json");
    require_report(ctx, violations, version, "any", None, &any_report);

    for target in &input_targets {
        let per_target = reports_dir.join(format!("coverage.{target}.json"));
        require_report(
            ctx,
            violations,
            version,
            "per_target",
            Some(target.as_str()),
            &per_target,
        );
    }

    let complete = union_value
        .as_ref()
        .and_then(|u| u.get("complete"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if complete {
        let all_report = reports_dir.join("coverage.all.json");
        require_report(ctx, violations, version, "all", None, &all_report);
    }
}

fn require_report(
    ctx: &ValidateCtx,
    violations: &mut Vec<Violation>,
    version: &str,
    kind: &'static str,
    target: Option<&str>,
    path: &Path,
) {
    match read_json_file(&ctx.root, path, violations, "REPORT_INVALID_JSON") {
        Some(value) => {
            schema_validate(
                ctx,
                violations,
                &ctx.schema,
                &value,
                path,
                "REPORT_SCHEMA_INVALID",
            );
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

fn validate_current_json(
    ctx: &mut ValidateCtx,
    violations: &mut Vec<Violation>,
    latest_validated: Option<&str>,
) {
    let current_path = ctx.root.join("current.json");
    let current_value = match read_json_file(
        &ctx.root,
        &current_path,
        violations,
        "CURRENT_INVALID_JSON",
    ) {
        Some(v) => {
            schema_validate(
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

fn validate_wrapper_coverage(ctx: &mut ValidateCtx, violations: &mut Vec<Violation>) {
    let path = ctx.root.join("wrapper_coverage.json");
    let value = match read_json_file(
        &ctx.root,
        &path,
        violations,
        "WRAPPER_COVERAGE_INVALID_JSON",
    ) {
        Some(v) => {
            schema_validate(
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

#[derive(Debug, Clone)]
struct ScopedEntry {
    index: String,
    scope_kind: &'static str,
    targets: BTreeSet<String>,
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
        let mut by_cmd = BTreeMap::<String, Vec<ScopedEntry>>::new();
        for (cmd_idx, cmd) in wc.coverage.iter().enumerate() {
            let cmd_targets = scope_to_targets(ctx, &expected, cmd.scope.as_ref());
            by_cmd
                .entry(format_command_path(&cmd.path))
                .or_default()
                .push(ScopedEntry {
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
        let mut by_flag = BTreeMap::<(String, String), Vec<ScopedEntry>>::new();
        for (cmd_idx, cmd) in wc.coverage.iter().enumerate() {
            let cmd_targets = scope_to_targets(ctx, &expected, cmd.scope.as_ref());
            for (flag_idx, flag) in cmd.flags.as_deref().unwrap_or(&[]).iter().enumerate() {
                let flag_targets = scope_to_targets(ctx, &expected, flag.scope.as_ref());
                let effective = intersect(&cmd_targets, &flag_targets);
                by_flag
                    .entry((format_command_path(&cmd.path), flag.key.clone()))
                    .or_default()
                    .push(ScopedEntry {
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
        let mut by_arg = BTreeMap::<(String, String), Vec<ScopedEntry>>::new();
        for (cmd_idx, cmd) in wc.coverage.iter().enumerate() {
            let cmd_targets = scope_to_targets(ctx, &expected, cmd.scope.as_ref());
            for (arg_idx, arg) in cmd.args.as_deref().unwrap_or(&[]).iter().enumerate() {
                let arg_targets = scope_to_targets(ctx, &expected, arg.scope.as_ref());
                let effective = intersect(&cmd_targets, &arg_targets);
                by_arg
                    .entry((format_command_path(&cmd.path), arg.name.clone()))
                    .or_default()
                    .push(ScopedEntry {
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
    entries: Vec<ScopedEntry>,
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

fn schema_validate(
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

fn read_json_file(
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

fn read_pointer_file(
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

fn parse_stable_version(s: &str, stable_semver_re: &Regex) -> Option<Version> {
    if !stable_semver_re.is_match(s) {
        return None;
    }
    Version::parse(s).ok()
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
