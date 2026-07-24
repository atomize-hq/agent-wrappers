use serde::Serialize;

#[derive(Debug, Serialize)]
pub(in super::super) struct CoverageReportV1 {
    pub(super) schema_version: u32,
    pub(super) generated_at: String,
    pub(super) inputs: ReportInputsV1,
    pub(super) platform_filter: PlatformFilterV1,
    pub(super) deltas: ReportDeltasV1,
}

#[derive(Debug, Serialize)]
pub(super) struct ReportInputsV1 {
    pub(super) upstream: ReportUpstreamInputsV1,
    pub(super) wrapper: ReportWrapperInputsV1,
    pub(super) rules: ReportRulesInputsV1,
}

#[derive(Debug, Serialize)]
pub(super) struct ReportUpstreamInputsV1 {
    pub(super) semantic_version: String,
    pub(super) mode: String,
    pub(super) targets: Vec<String>,
}

#[derive(Debug, Serialize)]
pub(super) struct ReportWrapperInputsV1 {
    pub(super) schema_version: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) wrapper_version: Option<String>,
}

#[derive(Debug, Serialize)]
pub(super) struct ReportRulesInputsV1 {
    pub(super) rules_schema_version: u32,
}

#[derive(Debug, Serialize)]
pub(super) struct PlatformFilterV1 {
    pub(super) mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) target_triple: Option<String>,
}

#[derive(Debug, Serialize)]
pub(super) struct ReportDeltasV1 {
    pub(super) missing_commands: Vec<ReportCommandDeltaV1>,
    pub(super) missing_flags: Vec<ReportFlagDeltaV1>,
    pub(super) missing_args: Vec<ReportArgDeltaV1>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) excluded_commands: Option<Vec<ReportCommandDeltaV1>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) excluded_flags: Option<Vec<ReportFlagDeltaV1>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) excluded_args: Option<Vec<ReportArgDeltaV1>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) passthrough_candidates: Option<Vec<ReportCommandDeltaV1>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) unsupported: Option<Vec<ReportCommandDeltaV1>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) intentionally_unsupported: Option<Vec<ReportIntentionallyUnsupportedDeltaV1>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) wrapper_only_commands: Option<Vec<ReportCommandDeltaV1>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) wrapper_only_flags: Option<Vec<ReportFlagDeltaV1>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) wrapper_only_args: Option<Vec<ReportArgDeltaV1>>,
}

#[derive(Debug, Serialize)]
pub(super) struct ReportCommandDeltaV1 {
    pub(super) path: Vec<String>,
    pub(super) upstream_available_on: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) wrapper_level: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) note: Option<String>,
}

#[derive(Debug, Serialize)]
pub(super) struct ReportFlagDeltaV1 {
    pub(super) path: Vec<String>,
    pub(super) key: String,
    pub(super) upstream_available_on: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) wrapper_level: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) note: Option<String>,
}

#[derive(Debug, Serialize)]
pub(super) struct ReportArgDeltaV1 {
    pub(super) path: Vec<String>,
    pub(super) name: String,
    pub(super) upstream_available_on: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) wrapper_level: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) note: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub(super) enum ReportIntentionallyUnsupportedDeltaV1 {
    Command(ReportCommandDeltaV1),
    Flag(ReportFlagDeltaV1),
    Arg(ReportArgDeltaV1),
}
