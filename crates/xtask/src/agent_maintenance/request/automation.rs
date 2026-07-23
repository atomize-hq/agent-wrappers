use std::{fs, path::Path};

use sha2::{Digest, Sha256};

use crate::{agent_registry::AgentRegistryEntry, workspace_mutation::WorkspacePathJail};

use super::super::contract_policy::{
    build_execution_contract, derive_detected_release_fields, LEGACY_EXECUTOR_ALIAS,
};
use super::{
    raw::{RawDetectedRelease, RawExecutionContract},
    validate::{
        validate_existing_repo_relative_string_array, validate_non_empty_scalar,
        validate_non_empty_string_array, validate_repo_relative_glob_path,
        validate_repo_relative_reference, validate_repo_relative_string_array,
        validate_sha256_value,
    },
    DetectedRelease, ExecutionContract, ExecutionContractRecovery, MaintenanceAction,
    MaintenanceRequestError, TriggerKind, AUTOMATED_ARTIFACT_VERSION,
};

#[allow(clippy::too_many_arguments)]
pub(super) fn validate_execution_contract(
    workspace_root: &Path,
    jail: &WorkspacePathJail,
    request_path: &Path,
    maintenance_root: &Path,
    registry_entry: &AgentRegistryEntry,
    trigger_kind: TriggerKind,
    detected_release: Option<&DetectedRelease>,
    raw: Option<RawExecutionContract>,
) -> Result<Option<ExecutionContract>, MaintenanceRequestError> {
    match (trigger_kind, raw) {
        (TriggerKind::UpstreamReleaseDetected, None) => Ok(None),
        (TriggerKind::UpstreamReleaseDetected, Some(raw_execution_contract)) => {
            let detected_release = detected_release.ok_or_else(|| {
                MaintenanceRequestError::Internal(format!(
                    "maintenance request `{}` is missing detected_release while validating execution_contract",
                    request_path.display()
                ))
            })?;
            let expected_contract = build_execution_contract(
                workspace_root,
                registry_entry,
                &request_path.display().to_string(),
                &maintenance_root.display().to_string(),
                &format!(".github/workflows/{}", detected_release.dispatch_workflow),
                &detected_release.target_version,
                &detected_release.branch_name,
            )
            .map_err(MaintenanceRequestError::Internal)?;

            validate_non_empty_scalar(
                request_path,
                "execution_contract.executor",
                &raw_execution_contract.executor,
            )?;
            let legacy_executor = raw_execution_contract.executor == LEGACY_EXECUTOR_ALIAS;
            if raw_execution_contract.executor != expected_contract.executor && !legacy_executor {
                return Err(MaintenanceRequestError::Validation(format!(
                    "maintenance request `{}` field `execution_contract.executor` must be `{}` (legacy `{}` accepted only for read compatibility)",
                    request_path.display(),
                    expected_contract.executor,
                    LEGACY_EXECUTOR_ALIAS
                )));
            }

            validate_repo_relative_reference(
                jail,
                request_path,
                "execution_contract.prompt_template_path",
                &raw_execution_contract.prompt_template_path,
            )?;
            if raw_execution_contract.prompt_template_path
                != expected_contract.prompt_template_path
            {
                return Err(MaintenanceRequestError::Validation(format!(
                    "maintenance request `{}` field `execution_contract.prompt_template_path` must be `{}` for agent `{}`",
                    request_path.display(),
                    expected_contract.prompt_template_path,
                    registry_entry.agent_id
                )));
            }
            validate_sha256_value(
                request_path,
                "execution_contract.prompt_sha256",
                &raw_execution_contract.prompt_sha256,
            )?;
            let rendered_prompt = render_execution_prompt(
                workspace_root,
                &raw_execution_contract.prompt_template_path,
                &detected_release.target_version,
            )?;
            let rendered_prompt_sha256 = hex::encode(Sha256::digest(rendered_prompt.as_bytes()));
            if raw_execution_contract.prompt_sha256 != rendered_prompt_sha256
                || raw_execution_contract.prompt_sha256 != expected_contract.prompt_sha256
            {
                return Err(MaintenanceRequestError::Validation(format!(
                    "maintenance request `{}` field `execution_contract.prompt_sha256` must match the rendered prompt template digest `{}`",
                    request_path.display(),
                    expected_contract.prompt_sha256
                )));
            }

            validate_repo_relative_glob_path(
                request_path,
                "execution_contract.pr_summary_path",
                &raw_execution_contract.pr_summary_path,
            )?;
            if raw_execution_contract.pr_summary_path != expected_contract.pr_summary_path {
                return Err(MaintenanceRequestError::Validation(format!(
                    "maintenance request `{}` field `execution_contract.pr_summary_path` must be `{}` under the same maintenance root",
                    request_path.display(),
                    expected_contract.pr_summary_path
                )));
            }

            validate_repo_relative_glob_path(
                request_path,
                "execution_contract.closeout_path",
                &raw_execution_contract.closeout_path,
            )?;
            if raw_execution_contract.closeout_path != expected_contract.closeout_path {
                return Err(MaintenanceRequestError::Validation(format!(
                    "maintenance request `{}` field `execution_contract.closeout_path` must be `{}` under the same maintenance root",
                    request_path.display(),
                    expected_contract.closeout_path
                )));
            }

            if !raw_execution_contract.requires_manual_closeout {
                return Err(MaintenanceRequestError::Validation(format!(
                    "maintenance request `{}` field `execution_contract.requires_manual_closeout` must be `true`",
                    request_path.display()
                )));
            }

            let writable_surfaces = validate_repo_relative_string_array(
                request_path,
                "execution_contract.writable_surfaces",
                &raw_execution_contract.writable_surfaces,
                true,
            )?;
            let read_only_inputs = validate_existing_repo_relative_string_array(
                jail,
                request_path,
                "execution_contract.read_only_inputs",
                &raw_execution_contract.read_only_inputs,
            )?;
            let ordered_commands = validate_non_empty_string_array(
                request_path,
                "execution_contract.ordered_commands",
                &raw_execution_contract.ordered_commands,
                true,
            )?;
            let green_gates = validate_non_empty_string_array(
                request_path,
                "execution_contract.green_gates",
                &raw_execution_contract.green_gates,
                true,
            )?;
            if registry_entry.maintenance.release_watch.is_some() && !legacy_executor {
                validate_exact_array(
                    request_path,
                    "execution_contract.writable_surfaces",
                    &writable_surfaces,
                    &expected_contract.writable_surfaces,
                )?;
                validate_exact_array(
                    request_path,
                    "execution_contract.read_only_inputs",
                    &read_only_inputs,
                    &expected_contract.read_only_inputs,
                )?;
                validate_exact_array(
                    request_path,
                    "execution_contract.ordered_commands",
                    &ordered_commands,
                    &expected_contract.ordered_commands,
                )?;
                validate_exact_array(
                    request_path,
                    "execution_contract.green_gates",
                    &green_gates,
                    &expected_contract.green_gates,
                )?;
            }

            validate_non_empty_scalar(
                request_path,
                "execution_contract.recovery.recreate_packet_command",
                &raw_execution_contract.recovery.recreate_packet_command,
            )?;
            if registry_entry.maintenance.release_watch.is_some()
                && !legacy_executor
                && raw_execution_contract.recovery.recreate_packet_command
                    != expected_contract.recovery.recreate_packet_command
            {
                return Err(MaintenanceRequestError::Validation(format!(
                    "maintenance request `{}` field `execution_contract.recovery.recreate_packet_command` must be `{}`",
                    request_path.display(),
                    expected_contract.recovery.recreate_packet_command
                )));
            }
            validate_repo_relative_glob_path(
                request_path,
                "execution_contract.recovery.reopen_pr_body_path",
                &raw_execution_contract.recovery.reopen_pr_body_path,
            )?;
            if raw_execution_contract.recovery.reopen_pr_body_path
                != expected_contract.recovery.reopen_pr_body_path
            {
                return Err(MaintenanceRequestError::Validation(format!(
                    "maintenance request `{}` field `execution_contract.recovery.reopen_pr_body_path` must match `execution_contract.pr_summary_path` `{}`",
                    request_path.display(),
                    expected_contract.recovery.reopen_pr_body_path
                )));
            }
            validate_non_empty_scalar(
                request_path,
                "execution_contract.recovery.reopen_pr_branch",
                &raw_execution_contract.recovery.reopen_pr_branch,
            )?;
            if raw_execution_contract.recovery.reopen_pr_branch != detected_release.branch_name {
                return Err(MaintenanceRequestError::Validation(format!(
                    "maintenance request `{}` field `execution_contract.recovery.reopen_pr_branch` must match `detected_release.branch_name` `{}`",
                    request_path.display(),
                    detected_release.branch_name
                )));
            }
            let recovery_notes = validate_non_empty_string_array(
                request_path,
                "execution_contract.recovery.notes",
                &raw_execution_contract.recovery.notes,
                true,
            )?;
            if registry_entry.maintenance.release_watch.is_some() && !legacy_executor {
                validate_exact_array(
                    request_path,
                    "execution_contract.recovery.notes",
                    &recovery_notes,
                    &expected_contract.recovery.notes,
                )?;
            }

            if legacy_executor {
                Ok(Some(ExecutionContract {
                    executor: raw_execution_contract.executor,
                    prompt_template_path: raw_execution_contract.prompt_template_path,
                    prompt_sha256: raw_execution_contract.prompt_sha256,
                    pr_summary_path: raw_execution_contract.pr_summary_path,
                    closeout_path: raw_execution_contract.closeout_path,
                    requires_manual_closeout: raw_execution_contract.requires_manual_closeout,
                    writable_surfaces,
                    read_only_inputs,
                    ordered_commands,
                    green_gates,
                    recovery: ExecutionContractRecovery {
                        recreate_packet_command: raw_execution_contract
                            .recovery
                            .recreate_packet_command,
                        reopen_pr_body_path: raw_execution_contract.recovery.reopen_pr_body_path,
                        reopen_pr_branch: raw_execution_contract.recovery.reopen_pr_branch,
                        notes: recovery_notes,
                    },
                }))
            } else {
                Ok(Some(expected_contract))
            }
        }
        (_, Some(_)) => Err(MaintenanceRequestError::Validation(format!(
            "maintenance request `{}` may only include `[execution_contract]` when `trigger_kind = \"upstream_release_detected\"`",
            request_path.display()
        ))),
        (_, None) => Ok(None),
    }
}

pub(super) fn validate_detected_release(
    registry_entry: &AgentRegistryEntry,
    request_path: &Path,
    trigger_kind: TriggerKind,
    raw: Option<RawDetectedRelease>,
) -> Result<Option<DetectedRelease>, MaintenanceRequestError> {
    match (trigger_kind, raw) {
        (TriggerKind::UpstreamReleaseDetected, Some(raw)) => {
            validate_non_empty_scalar(request_path, "detected_release.detected_by", &raw.detected_by)?;
            validate_non_empty_scalar(
                request_path,
                "detected_release.current_validated",
                &raw.current_validated,
            )?;
            validate_non_empty_scalar(
                request_path,
                "detected_release.target_version",
                &raw.target_version,
            )?;
            validate_non_empty_scalar(
                request_path,
                "detected_release.latest_stable",
                &raw.latest_stable,
            )?;
            validate_non_empty_scalar(
                request_path,
                "detected_release.version_policy",
                &raw.version_policy,
            )?;
            validate_non_empty_scalar(
                request_path,
                "detected_release.source_kind",
                &raw.source_kind,
            )?;
            validate_non_empty_scalar(request_path, "detected_release.source_ref", &raw.source_ref)?;
            validate_non_empty_scalar(
                request_path,
                "detected_release.dispatch_kind",
                &raw.dispatch_kind,
            )?;
            validate_non_empty_scalar(
                request_path,
                "detected_release.dispatch_workflow",
                &raw.dispatch_workflow,
            )?;
            validate_non_empty_scalar(
                request_path,
                "detected_release.branch_name",
                &raw.branch_name,
            )?;
            let raw_detected_release = DetectedRelease {
                detected_by: raw.detected_by,
                current_validated: raw.current_validated,
                target_version: raw.target_version,
                latest_stable: raw.latest_stable,
                version_policy: raw.version_policy,
                source_kind: raw.source_kind,
                source_ref: raw.source_ref,
                dispatch_kind: raw.dispatch_kind,
                dispatch_workflow: raw.dispatch_workflow,
                branch_name: raw.branch_name,
            };
            if let Some(release_watch) = registry_entry.maintenance.release_watch.as_ref() {
                let derived = derive_detected_release_fields(&registry_entry.agent_id, release_watch)
                    .map_err(MaintenanceRequestError::Internal)?;
                validate_exact_field(
                    request_path,
                    "detected_release.version_policy",
                    &raw_detected_release.version_policy,
                    &derived.version_policy,
                )?;
                validate_exact_field(
                    request_path,
                    "detected_release.source_kind",
                    &raw_detected_release.source_kind,
                    &derived.source_kind,
                )?;
                validate_exact_field(
                    request_path,
                    "detected_release.source_ref",
                    &raw_detected_release.source_ref,
                    &derived.source_ref,
                )?;
                validate_exact_field(
                    request_path,
                    "detected_release.dispatch_kind",
                    &raw_detected_release.dispatch_kind,
                    &derived.dispatch_kind,
                )?;
                validate_exact_field(
                    request_path,
                    "detected_release.dispatch_workflow",
                    &raw_detected_release.dispatch_workflow,
                    &derived.dispatch_workflow,
                )?;
                Ok(Some(super::super::contract_policy::normalize_detected_release(
                    &raw_detected_release,
                    &derived,
                )))
            } else {
                Ok(Some(raw_detected_release))
            }
        }
        (TriggerKind::UpstreamReleaseDetected, None) => Err(MaintenanceRequestError::Validation(
            format!(
                "maintenance request `{}` trigger_kind `upstream_release_detected` requires a `[detected_release]` table",
                request_path.display()
            ),
        )),
        (_, Some(_)) => Err(MaintenanceRequestError::Validation(format!(
            "maintenance request `{}` may only include `[detected_release]` when `trigger_kind = \"upstream_release_detected\"`",
            request_path.display()
        ))),
        (_, None) => Ok(None),
    }
}

pub(super) fn validate_automated_watch_request(
    request_path: &Path,
    artifact_version: &str,
    trigger_kind: TriggerKind,
    requested_control_plane_actions: &[MaintenanceAction],
) -> Result<(), MaintenanceRequestError> {
    if trigger_kind != TriggerKind::UpstreamReleaseDetected {
        return Ok(());
    }
    if artifact_version != AUTOMATED_ARTIFACT_VERSION {
        return Err(MaintenanceRequestError::Validation(format!(
            "maintenance request `{}` trigger_kind `upstream_release_detected` requires `artifact_version = \"{AUTOMATED_ARTIFACT_VERSION}\"`",
            request_path.display()
        )));
    }
    if requested_control_plane_actions != [MaintenanceAction::PacketDocRefresh] {
        return Err(MaintenanceRequestError::Validation(format!(
            "maintenance request `{}` trigger_kind `upstream_release_detected` requires `requested_control_plane_actions = [\"packet_doc_refresh\"]`",
            request_path.display()
        )));
    }
    Ok(())
}

fn render_execution_prompt(
    workspace_root: &Path,
    prompt_template_path: &str,
    target_version: &str,
) -> Result<String, MaintenanceRequestError> {
    let prompt_template =
        fs::read_to_string(workspace_root.join(prompt_template_path)).map_err(|err| {
            MaintenanceRequestError::Validation(format!(
                "read execution contract prompt template `{prompt_template_path}`: {err}"
            ))
        })?;
    Ok(prompt_template.replace("{{VERSION}}", target_version))
}

fn validate_exact_field(
    request_path: &Path,
    field_name: &str,
    actual: &str,
    expected: &str,
) -> Result<(), MaintenanceRequestError> {
    if actual == expected {
        return Ok(());
    }
    Err(MaintenanceRequestError::Validation(format!(
        "maintenance request `{}` field `{field_name}` must be `{expected}`",
        request_path.display()
    )))
}

fn validate_exact_array(
    request_path: &Path,
    field_name: &str,
    actual: &[String],
    expected: &[String],
) -> Result<(), MaintenanceRequestError> {
    if actual == expected {
        return Ok(());
    }
    Err(MaintenanceRequestError::Validation(format!(
        "maintenance request `{}` field `{field_name}` must match the shared maintenance contract",
        request_path.display()
    )))
}
