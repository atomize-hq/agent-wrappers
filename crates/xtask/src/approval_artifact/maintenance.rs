use std::path::Path;

use serde::Serialize;
use sha2::{Digest, Sha256};
use toml_edit::TableLike;

use crate::agent_registry::{
    normalize_release_watch_metadata, normalized_release_watch_sha256,
    validate_release_watch_metadata, AgentRegistryError, NormalizedReleaseWatchMetadata,
    ReleaseWatchDispatchKind, ReleaseWatchMetadata, ReleaseWatchSourceKind, ReleaseWatchUpstream,
    ReleaseWatchVersionPolicy,
};

use super::{
    fields::{optional_string, required_bool, required_string, required_table},
    ApprovalArtifactError, ApprovalMaintenance, ApprovalMaintenanceDeferral,
    ApprovalMaintenanceMode, APPROVED_SCOPE_CREATE_LANE_CLOSEOUT,
};

#[derive(Debug, Clone, Serialize)]
struct NormalizedApprovalMaintenancePayload {
    mode: String,
    release_watch: Option<NormalizedReleaseWatchMetadata>,
    deferral: Option<ApprovalMaintenanceDeferral>,
}

pub(super) fn parse_maintenance(
    table: &dyn TableLike,
    relative_path: &Path,
) -> Result<ApprovalMaintenance, ApprovalArtifactError> {
    let mode = required_string(table, "mode", relative_path)?;
    match mode.as_str() {
        "release_watch_enrolled" => parse_release_watch_enrollment(table, relative_path, mode),
        "explicitly_deferred" => parse_explicit_deferral(table, relative_path, mode),
        _ => Err(ApprovalArtifactError::Validation(format!(
            "approval artifact `{}` field `descriptor.maintenance.mode` has invalid value `{mode}`; expected `release_watch_enrolled` or `explicitly_deferred`",
            relative_path.display()
        ))),
    }
}

fn parse_release_watch_enrollment(
    table: &dyn TableLike,
    relative_path: &Path,
    mode: String,
) -> Result<ApprovalMaintenance, ApprovalArtifactError> {
    if table.get("deferral").is_some() {
        return Err(ApprovalArtifactError::Validation(format!(
            "approval artifact `{}` must omit `descriptor.maintenance.deferral` when `descriptor.maintenance.mode = \"release_watch_enrolled\"`",
            relative_path.display()
        )));
    }
    let item = table.get("release_watch").ok_or_else(|| {
        ApprovalArtifactError::Validation(format!(
            "approval artifact `{}` is missing required table `descriptor.maintenance.release_watch`",
            relative_path.display()
        ))
    })?;
    let release_watch_table = item.as_table_like().ok_or_else(|| {
        ApprovalArtifactError::Validation(format!(
            "approval artifact `{}` field `descriptor.maintenance.release_watch` must be a table",
            relative_path.display()
        ))
    })?;
    let release_watch = parse_release_watch_metadata(release_watch_table, relative_path)?;
    let normalized = normalize_release_watch_metadata(&release_watch)
        .map_err(|err| map_release_watch_error(relative_path, err))?;
    let release_watch_sha256 = normalized_release_watch_sha256(&release_watch)
        .map_err(|err| map_release_watch_error(relative_path, err))?;
    let payload = NormalizedApprovalMaintenancePayload {
        mode,
        release_watch: Some(normalized),
        deferral: None,
    };
    Ok(ApprovalMaintenance {
        mode: ApprovalMaintenanceMode::ReleaseWatchEnrolled {
            release_watch,
            release_watch_sha256,
        },
        section_sha256: normalized_sha256(&payload)?,
    })
}

fn parse_explicit_deferral(
    table: &dyn TableLike,
    relative_path: &Path,
    mode: String,
) -> Result<ApprovalMaintenance, ApprovalArtifactError> {
    if table.get("release_watch").is_some() {
        return Err(ApprovalArtifactError::Validation(format!(
            "approval artifact `{}` must omit `descriptor.maintenance.release_watch` when `descriptor.maintenance.mode = \"explicitly_deferred\"`",
            relative_path.display()
        )));
    }
    let deferral = required_table(table, "deferral", relative_path)?;
    let deferral = ApprovalMaintenanceDeferral {
        reason: required_string(deferral, "reason", relative_path)?,
        follow_up: required_string(deferral, "follow_up", relative_path)?,
        approved_scope: required_string(deferral, "approved_scope", relative_path)?,
    };
    if deferral.approved_scope != APPROVED_SCOPE_CREATE_LANE_CLOSEOUT {
        return Err(ApprovalArtifactError::Validation(format!(
            "approval artifact `{}` field `descriptor.maintenance.deferral.approved_scope` must equal `{APPROVED_SCOPE_CREATE_LANE_CLOSEOUT}`",
            relative_path.display()
        )));
    }
    let payload = NormalizedApprovalMaintenancePayload {
        mode,
        release_watch: None,
        deferral: Some(deferral.clone()),
    };
    Ok(ApprovalMaintenance {
        mode: ApprovalMaintenanceMode::ExplicitlyDeferred {
            deferral_sha256: normalized_sha256(&deferral)?,
            deferral,
        },
        section_sha256: normalized_sha256(&payload)?,
    })
}

fn parse_release_watch_metadata(
    table: &dyn TableLike,
    relative_path: &Path,
) -> Result<ReleaseWatchMetadata, ApprovalArtifactError> {
    let upstream = required_table(table, "upstream", relative_path)?;
    let release_watch = ReleaseWatchMetadata {
        enabled: required_bool(table, "enabled", relative_path)?,
        version_policy: parse_release_watch_version_policy(
            &required_string(table, "version_policy", relative_path)?,
            relative_path,
        )?,
        dispatch_kind: parse_release_watch_dispatch_kind(
            &required_string(table, "dispatch_kind", relative_path)?,
            relative_path,
        )?,
        dispatch_workflow: optional_string(table, "dispatch_workflow", relative_path)?,
        upstream: ReleaseWatchUpstream {
            source_kind: parse_release_watch_source_kind(
                &required_string(upstream, "source_kind", relative_path)?,
                relative_path,
            )?,
            owner: optional_string(upstream, "owner", relative_path)?,
            repo: optional_string(upstream, "repo", relative_path)?,
            tag_prefix: optional_string(upstream, "tag_prefix", relative_path)?,
            bucket: optional_string(upstream, "bucket", relative_path)?,
            prefix: optional_string(upstream, "prefix", relative_path)?,
            version_marker: optional_string(upstream, "version_marker", relative_path)?,
            package: optional_string(upstream, "package", relative_path)?,
            dist_tag: optional_string(upstream, "dist_tag", relative_path)?,
        },
    };
    validate_release_watch_metadata(&release_watch)
        .map_err(|err| map_release_watch_error(relative_path, err))?;
    Ok(release_watch)
}

fn parse_release_watch_version_policy(
    value: &str,
    relative_path: &Path,
) -> Result<ReleaseWatchVersionPolicy, ApprovalArtifactError> {
    match value {
        "latest_stable_minus_one" => Ok(ReleaseWatchVersionPolicy::LatestStableMinusOne),
        "upstream_stable_pointer" => Ok(ReleaseWatchVersionPolicy::UpstreamStablePointer),
        _ => Err(ApprovalArtifactError::Validation(format!(
            "approval artifact `{}` field `descriptor.maintenance.release_watch.version_policy` has invalid value `{value}`",
            relative_path.display()
        ))),
    }
}

fn parse_release_watch_dispatch_kind(
    value: &str,
    relative_path: &Path,
) -> Result<ReleaseWatchDispatchKind, ApprovalArtifactError> {
    match value {
        "workflow_dispatch" => Ok(ReleaseWatchDispatchKind::WorkflowDispatch),
        "packet_pr" => Ok(ReleaseWatchDispatchKind::PacketPr),
        _ => Err(ApprovalArtifactError::Validation(format!(
            "approval artifact `{}` field `descriptor.maintenance.release_watch.dispatch_kind` has invalid value `{value}`",
            relative_path.display()
        ))),
    }
}

fn parse_release_watch_source_kind(
    value: &str,
    relative_path: &Path,
) -> Result<ReleaseWatchSourceKind, ApprovalArtifactError> {
    match value {
        "github_releases" => Ok(ReleaseWatchSourceKind::GithubReleases),
        "gcs_object_listing" => Ok(ReleaseWatchSourceKind::GcsObjectListing),
        "npm_dist_tag" => Ok(ReleaseWatchSourceKind::NpmDistTag),
        _ => Err(ApprovalArtifactError::Validation(format!(
            "approval artifact `{}` field `descriptor.maintenance.release_watch.upstream.source_kind` has invalid value `{value}`",
            relative_path.display()
        ))),
    }
}

fn map_release_watch_error(relative_path: &Path, err: AgentRegistryError) -> ApprovalArtifactError {
    ApprovalArtifactError::Validation(format!(
        "approval artifact `{}` field `descriptor.maintenance.release_watch` is invalid: {err}",
        relative_path.display()
    ))
}

fn normalized_sha256<T: Serialize>(value: &T) -> Result<String, ApprovalArtifactError> {
    let bytes = serde_json::to_vec(value).map_err(|err| {
        ApprovalArtifactError::Internal(format!(
            "serialize normalized approval maintenance payload: {err}"
        ))
    })?;
    Ok(hex::encode(Sha256::digest(bytes)))
}
