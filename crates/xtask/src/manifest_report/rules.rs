use std::{collections::BTreeMap, fs, path::Path};

use serde::Deserialize;

use super::ReportError;

#[derive(Debug, Deserialize)]
pub(super) struct RulesFile {
    #[serde(rename = "rules_schema_version")]
    pub(super) rules_schema_version: u32,
    pub(super) union: RulesUnion,
    pub(super) report: RulesReport,
    pub(super) sorting: RulesSorting,
    #[serde(default)]
    pub(super) parity_exclusions: Option<RulesParityExclusions>,
}

#[derive(Debug, Deserialize)]
pub(super) struct RulesUnion {
    pub(super) expected_targets: Vec<String>,
    pub(super) platform_mapping: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct RulesSorting {
    pub(super) report: RulesReportSorting,
}

#[derive(Debug, Deserialize)]
pub(super) struct RulesReportSorting {
    pub(super) missing_commands: String,
    pub(super) missing_flags: String,
    pub(super) missing_args: String,
    pub(super) excluded_commands: String,
    pub(super) excluded_flags: String,
    pub(super) excluded_args: String,
    pub(super) passthrough_candidates: String,
    pub(super) unsupported: String,
    pub(super) intentionally_unsupported: String,
    pub(super) wrapper_only_commands: String,
    pub(super) wrapper_only_flags: String,
    pub(super) wrapper_only_args: String,
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
    #[allow(dead_code)]
    pub(super) category: Option<String>,
    pub(super) note: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct RulesReport {
    pub(super) file_naming: RulesReportFileNaming,
    pub(super) filter_semantics: RulesFilterSemantics,
}

#[derive(Debug, Deserialize)]
pub(super) struct RulesReportFileNaming {
    pub(super) any: String,
    pub(super) all: String,
    pub(super) per_target: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct RulesFilterSemantics {
    pub(super) when_union_incomplete: RulesWhenUnionIncomplete,
}

#[derive(Debug, Deserialize)]
pub(super) struct RulesWhenUnionIncomplete {
    pub(super) all: String,
}

pub(super) fn load_rules(rules_path: &Path) -> Result<RulesFile, ReportError> {
    Ok(serde_json::from_slice(&fs::read(rules_path)?)?)
}

pub(super) fn assert_supported_rules(rules: &RulesFile) -> Result<(), ReportError> {
    let mut unsupported = Vec::new();

    if rules.report.file_naming.any != "coverage.any.json" {
        unsupported.push(format!(
            "report.file_naming.any={}",
            rules.report.file_naming.any
        ));
    }
    if rules.report.file_naming.all != "coverage.all.json" {
        unsupported.push(format!(
            "report.file_naming.all={}",
            rules.report.file_naming.all
        ));
    }
    if rules.report.file_naming.per_target != "coverage.<target_triple>.json" {
        unsupported.push(format!(
            "report.file_naming.per_target={}",
            rules.report.file_naming.per_target
        ));
    }

    if rules.sorting.report.missing_commands != "by_path" {
        unsupported.push(format!(
            "sorting.report.missing_commands={}",
            rules.sorting.report.missing_commands
        ));
    }
    if rules.sorting.report.missing_flags != "by_path_then_key" {
        unsupported.push(format!(
            "sorting.report.missing_flags={}",
            rules.sorting.report.missing_flags
        ));
    }
    if rules.sorting.report.missing_args != "by_path_then_name" {
        unsupported.push(format!(
            "sorting.report.missing_args={}",
            rules.sorting.report.missing_args
        ));
    }
    if rules.sorting.report.excluded_commands != "by_path" {
        unsupported.push(format!(
            "sorting.report.excluded_commands={}",
            rules.sorting.report.excluded_commands
        ));
    }
    if rules.sorting.report.excluded_flags != "by_path_then_key" {
        unsupported.push(format!(
            "sorting.report.excluded_flags={}",
            rules.sorting.report.excluded_flags
        ));
    }
    if rules.sorting.report.excluded_args != "by_path_then_name" {
        unsupported.push(format!(
            "sorting.report.excluded_args={}",
            rules.sorting.report.excluded_args
        ));
    }
    if rules.sorting.report.passthrough_candidates != "by_path" {
        unsupported.push(format!(
            "sorting.report.passthrough_candidates={}",
            rules.sorting.report.passthrough_candidates
        ));
    }
    if rules.sorting.report.unsupported != "by_path" {
        unsupported.push(format!(
            "sorting.report.unsupported={}",
            rules.sorting.report.unsupported
        ));
    }
    if rules.sorting.report.intentionally_unsupported != "by_kind_then_path_then_key_or_name" {
        unsupported.push(format!(
            "sorting.report.intentionally_unsupported={}",
            rules.sorting.report.intentionally_unsupported
        ));
    }
    if rules.sorting.report.wrapper_only_commands != "by_path" {
        unsupported.push(format!(
            "sorting.report.wrapper_only_commands={}",
            rules.sorting.report.wrapper_only_commands
        ));
    }
    if rules.sorting.report.wrapper_only_flags != "by_path_then_key" {
        unsupported.push(format!(
            "sorting.report.wrapper_only_flags={}",
            rules.sorting.report.wrapper_only_flags
        ));
    }
    if rules.sorting.report.wrapper_only_args != "by_path_then_name" {
        unsupported.push(format!(
            "sorting.report.wrapper_only_args={}",
            rules.sorting.report.wrapper_only_args
        ));
    }

    if rules.report.filter_semantics.when_union_incomplete.all != "error" {
        unsupported.push(format!(
            "report.filter_semantics.when_union_incomplete.all={}",
            rules.report.filter_semantics.when_union_incomplete.all
        ));
    }

    if rules.union.expected_targets.is_empty() {
        unsupported.push("union.expected_targets must not be empty".to_string());
    }

    if unsupported.is_empty() {
        Ok(())
    } else {
        Err(ReportError::Rules(format!(
            "unsupported rules: {}",
            unsupported.join(", ")
        )))
    }
}
