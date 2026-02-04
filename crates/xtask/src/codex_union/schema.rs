use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;

use crate::codex_snapshot::BinarySnapshot;

#[derive(Debug, Serialize)]
pub(super) struct SnapshotUnionV2 {
    pub(super) snapshot_schema_version: u32,
    pub(super) tool: String,
    pub(super) mode: String,
    pub(super) collected_at: String,
    pub(super) expected_targets: Vec<String>,
    pub(super) complete: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) missing_targets: Option<Vec<String>>,
    pub(super) inputs: Vec<UnionInputV2>,
    pub(super) commands: Vec<UnionCommandSnapshotV2>,
}

#[derive(Debug, Serialize)]
pub(super) struct UnionInputV2 {
    pub(super) target_triple: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) collected_at: Option<String>,
    pub(super) binary: BinarySnapshot,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) features: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) known_omissions: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub(super) struct UnionCommandSnapshotV2 {
    pub(super) path: Vec<String>,
    pub(super) available_on: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) about: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) usage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) args: Option<Vec<UnionArgSnapshotV2>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) flags: Option<Vec<UnionFlagSnapshotV2>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) conflicts: Option<Vec<UnionConflictEntryV2>>,
}

#[derive(Debug, Serialize)]
pub(super) struct UnionFlagSnapshotV2 {
    pub(super) key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) long: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) short: Option<String>,
    pub(super) takes_value: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) value_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) repeatable: Option<bool>,
    pub(super) available_on: Vec<String>,
}

#[derive(Debug, Serialize)]
pub(super) struct UnionArgSnapshotV2 {
    pub(super) name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) required: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) variadic: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) inferred_from_usage: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) note: Option<String>,
    pub(super) available_on: Vec<String>,
}

#[derive(Debug, Serialize)]
pub(super) struct UnionConflictEntryV2 {
    pub(super) unit: String,
    pub(super) path: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) name: Option<String>,
    pub(super) field: String,
    pub(super) values_by_target: BTreeMap<String, Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) help_context: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) evidence: Option<UnionConflictEvidenceV2>,
}

#[derive(Debug, Serialize)]
pub(super) struct UnionConflictEvidenceV2 {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) help_ref_by_target: Option<BTreeMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) help_sha256_by_target: Option<BTreeMap<String, String>>,
}
