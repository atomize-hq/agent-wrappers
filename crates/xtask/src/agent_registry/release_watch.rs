#![cfg_attr(test, allow(dead_code))]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{validate_non_empty_scalar, AgentRegistryEntry, AgentRegistryError};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseWatchMetadata {
    pub enabled: bool,
    pub version_policy: ReleaseWatchVersionPolicy,
    pub dispatch_kind: ReleaseWatchDispatchKind,
    #[serde(default)]
    pub dispatch_workflow: Option<String>,
    pub upstream: ReleaseWatchUpstream,
}

impl ReleaseWatchMetadata {
    pub(super) fn validate(&self, _entry: &AgentRegistryEntry) -> Result<(), AgentRegistryError> {
        validate_release_watch_metadata(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NormalizedReleaseWatchMetadata {
    pub enabled: bool,
    pub version_policy: &'static str,
    pub dispatch_kind: &'static str,
    pub dispatch_workflow: Option<String>,
    pub upstream: NormalizedReleaseWatchUpstream,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NormalizedReleaseWatchUpstream {
    pub source_kind: &'static str,
    pub owner: Option<String>,
    pub repo: Option<String>,
    pub tag_prefix: Option<String>,
    pub bucket: Option<String>,
    pub prefix: Option<String>,
    pub version_marker: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dist_tag: Option<String>,
}

pub fn validate_release_watch_metadata(
    metadata: &ReleaseWatchMetadata,
) -> Result<(), AgentRegistryError> {
    if !metadata.enabled {
        return Err(AgentRegistryError::Validation(
            "maintenance.release_watch.enabled=false is not allowed; omit maintenance.release_watch entirely when an agent is not enrolled".to_string(),
        ));
    }

    metadata.upstream.validate()?;
    metadata.validate_dispatch_contract()?;
    Ok(())
}

pub fn normalize_release_watch_metadata(
    metadata: &ReleaseWatchMetadata,
) -> Result<NormalizedReleaseWatchMetadata, AgentRegistryError> {
    validate_release_watch_metadata(metadata)?;
    Ok(NormalizedReleaseWatchMetadata {
        enabled: true,
        version_policy: metadata.version_policy.as_str(),
        dispatch_kind: metadata.dispatch_kind.as_str(),
        dispatch_workflow: metadata
            .dispatch_workflow
            .as_deref()
            .map(str::trim)
            .map(str::to_string),
        upstream: NormalizedReleaseWatchUpstream {
            source_kind: metadata.upstream.source_kind.as_str(),
            owner: metadata
                .upstream
                .owner
                .as_deref()
                .map(str::trim)
                .map(str::to_string),
            repo: metadata
                .upstream
                .repo
                .as_deref()
                .map(str::trim)
                .map(str::to_string),
            tag_prefix: metadata
                .upstream
                .tag_prefix
                .as_deref()
                .map(str::trim)
                .map(str::to_string),
            bucket: metadata
                .upstream
                .bucket
                .as_deref()
                .map(str::trim)
                .map(str::to_string),
            prefix: metadata
                .upstream
                .prefix
                .as_deref()
                .map(str::trim)
                .map(str::to_string),
            version_marker: metadata
                .upstream
                .version_marker
                .as_deref()
                .map(str::trim)
                .map(str::to_string),
            package: metadata
                .upstream
                .package
                .as_deref()
                .map(str::trim)
                .map(str::to_string),
            dist_tag: metadata
                .upstream
                .dist_tag
                .as_deref()
                .map(str::trim)
                .map(str::to_string),
        },
    })
}

pub fn normalized_release_watch_sha256(
    metadata: &ReleaseWatchMetadata,
) -> Result<String, AgentRegistryError> {
    let normalized = normalize_release_watch_metadata(metadata)?;
    let bytes = serde_json::to_vec(&normalized).map_err(|err| {
        AgentRegistryError::Validation(format!(
            "serialize normalized maintenance.release_watch payload: {err}"
        ))
    })?;
    Ok(hex::encode(Sha256::digest(bytes)))
}

impl ReleaseWatchMetadata {
    fn validate_dispatch_contract(&self) -> Result<(), AgentRegistryError> {
        if !self.enabled {
            return Err(AgentRegistryError::Validation(
                "maintenance.release_watch.enabled=false is not allowed; omit maintenance.release_watch entirely when an agent is not enrolled".to_string(),
            ));
        }

        match self.dispatch_kind {
            ReleaseWatchDispatchKind::WorkflowDispatch => {
                let workflow = self.dispatch_workflow.as_deref().ok_or_else(|| {
                    AgentRegistryError::Validation(
                        "maintenance.release_watch.dispatch_workflow is required when dispatch_kind = `workflow_dispatch`".to_string(),
                    )
                })?;
                validate_non_empty_scalar("maintenance.release_watch.dispatch_workflow", workflow)?;
                if !workflow.ends_with(".yml") {
                    return Err(AgentRegistryError::Validation(format!(
                        "maintenance.release_watch.dispatch_workflow must reference a workflow file ending in `.yml` (got `{workflow}`)"
                    )));
                }
            }
            ReleaseWatchDispatchKind::PacketPr => {
                if self.dispatch_workflow.is_some() {
                    return Err(AgentRegistryError::Validation(
                        "maintenance.release_watch.dispatch_workflow must be omitted when dispatch_kind = `packet_pr`".to_string(),
                    ));
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseWatchVersionPolicy {
    LatestStableMinusOne,
    UpstreamStablePointer,
}

impl ReleaseWatchVersionPolicy {
    fn as_str(self) -> &'static str {
        match self {
            Self::LatestStableMinusOne => "latest_stable_minus_one",
            Self::UpstreamStablePointer => "upstream_stable_pointer",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseWatchDispatchKind {
    WorkflowDispatch,
    PacketPr,
}

impl ReleaseWatchDispatchKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::WorkflowDispatch => "workflow_dispatch",
            Self::PacketPr => "packet_pr",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseWatchUpstream {
    pub source_kind: ReleaseWatchSourceKind,
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(default)]
    pub repo: Option<String>,
    #[serde(default)]
    pub tag_prefix: Option<String>,
    #[serde(default)]
    pub bucket: Option<String>,
    #[serde(default)]
    pub prefix: Option<String>,
    #[serde(default)]
    pub version_marker: Option<String>,
    #[serde(default)]
    pub package: Option<String>,
    #[serde(default)]
    pub dist_tag: Option<String>,
}

impl ReleaseWatchUpstream {
    fn validate(&self) -> Result<(), AgentRegistryError> {
        match self.source_kind {
            ReleaseWatchSourceKind::GithubReleases => {
                validate_required_optional_scalar(
                    "maintenance.release_watch.upstream.owner",
                    self.owner.as_deref(),
                )?;
                validate_required_optional_scalar(
                    "maintenance.release_watch.upstream.repo",
                    self.repo.as_deref(),
                )?;
                validate_required_optional_scalar(
                    "maintenance.release_watch.upstream.tag_prefix",
                    self.tag_prefix.as_deref(),
                )?;
                validate_absent_optional_scalar(
                    "maintenance.release_watch.upstream.bucket",
                    self.bucket.as_deref(),
                    self.source_kind,
                )?;
                validate_absent_optional_scalar(
                    "maintenance.release_watch.upstream.prefix",
                    self.prefix.as_deref(),
                    self.source_kind,
                )?;
                validate_absent_optional_scalar(
                    "maintenance.release_watch.upstream.version_marker",
                    self.version_marker.as_deref(),
                    self.source_kind,
                )?;
                validate_absent_optional_scalar(
                    "maintenance.release_watch.upstream.package",
                    self.package.as_deref(),
                    self.source_kind,
                )?;
                validate_absent_optional_scalar(
                    "maintenance.release_watch.upstream.dist_tag",
                    self.dist_tag.as_deref(),
                    self.source_kind,
                )?;
            }
            ReleaseWatchSourceKind::GcsObjectListing => {
                validate_required_optional_scalar(
                    "maintenance.release_watch.upstream.bucket",
                    self.bucket.as_deref(),
                )?;
                validate_required_optional_scalar(
                    "maintenance.release_watch.upstream.prefix",
                    self.prefix.as_deref(),
                )?;
                validate_required_optional_scalar(
                    "maintenance.release_watch.upstream.version_marker",
                    self.version_marker.as_deref(),
                )?;
                validate_absent_optional_scalar(
                    "maintenance.release_watch.upstream.owner",
                    self.owner.as_deref(),
                    self.source_kind,
                )?;
                validate_absent_optional_scalar(
                    "maintenance.release_watch.upstream.repo",
                    self.repo.as_deref(),
                    self.source_kind,
                )?;
                validate_absent_optional_scalar(
                    "maintenance.release_watch.upstream.tag_prefix",
                    self.tag_prefix.as_deref(),
                    self.source_kind,
                )?;
                validate_absent_optional_scalar(
                    "maintenance.release_watch.upstream.package",
                    self.package.as_deref(),
                    self.source_kind,
                )?;
                validate_absent_optional_scalar(
                    "maintenance.release_watch.upstream.dist_tag",
                    self.dist_tag.as_deref(),
                    self.source_kind,
                )?;
            }
            ReleaseWatchSourceKind::NpmDistTag => {
                validate_required_optional_scalar(
                    "maintenance.release_watch.upstream.package",
                    self.package.as_deref(),
                )?;
                validate_required_optional_scalar(
                    "maintenance.release_watch.upstream.dist_tag",
                    self.dist_tag.as_deref(),
                )?;
                validate_absent_optional_scalar(
                    "maintenance.release_watch.upstream.owner",
                    self.owner.as_deref(),
                    self.source_kind,
                )?;
                validate_absent_optional_scalar(
                    "maintenance.release_watch.upstream.repo",
                    self.repo.as_deref(),
                    self.source_kind,
                )?;
                validate_absent_optional_scalar(
                    "maintenance.release_watch.upstream.tag_prefix",
                    self.tag_prefix.as_deref(),
                    self.source_kind,
                )?;
                validate_absent_optional_scalar(
                    "maintenance.release_watch.upstream.bucket",
                    self.bucket.as_deref(),
                    self.source_kind,
                )?;
                validate_absent_optional_scalar(
                    "maintenance.release_watch.upstream.prefix",
                    self.prefix.as_deref(),
                    self.source_kind,
                )?;
                validate_absent_optional_scalar(
                    "maintenance.release_watch.upstream.version_marker",
                    self.version_marker.as_deref(),
                    self.source_kind,
                )?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseWatchSourceKind {
    GithubReleases,
    GcsObjectListing,
    NpmDistTag,
}

impl ReleaseWatchSourceKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::GithubReleases => "github_releases",
            Self::GcsObjectListing => "gcs_object_listing",
            Self::NpmDistTag => "npm_dist_tag",
        }
    }
}

fn validate_required_optional_scalar(
    field_name: &str,
    value: Option<&str>,
) -> Result<(), AgentRegistryError> {
    let value = value.ok_or_else(|| {
        AgentRegistryError::Validation(format!("{field_name} is required for this upstream source"))
    })?;
    validate_non_empty_scalar(field_name, value)
}

fn validate_absent_optional_scalar(
    field_name: &str,
    value: Option<&str>,
    source_kind: ReleaseWatchSourceKind,
) -> Result<(), AgentRegistryError> {
    if value.is_some() {
        return Err(AgentRegistryError::Validation(format!(
            "{field_name} must not be set when maintenance.release_watch.upstream.source_kind = `{}`",
            source_kind.as_str()
        )));
    }
    Ok(())
}
