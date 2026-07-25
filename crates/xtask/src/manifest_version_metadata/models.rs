use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub(super) struct UnionSnapshotV2 {
    pub(super) snapshot_schema_version: u32,
    pub(super) mode: String,
    pub(super) complete: bool,
    pub(super) inputs: Vec<UnionInputV2>,
    pub(super) commands: Vec<UnionCommandV2>,
}

#[derive(Debug, Deserialize, Clone)]
pub(super) struct UnionInputV2 {
    pub(super) target_triple: String,
}

#[derive(Debug, Deserialize, Clone)]
pub(super) struct UnionCommandV2 {
    pub(super) path: Vec<String>,
    pub(super) available_on: Vec<String>,
    #[serde(default)]
    pub(super) flags: Vec<UnionFlagV2>,
    #[serde(default)]
    pub(super) args: Vec<UnionArgV2>,
}

#[derive(Debug, Deserialize, Clone)]
pub(super) struct UnionFlagV2 {
    pub(super) key: String,
    pub(super) available_on: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub(super) struct UnionArgV2 {
    pub(super) name: String,
    pub(super) available_on: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct WrapperCoverageV1 {
    pub(super) schema_version: u32,
    pub(super) coverage: Vec<WrapperCommandCoverageV1>,
}

#[derive(Debug, Deserialize)]
pub(super) struct WrapperCommandCoverageV1 {
    pub(super) path: Vec<String>,
    pub(super) level: String,
    pub(super) note: Option<String>,
    pub(super) scope: Option<WrapperScope>,
    #[serde(default)]
    pub(super) flags: Vec<WrapperFlagCoverageV1>,
    #[serde(default)]
    pub(super) args: Vec<WrapperArgCoverageV1>,
}

#[derive(Debug, Deserialize)]
pub(super) struct WrapperFlagCoverageV1 {
    pub(super) key: String,
    pub(super) level: String,
    pub(super) note: Option<String>,
    pub(super) scope: Option<WrapperScope>,
}

#[derive(Debug, Deserialize)]
pub(super) struct WrapperArgCoverageV1 {
    pub(super) name: String,
    pub(super) level: String,
    pub(super) note: Option<String>,
    pub(super) scope: Option<WrapperScope>,
}

#[derive(Debug, Deserialize, Clone)]
pub(super) struct WrapperScope {
    pub(super) platforms: Option<Vec<String>>,
    pub(super) target_triples: Option<Vec<String>>,
}
