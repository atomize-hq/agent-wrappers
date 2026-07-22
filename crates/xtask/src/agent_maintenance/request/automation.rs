use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

use sha2::{Digest, Sha256};

use crate::{agent_registry::AgentRegistryEntry, workspace_mutation::WorkspacePathJail};

use super::super::contract_policy::{
    build_execution_contract, derive_detected_release_fields, LEGACY_EXECUTOR_ALIAS,
};
use super::super::support_audit::{
    allowed_deferrals, coverage_report_present_for_target, derive_support_surface_audit,
    excluded_surface_kinds, surface_kinds, DebtBackedSurface, DeferredGap, EligibleSurface,
    EvidenceBackedSurface, PublicationImpact, RequiredUplift, SupportSurfaceAudit, SurfaceIdentity,
};
use super::{
    raw::{
        RawDebtBackedSurface, RawDeferredGap, RawDetectedRelease, RawEligibleSurface,
        RawEvidenceBackedSurface, RawExecutionContract, RawPublicationImpact, RawRequiredUplift,
        RawSupportSurfaceAudit, RawSurfaceIdentity,
    },
    validate::{
        validate_existing_repo_relative_string_array, validate_non_empty_scalar,
        validate_non_empty_string_array, validate_repo_relative_glob_path,
        validate_repo_relative_reference, validate_repo_relative_string_array,
        validate_sha256_value,
    },
    AuditReconciliation, DetectedRelease, ExecutionContract, ExecutionContractRecovery,
    MaintenanceAction, MaintenanceRequestError, TriggerKind, AUTOMATED_ARTIFACT_VERSION,
};

pub(super) struct SupportSurfaceAuditValidation {
    pub audit: Option<SupportSurfaceAudit>,
    pub reconciliation: Option<AuditReconciliation>,
}

pub(super) fn validate_support_surface_audit(
    workspace_root: &Path,
    request_path: &Path,
    registry_entry: &AgentRegistryEntry,
    trigger_kind: TriggerKind,
    detected_release: Option<&DetectedRelease>,
    raw: Option<RawSupportSurfaceAudit>,
) -> Result<SupportSurfaceAuditValidation, MaintenanceRequestError> {
    match (trigger_kind, raw) {
        (TriggerKind::UpstreamReleaseDetected, Some(raw_audit)) => {
            let detected_release = detected_release.ok_or_else(|| {
                MaintenanceRequestError::Internal(format!(
                    "maintenance request `{}` is missing detected_release while validating support_surface_audit",
                    request_path.display()
                ))
            })?;
            let actual = map_raw_support_surface_audit(raw_audit);
            let frozen_had_discovery_work = !actual.discovered_upstream_surface.is_empty()
                || !actual.required_uplifts_this_run.is_empty();
            if !actual.required {
                return Err(MaintenanceRequestError::Validation(format!(
                    "maintenance request `{}` field `support_surface_audit.required` must be `true`",
                    request_path.display()
                )));
            }
            if actual.surface_kinds != surface_kinds() {
                return Err(MaintenanceRequestError::Validation(format!(
                    "maintenance request `{}` field `support_surface_audit.surface_kinds` must match the shared maintenance contract",
                    request_path.display()
                )));
            }
            if actual.excluded_surface_kinds != excluded_surface_kinds() {
                return Err(MaintenanceRequestError::Validation(format!(
                    "maintenance request `{}` field `support_surface_audit.excluded_surface_kinds` must match the shared maintenance contract",
                    request_path.display()
                )));
            }
            if actual.allowed_deferrals != allowed_deferrals() {
                return Err(MaintenanceRequestError::Validation(format!(
                    "maintenance request `{}` field `support_surface_audit.allowed_deferrals` must match the shared maintenance contract",
                    request_path.display()
                )));
            }
            if frozen_had_discovery_work {
                let report_dir = format!(
                    "{}/reports/{}",
                    registry_entry.manifest_root, detected_release.target_version
                );
                match coverage_report_present_for_target(
                    workspace_root,
                    registry_entry,
                    &detected_release.target_version,
                ) {
                    Ok(true) => {}
                    Ok(false) => {
                        return Err(MaintenanceRequestError::Validation(format!(
                            "maintenance request `{}` field `support_surface_audit` cannot confirm reconciliation because live coverage report evidence for target version `{}` is missing under `{}`",
                            request_path.display(),
                            detected_release.target_version,
                            report_dir
                        )));
                    }
                    Err(error) => {
                        return Err(MaintenanceRequestError::Validation(format!(
                            "maintenance request `{}` field `support_surface_audit` cannot confirm reconciliation because live coverage report evidence for target version `{}` under `{}` is unreadable: {}",
                            request_path.display(),
                            detected_release.target_version,
                            report_dir,
                            error
                        )));
                    }
                }
            }
            let expected = derive_support_surface_audit(workspace_root, registry_entry, detected_release)
                .map_err(MaintenanceRequestError::Internal)?;
            let reconciliation = reconcile_support_surface_audit(request_path, &actual, &expected)?;
            Ok(SupportSurfaceAuditValidation {
                audit: Some(actual),
                reconciliation: Some(reconciliation),
            })
        }
        (TriggerKind::UpstreamReleaseDetected, None) => {
            Err(MaintenanceRequestError::Validation(format!(
                "maintenance request `{}` trigger_kind `upstream_release_detected` requires a `[support_surface_audit]` table",
                request_path.display()
            )))
        }
        (_, Some(_)) => Err(MaintenanceRequestError::Validation(format!(
            "maintenance request `{}` may only include `[support_surface_audit]` when `trigger_kind = \"upstream_release_detected\"`",
            request_path.display()
        ))),
        (_, None) => Ok(SupportSurfaceAuditValidation {
            audit: None,
            reconciliation: None,
        }),
    }
}

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

fn reconcile_support_surface_audit(
    request_path: &Path,
    frozen: &SupportSurfaceAudit,
    live: &SupportSurfaceAudit,
) -> Result<AuditReconciliation, MaintenanceRequestError> {
    if frozen == live {
        return Ok(AuditReconciliation::Exact);
    }
    if support_surface_audit_satisfied(frozen, live) {
        return Ok(AuditReconciliation::Satisfied);
    }

    Err(MaintenanceRequestError::Validation(format!(
        "maintenance request `{}` field `support_surface_audit` no longer matches the live derived maintenance contract: {}",
        request_path.display(),
        describe_support_surface_audit_drift(frozen, live)
    )))
}

fn support_surface_audit_satisfied(
    frozen: &SupportSurfaceAudit,
    live: &SupportSurfaceAudit,
) -> bool {
    let frozen_had_work = !frozen.discovered_upstream_surface.is_empty()
        || !frozen.required_uplifts_this_run.is_empty();
    if !frozen_had_work {
        return false;
    }
    if !live.discovered_upstream_surface.is_empty() || !live.required_uplifts_this_run.is_empty() {
        return false;
    }
    if live.deferred_preexisting_gaps != frozen.deferred_preexisting_gaps {
        return false;
    }
    if live.pre_run_debt_count != frozen.deferred_preexisting_gaps.len() {
        return false;
    }
    if !live.removed_upstream_surface.is_empty() {
        return false;
    }

    let live_debt_rows = live
        .preexisting_unsupported_surface
        .iter()
        .map(DebtBackedSurface::identity)
        .collect::<BTreeSet<_>>();
    let frozen_deferred_rows = frozen
        .deferred_preexisting_gaps
        .iter()
        .map(DeferredGap::identity)
        .collect::<BTreeSet<_>>();
    live_debt_rows == frozen_deferred_rows
}

fn describe_support_surface_audit_drift(
    frozen: &SupportSurfaceAudit,
    live: &SupportSurfaceAudit,
) -> String {
    let mut diffs = Vec::new();
    push_scalar_diff(
        &mut diffs,
        "support_surface_audit.pre_run_debt_count",
        frozen.pre_run_debt_count,
        live.pre_run_debt_count,
    );
    push_scalar_diff(
        &mut diffs,
        "support_surface_audit.expected_post_run_debt_count",
        frozen.expected_post_run_debt_count,
        live.expected_post_run_debt_count,
    );
    push_row_diffs(
        &mut diffs,
        "support_surface_audit.discovered_upstream_surface",
        &frozen.discovered_upstream_surface,
        &live.discovered_upstream_surface,
        EvidenceBackedSurface::identity,
        |row| format!("evidence_ref={}", row.evidence_ref),
    );
    push_row_diffs(
        &mut diffs,
        "support_surface_audit.removed_upstream_surface",
        &frozen.removed_upstream_surface,
        &live.removed_upstream_surface,
        EvidenceBackedSurface::identity,
        |row| format!("evidence_ref={}", row.evidence_ref),
    );
    push_row_diffs(
        &mut diffs,
        "support_surface_audit.preexisting_unsupported_surface",
        &frozen.preexisting_unsupported_surface,
        &live.preexisting_unsupported_surface,
        DebtBackedSurface::identity,
        |row| format!("debt_ref={}", row.debt_ref),
    );
    push_row_diffs(
        &mut diffs,
        "support_surface_audit.eligible_preexisting_surface",
        &frozen.eligible_preexisting_surface,
        &live.eligible_preexisting_surface,
        EligibleSurface::identity,
        |row| format!("eligibility_reason={}", row.eligibility_reason),
    );
    push_row_diffs(
        &mut diffs,
        "support_surface_audit.missing_wrapper_support",
        &frozen.missing_wrapper_support,
        &live.missing_wrapper_support,
        Clone::clone,
        |_| String::new(),
    );
    push_row_diffs(
        &mut diffs,
        "support_surface_audit.missing_backend_support",
        &frozen.missing_backend_support,
        &live.missing_backend_support,
        Clone::clone,
        |_| String::new(),
    );
    push_row_diffs(
        &mut diffs,
        "support_surface_audit.required_uplifts_this_run",
        &frozen.required_uplifts_this_run,
        &live.required_uplifts_this_run,
        RequiredUplift::identity,
        |row| {
            format!(
                "reason={}; required_writes={}",
                row.reason,
                row.required_writes.join(",")
            )
        },
    );
    push_row_diffs(
        &mut diffs,
        "support_surface_audit.deferred_preexisting_gaps",
        &frozen.deferred_preexisting_gaps,
        &live.deferred_preexisting_gaps,
        DeferredGap::identity,
        |row| {
            let follow_on = row.blocking_follow_on.as_deref().unwrap_or("none");
            format!(
                "defer_reason={}; blocking_follow_on={follow_on}",
                row.defer_reason
            )
        },
    );
    push_row_diffs(
        &mut diffs,
        "support_surface_audit.publication_impacts",
        &frozen.publication_impacts,
        &live.publication_impacts,
        PublicationImpact::identity,
        |row| format!("surface_doc={}", row.surface_doc),
    );

    if diffs.is_empty() {
        "the frozen audit differs from the live derivation in an unclassified way".to_string()
    } else {
        diffs.join("; ")
    }
}

fn push_scalar_diff<T>(diffs: &mut Vec<String>, field_name: &str, frozen: T, live: T)
where
    T: PartialEq + std::fmt::Display,
{
    if frozen != live {
        diffs.push(format!(
            "{field_name} changed: frozen `{frozen}` vs live `{live}`"
        ));
    }
}

fn push_row_diffs<T, IdentityFn, DetailFn>(
    diffs: &mut Vec<String>,
    field_name: &str,
    frozen: &[T],
    live: &[T],
    identity: IdentityFn,
    detail: DetailFn,
) where
    IdentityFn: Fn(&T) -> SurfaceIdentity,
    DetailFn: Fn(&T) -> String,
{
    let frozen_map = frozen
        .iter()
        .map(|row| (identity(row), detail(row)))
        .collect::<BTreeMap<_, _>>();
    let live_map = live
        .iter()
        .map(|row| (identity(row), detail(row)))
        .collect::<BTreeMap<_, _>>();
    let identities = frozen_map
        .keys()
        .chain(live_map.keys())
        .cloned()
        .collect::<BTreeSet<_>>();

    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut changed = Vec::new();
    for identity in identities {
        match (frozen_map.get(&identity), live_map.get(&identity)) {
            (Some(frozen_detail), Some(live_detail)) if frozen_detail != live_detail => {
                changed.push(format!(
                    "{} [frozen: {}; live: {}]",
                    identity.describe(),
                    render_detail(frozen_detail),
                    render_detail(live_detail)
                ));
            }
            (Some(frozen_detail), None) => {
                removed.push(render_row(&identity, frozen_detail));
            }
            (None, Some(live_detail)) => {
                added.push(render_row(&identity, live_detail));
            }
            _ => {}
        }
    }

    if !added.is_empty() {
        diffs.push(format!("{field_name} added: {}", added.join(", ")));
    }
    if !removed.is_empty() {
        diffs.push(format!("{field_name} removed: {}", removed.join(", ")));
    }
    if !changed.is_empty() {
        diffs.push(format!("{field_name} changed: {}", changed.join(", ")));
    }
}

fn render_row(identity: &SurfaceIdentity, detail: &str) -> String {
    if detail.is_empty() {
        identity.describe()
    } else {
        format!("{} [{}]", identity.describe(), detail)
    }
}

fn render_detail(detail: &str) -> &str {
    if detail.is_empty() {
        "no additional detail"
    } else {
        detail
    }
}

fn map_raw_support_surface_audit(raw: RawSupportSurfaceAudit) -> SupportSurfaceAudit {
    SupportSurfaceAudit {
        required: raw.required,
        surface_kinds: raw.surface_kinds,
        excluded_surface_kinds: raw.excluded_surface_kinds,
        allowed_deferrals: raw.allowed_deferrals,
        pre_run_debt_count: raw.pre_run_debt_count,
        expected_post_run_debt_count: raw.expected_post_run_debt_count,
        discovered_upstream_surface: raw
            .discovered_upstream_surface
            .into_iter()
            .map(map_raw_evidence_backed_surface)
            .collect(),
        removed_upstream_surface: raw
            .removed_upstream_surface
            .into_iter()
            .map(map_raw_evidence_backed_surface)
            .collect(),
        preexisting_unsupported_surface: raw
            .preexisting_unsupported_surface
            .into_iter()
            .map(map_raw_debt_backed_surface)
            .collect(),
        eligible_preexisting_surface: raw
            .eligible_preexisting_surface
            .into_iter()
            .map(map_raw_eligible_surface)
            .collect(),
        missing_wrapper_support: raw
            .missing_wrapper_support
            .into_iter()
            .map(map_raw_surface_identity)
            .collect(),
        missing_backend_support: raw
            .missing_backend_support
            .into_iter()
            .map(map_raw_surface_identity)
            .collect(),
        required_uplifts_this_run: raw
            .required_uplifts_this_run
            .into_iter()
            .map(map_raw_required_uplift)
            .collect(),
        deferred_preexisting_gaps: raw
            .deferred_preexisting_gaps
            .into_iter()
            .map(map_raw_deferred_gap)
            .collect(),
        publication_impacts: raw
            .publication_impacts
            .into_iter()
            .map(map_raw_publication_impact)
            .collect(),
    }
}

fn map_raw_surface_identity(raw: RawSurfaceIdentity) -> SurfaceIdentity {
    SurfaceIdentity::new(raw.surface_kind, raw.command_path, raw.surface_id)
}

fn map_raw_evidence_backed_surface(raw: RawEvidenceBackedSurface) -> EvidenceBackedSurface {
    EvidenceBackedSurface {
        surface_kind: raw.surface_kind,
        command_path: raw.command_path,
        surface_id: raw.surface_id,
        evidence_ref: raw.evidence_ref,
    }
}

fn map_raw_debt_backed_surface(raw: RawDebtBackedSurface) -> DebtBackedSurface {
    DebtBackedSurface {
        surface_kind: raw.surface_kind,
        command_path: raw.command_path,
        surface_id: raw.surface_id,
        debt_ref: raw.debt_ref,
    }
}

fn map_raw_eligible_surface(raw: RawEligibleSurface) -> EligibleSurface {
    EligibleSurface {
        surface_kind: raw.surface_kind,
        command_path: raw.command_path,
        surface_id: raw.surface_id,
        eligibility_reason: raw.eligibility_reason,
    }
}

fn map_raw_required_uplift(raw: RawRequiredUplift) -> RequiredUplift {
    RequiredUplift {
        surface_kind: raw.surface_kind,
        command_path: raw.command_path,
        surface_id: raw.surface_id,
        reason: raw.reason,
        required_writes: raw.required_writes,
    }
}

fn map_raw_deferred_gap(raw: RawDeferredGap) -> DeferredGap {
    DeferredGap {
        surface_kind: raw.surface_kind,
        command_path: raw.command_path,
        surface_id: raw.surface_id,
        defer_reason: raw.defer_reason,
        blocking_follow_on: raw.blocking_follow_on,
    }
}

fn map_raw_publication_impact(raw: RawPublicationImpact) -> PublicationImpact {
    PublicationImpact {
        surface_kind: raw.surface_kind,
        command_path: raw.command_path,
        surface_id: raw.surface_id,
        surface_doc: raw.surface_doc,
    }
}
