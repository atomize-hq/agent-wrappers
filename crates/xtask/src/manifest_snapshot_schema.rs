use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SnapshotV1 {
    pub(crate) snapshot_schema_version: u32,
    pub(crate) tool: String,
    pub(crate) collected_at: String,
    pub(crate) binary: BinarySnapshot,
    pub(crate) commands: Vec<CommandSnapshot>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) features: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) known_omissions: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct BinarySnapshot {
    pub(crate) sha256: String,
    pub(crate) size_bytes: u64,
    pub(crate) platform: BinaryPlatform,
    pub(crate) target_triple: String,
    pub(crate) version_output: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) semantic_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) channel: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) commit: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct BinaryPlatform {
    pub(crate) os: String,
    pub(crate) arch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CommandSnapshot {
    pub(crate) path: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) about: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) usage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) stability: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) platforms: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) args: Option<Vec<ArgSnapshot>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) flags: Option<Vec<FlagSnapshot>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ArgSnapshot {
    pub(crate) name: String,
    pub(crate) required: bool,
    pub(crate) variadic: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct FlagSnapshot {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) long: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) short: Option<String>,
    pub(crate) takes_value: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) value_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) repeatable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) stability: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) platforms: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct SupplementV1 {
    pub(crate) version: u32,
    pub(crate) commands: Vec<SupplementCommand>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct SupplementCommand {
    pub(crate) path: Vec<String>,
    #[serde(default)]
    pub(crate) platforms: Option<Vec<String>>,
    pub(crate) note: String,
}
