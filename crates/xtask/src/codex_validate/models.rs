use std::collections::{BTreeMap, BTreeSet};

use regex::Regex;
use semver::Version;
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Debug, Clone)]
pub(super) struct Violation {
    pub(super) code: &'static str,
    pub(super) path: String,
    pub(super) json_pointer: Option<String>,
    pub(super) message: String,

    pub(super) unit: Option<&'static str>,
    pub(super) command_path: Option<String>,
    pub(super) key_or_name: Option<String>,
    pub(super) field: Option<&'static str>,
    pub(super) target_triple: Option<String>,

    pub(super) details: Option<Value>,
}

impl Violation {
    pub(super) fn sort_key(&self) -> (&str, &str, &str, &str, &str) {
        (
            self.path.as_str(),
            self.unit.unwrap_or(""),
            self.command_path.as_deref().unwrap_or(""),
            self.key_or_name.as_deref().unwrap_or(""),
            self.field.unwrap_or(""),
        )
    }

    pub(super) fn to_human_line(&self) -> String {
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

    pub(super) fn to_json(&self) -> Value {
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
pub(super) struct Rules {
    pub(super) union: RulesUnion,
    pub(super) versioning: RulesVersioning,
    pub(super) wrapper_coverage: RulesWrapperCoverage,
    #[serde(default)]
    pub(super) parity_exclusions: Option<RulesParityExclusions>,
}

#[derive(Debug, Deserialize)]
pub(super) struct RulesUnion {
    pub(super) required_target: String,
    pub(super) expected_targets: Vec<String>,
    pub(super) platform_mapping: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct RulesVersioning {
    pub(super) pointers: RulesPointers,
}

#[derive(Debug, Deserialize)]
pub(super) struct RulesPointers {
    pub(super) stable_semver_pattern: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct RulesWrapperCoverage {
    pub(super) scope_semantics: RulesWrapperScopeSemantics,
    pub(super) validation: RulesWrapperValidation,
}

#[derive(Debug, Deserialize)]
pub(super) struct RulesParityExclusions {
    pub(super) schema_version: u32,
    pub(super) units: Vec<ParityExclusionUnit>,
}

#[derive(Debug, Deserialize, Clone)]
pub(super) struct ParityExclusionUnit {
    pub(super) unit: String,
    pub(super) path: Vec<String>,
    #[serde(default)]
    pub(super) key: Option<String>,
    #[serde(default)]
    pub(super) name: Option<String>,
    #[serde(default)]
    pub(super) category: Option<String>,
    pub(super) note: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct RulesWrapperScopeSemantics {
    pub(super) defaults: RulesWrapperScopeDefaults,
    pub(super) platforms_expand_to_expected_targets: bool,
    pub(super) platforms_expand_using: String,
    pub(super) scope_set_resolution: RulesWrapperScopeSetResolution,
}

#[derive(Debug, Deserialize)]
pub(super) struct RulesWrapperScopeDefaults {
    pub(super) no_scope_means: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct RulesWrapperScopeSetResolution {
    pub(super) mode: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct RulesWrapperValidation {
    pub(super) disallow_overlapping_scopes: bool,
    pub(super) overlap_units: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct WrapperCoverageFile {
    pub(super) schema_version: u32,
    pub(super) coverage: Vec<WrapperCommandCoverage>,
}

#[derive(Debug, Deserialize)]
pub(super) struct WrapperCommandCoverage {
    pub(super) path: Vec<String>,
    pub(super) level: String,
    pub(super) note: Option<String>,
    pub(super) scope: Option<WrapperScope>,
    pub(super) flags: Option<Vec<WrapperFlagCoverage>>,
    pub(super) args: Option<Vec<WrapperArgCoverage>>,
}

#[derive(Debug, Deserialize, Clone)]
pub(super) struct WrapperScope {
    pub(super) platforms: Option<Vec<String>>,
    pub(super) target_triples: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub(super) struct WrapperFlagCoverage {
    pub(super) key: String,
    pub(super) level: String,
    pub(super) note: Option<String>,
    pub(super) scope: Option<WrapperScope>,
}

#[derive(Debug, Deserialize)]
pub(super) struct WrapperArgCoverage {
    pub(super) name: String,
    pub(super) level: String,
    pub(super) note: Option<String>,
    pub(super) scope: Option<WrapperScope>,
}

#[derive(Debug, Clone)]
pub(super) enum PointerValue {
    None,
    Version(Version),
}

#[derive(Debug)]
pub(super) enum PointerRead {
    Missing,
    InvalidFormat { reason: &'static str },
    InvalidValue { raw: String },
    Value(PointerValue),
}

#[derive(Debug, Default)]
pub(super) struct PointerValues {
    pub(super) min_supported: Option<String>,
    pub(super) latest_validated: Option<String>,
    pub(super) by_target_latest_supported: BTreeMap<String, Option<String>>,
    pub(super) by_target_latest_validated: BTreeMap<String, Option<String>>,
}

#[derive(Debug, Clone)]
pub(super) struct ScopedEntry {
    pub(super) index: String,
    pub(super) scope_kind: &'static str,
    pub(super) targets: BTreeSet<String>,
}

#[derive(Debug)]
pub(super) struct ParityExclusionsIndex {
    pub(super) commands: BTreeMap<Vec<String>, ParityExclusionUnit>,
    pub(super) flags: BTreeMap<(Vec<String>, String), ParityExclusionUnit>,
    pub(super) args: BTreeMap<(Vec<String>, String), ParityExclusionUnit>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct IuSortKey {
    pub(super) kind_rank: u8,
    pub(super) path: Vec<String>,
    pub(super) key_or_name: String,
}

pub(super) fn parse_stable_version(s: &str, stable_semver_re: &Regex) -> Option<Version> {
    if !stable_semver_re.is_match(s) {
        return None;
    }
    Version::parse(s).ok()
}
