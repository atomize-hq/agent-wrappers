#[path = "request/automation.rs"]
mod automation;
#[path = "request/paths.rs"]
mod paths;
#[path = "request/raw.rs"]
mod raw;
#[path = "request/support_surface_audit_reconcile.rs"]
mod support_surface_audit_reconcile;
#[path = "request/validate.rs"]
mod validate;

use std::{
    fmt, fs,
    path::{Path, PathBuf},
};

use sha2::{Digest, Sha256};
use toml_edit::de::from_str;

use super::support_audit::SupportSurfaceAudit;
use crate::{
    agent_registry::{AgentRegistry, REGISTRY_RELATIVE_PATH},
    workspace_mutation::WorkspacePathJail,
};

use self::{
    automation::{
        validate_automated_watch_request, validate_detected_release, validate_execution_contract,
    },
    paths::{normalize_repo_relative_path, validate_request_path},
    raw::RawMaintenanceRequest,
    support_surface_audit_reconcile::validate_support_surface_audit,
    validate::{map_jail_error, validate_actions, validate_runtime_followup_required},
};

const DOCS_NEXT_ROOT: &str = "docs/agents/lifecycle";
const LEGACY_ARTIFACT_VERSION: &str = "1";
pub(crate) const AUTOMATED_ARTIFACT_VERSION: &str = "2";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaintenanceRequest {
    pub relative_path: String,
    pub canonical_path: PathBuf,
    pub sha256: String,
    pub maintenance_pack_prefix: String,
    pub maintenance_root: String,
    pub agent_id: String,
    pub trigger_kind: TriggerKind,
    pub basis_ref: String,
    pub opened_from: String,
    pub requested_control_plane_actions: Vec<MaintenanceAction>,
    pub runtime_followup_required: RuntimeFollowupRequired,
    pub detected_release: Option<DetectedRelease>,
    pub support_surface_audit: Option<SupportSurfaceAudit>,
    pub request_recorded_at: String,
    pub request_commit: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaintenanceRequestEnvelope {
    pub request: MaintenanceRequest,
    pub execution_contract: Option<ExecutionContract>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditReconciliation {
    Exact,
    Satisfied,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedMaintenanceRequestEnvelope {
    pub envelope: MaintenanceRequestEnvelope,
    pub support_surface_audit_reconciliation: Option<AuditReconciliation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeFollowupRequired {
    pub required: bool,
    pub items: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedRelease {
    pub detected_by: String,
    pub current_validated: String,
    pub target_version: String,
    pub latest_stable: String,
    pub version_policy: String,
    pub source_kind: String,
    pub source_ref: String,
    pub dispatch_kind: String,
    pub dispatch_workflow: String,
    pub branch_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionContract {
    pub executor: String,
    pub prompt_template_path: String,
    pub prompt_sha256: String,
    pub pr_summary_path: String,
    pub closeout_path: String,
    pub requires_manual_closeout: bool,
    pub writable_surfaces: Vec<String>,
    pub read_only_inputs: Vec<String>,
    pub ordered_commands: Vec<String>,
    pub green_gates: Vec<String>,
    pub recovery: ExecutionContractRecovery,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionContractRecovery {
    pub recreate_packet_command: String,
    pub reopen_pr_body_path: String,
    pub reopen_pr_branch: String,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerKind {
    DriftDetected,
    ManualReopen,
    PostReleaseAudit,
    UpstreamReleaseDetected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[allow(clippy::enum_variant_names)]
pub enum MaintenanceAction {
    PacketDocRefresh,
    SupportMatrixRefresh,
    CapabilityMatrixRefresh,
    ReleaseDocRefresh,
}

#[derive(Debug)]
pub enum MaintenanceRequestError {
    Validation(String),
    Internal(String),
}

impl fmt::Display for MaintenanceRequestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) | Self::Internal(message) => f.write_str(message),
        }
    }
}

impl TriggerKind {
    fn parse(value: &str, request_path: &Path) -> Result<Self, MaintenanceRequestError> {
        match value {
            "drift_detected" => Ok(Self::DriftDetected),
            "manual_reopen" => Ok(Self::ManualReopen),
            "post_release_audit" => Ok(Self::PostReleaseAudit),
            "upstream_release_detected" => Ok(Self::UpstreamReleaseDetected),
            other => Err(MaintenanceRequestError::Validation(format!(
                "maintenance request `{}` has invalid `trigger_kind` `{other}`; expected `drift_detected`, `manual_reopen`, `post_release_audit`, or `upstream_release_detected`",
                request_path.display()
            ))),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::DriftDetected => "drift_detected",
            Self::ManualReopen => "manual_reopen",
            Self::PostReleaseAudit => "post_release_audit",
            Self::UpstreamReleaseDetected => "upstream_release_detected",
        }
    }
}

impl MaintenanceRequest {
    pub fn is_automated_watch_request(&self) -> bool {
        matches!(self.trigger_kind, TriggerKind::UpstreamReleaseDetected)
    }
}

impl MaintenanceRequestEnvelope {
    pub fn require_execution_contract_for_relay(
        &self,
    ) -> Result<&ExecutionContract, MaintenanceRequestError> {
        self.execution_contract.as_ref().ok_or_else(|| {
            MaintenanceRequestError::Validation(format!(
                "maintenance request `{}` trigger_kind `upstream_release_detected` requires an `[execution_contract]` table for relay execution",
                self.request.relative_path
            ))
        })
    }
}

impl AuditReconciliation {
    pub fn human_status(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::Satisfied => "satisfied (frozen discovery preserved)",
        }
    }
}

impl MaintenanceAction {
    fn parse(value: &str, request_path: &Path) -> Result<Self, MaintenanceRequestError> {
        match value {
            "packet_doc_refresh" => Ok(Self::PacketDocRefresh),
            "support_matrix_refresh" => Ok(Self::SupportMatrixRefresh),
            "capability_matrix_refresh" => Ok(Self::CapabilityMatrixRefresh),
            "release_doc_refresh" => Ok(Self::ReleaseDocRefresh),
            other => Err(MaintenanceRequestError::Validation(format!(
                "maintenance request `{}` requested runtime-owned or unsupported action `{other}`; allowed actions: `packet_doc_refresh`, `support_matrix_refresh`, `capability_matrix_refresh`, `release_doc_refresh`",
                request_path.display()
            ))),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::PacketDocRefresh => "packet_doc_refresh",
            Self::SupportMatrixRefresh => "support_matrix_refresh",
            Self::CapabilityMatrixRefresh => "capability_matrix_refresh",
            Self::ReleaseDocRefresh => "release_doc_refresh",
        }
    }
}

pub fn load_request(
    workspace_root: &Path,
    request_path: &Path,
) -> Result<MaintenanceRequest, MaintenanceRequestError> {
    Ok(
        load_request_envelope_validated(workspace_root, request_path)?
            .envelope
            .request,
    )
}

pub(crate) use self::validate::{
    validate_commit_value, validate_non_empty_scalar, validate_repo_relative_reference,
    validate_rfc3339_utc,
};

pub fn load_request_envelope(
    workspace_root: &Path,
    request_path: &Path,
) -> Result<MaintenanceRequestEnvelope, MaintenanceRequestError> {
    Ok(load_request_envelope_validated(workspace_root, request_path)?.envelope)
}

pub fn load_request_envelope_validated(
    workspace_root: &Path,
    request_path: &Path,
) -> Result<ValidatedMaintenanceRequestEnvelope, MaintenanceRequestError> {
    let workspace_root = fs::canonicalize(workspace_root).map_err(|err| {
        MaintenanceRequestError::Internal(format!(
            "canonicalize {}: {err}",
            workspace_root.display()
        ))
    })?;
    let relative_path = normalize_repo_relative_path(&workspace_root, request_path)?;
    let maintenance_pack_prefix = validate_request_path(&relative_path)?;
    let maintenance_root = Path::new(DOCS_NEXT_ROOT).join(&maintenance_pack_prefix);

    let lexical_path = workspace_root.join(&relative_path);
    let canonical_path = fs::canonicalize(&lexical_path).map_err(|err| {
        MaintenanceRequestError::Validation(format!(
            "maintenance request `{}` does not resolve: {err}",
            relative_path.display()
        ))
    })?;
    if !canonical_path.starts_with(&workspace_root) {
        return Err(MaintenanceRequestError::Validation(format!(
            "maintenance request `{}` resolves outside workspace root",
            relative_path.display()
        )));
    }

    let bytes = fs::read(&canonical_path).map_err(|err| {
        MaintenanceRequestError::Validation(format!(
            "read maintenance request `{}`: {err}",
            relative_path.display()
        ))
    })?;
    let text = std::str::from_utf8(&bytes).map_err(|err| {
        MaintenanceRequestError::Validation(format!(
            "maintenance request `{}` must be valid utf-8: {err}",
            relative_path.display()
        ))
    })?;
    let raw: RawMaintenanceRequest = from_str(text).map_err(|err| {
        MaintenanceRequestError::Validation(format!(
            "parse maintenance request `{}`: {err}",
            relative_path.display()
        ))
    })?;

    validate_non_empty_scalar(&relative_path, "agent_id", &raw.agent_id)?;
    let expected_pack_prefix = format!("{}-maintenance", raw.agent_id);
    if maintenance_pack_prefix != expected_pack_prefix {
        return Err(MaintenanceRequestError::Validation(format!(
            "maintenance request `{}` belongs to maintenance pack `{maintenance_pack_prefix}` instead of `{expected_pack_prefix}`",
            relative_path.display()
        )));
    }

    if raw.artifact_version != LEGACY_ARTIFACT_VERSION
        && raw.artifact_version != AUTOMATED_ARTIFACT_VERSION
    {
        return Err(MaintenanceRequestError::Validation(format!(
            "maintenance request `{}` has unsupported `artifact_version` `{}`; expected `{LEGACY_ARTIFACT_VERSION}` or `{AUTOMATED_ARTIFACT_VERSION}`",
            relative_path.display(),
            raw.artifact_version
        )));
    }

    let registry = AgentRegistry::load(&workspace_root).map_err(|err| {
        MaintenanceRequestError::Internal(format!("load {REGISTRY_RELATIVE_PATH}: {err}"))
    })?;
    let registry_entry = registry.find(&raw.agent_id).ok_or_else(|| {
        MaintenanceRequestError::Validation(format!(
            "maintenance request `{}` references unknown agent_id `{}`; onboarded agents must already exist in {REGISTRY_RELATIVE_PATH}",
            relative_path.display(),
            raw.agent_id
        ))
    })?;

    let trigger_kind = TriggerKind::parse(&raw.trigger_kind, &relative_path)?;
    let jail = WorkspacePathJail::new(&workspace_root).map_err(map_jail_error)?;
    validate_repo_relative_reference(&jail, &relative_path, "basis_ref", &raw.basis_ref)?;
    validate_repo_relative_reference(&jail, &relative_path, "opened_from", &raw.opened_from)?;
    let requested_control_plane_actions =
        validate_actions(&relative_path, &raw.requested_control_plane_actions)?;
    let runtime_followup_required =
        validate_runtime_followup_required(&relative_path, raw.runtime_followup_required)?;
    let detected_release = validate_detected_release(
        registry_entry,
        &relative_path,
        trigger_kind,
        raw.detected_release,
    )?;
    let support_surface_audit_validation = validate_support_surface_audit(
        &workspace_root,
        &relative_path,
        registry_entry,
        trigger_kind,
        detected_release.as_ref(),
        raw.support_surface_audit,
    )?;
    let execution_contract = validate_execution_contract(
        &workspace_root,
        &jail,
        &relative_path,
        &maintenance_root,
        registry_entry,
        trigger_kind,
        detected_release.as_ref(),
        raw.execution_contract,
    )?;
    validate_automated_watch_request(
        &relative_path,
        &raw.artifact_version,
        trigger_kind,
        &requested_control_plane_actions,
    )?;
    validate_rfc3339_utc(
        &relative_path,
        "request_recorded_at",
        &raw.request_recorded_at,
    )?;
    validate_commit_value(&relative_path, "request_commit", &raw.request_commit)?;

    Ok(ValidatedMaintenanceRequestEnvelope {
        envelope: MaintenanceRequestEnvelope {
            request: MaintenanceRequest {
                relative_path: relative_path.display().to_string(),
                canonical_path,
                sha256: hex::encode(Sha256::digest(&bytes)),
                maintenance_pack_prefix,
                maintenance_root: maintenance_root.display().to_string(),
                agent_id: raw.agent_id,
                trigger_kind,
                basis_ref: raw.basis_ref,
                opened_from: raw.opened_from,
                requested_control_plane_actions,
                runtime_followup_required,
                detected_release,
                support_surface_audit: support_surface_audit_validation.audit,
                request_recorded_at: raw.request_recorded_at,
                request_commit: raw.request_commit,
            },
            execution_contract,
        },
        support_surface_audit_reconciliation: support_surface_audit_validation.reconciliation,
    })
}
