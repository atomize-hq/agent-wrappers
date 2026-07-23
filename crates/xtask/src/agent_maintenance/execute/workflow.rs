use std::{io::Write, path::Path};

use crate::agent_maintenance::{docs, request};

use super::{
    generate_run_id, now_rfc3339,
    packet::{build_run_status, load_prepared_packet, persist_run_packet},
    resolve_codex_binary,
    runtime::{execute_codex_write, run_codex_preflight, run_green_gates},
    types::{Context, InputContract, RecoveryContract, ValidationCheck, ValidationReport},
    validate::{
        diff_snapshots, exclude_explicit_diff_paths, operator_governance_diff_exclusions,
        snapshot_workspace, validate_prepared_packet, validate_written_paths,
    },
    Args, Error, EXECUTION_RUNS_ROOT, WORKFLOW_VERSION,
};

pub(super) fn build_context(workspace_root: &Path, args: &Args) -> Result<Context, Error> {
    let validated_envelope =
        request::load_request_envelope_validated(workspace_root, &args.request)?;
    let envelope = validated_envelope.envelope;
    if !envelope.request.is_automated_watch_request() {
        return Err(Error::Validation(format!(
            "execute-agent-maintenance only supports automated upstream-release requests; `{}` has trigger_kind `{}`",
            envelope.request.relative_path,
            envelope.request.trigger_kind.as_str()
        )));
    }

    let execution_contract = envelope
        .require_execution_contract_for_relay()
        .map_err(Error::from)?
        .clone();
    let rendered_packet =
        docs::render_execution_packet(workspace_root, &envelope.request, &execution_contract)
            .map_err(Error::Validation)?;
    let detected_release = envelope.request.detected_release.as_ref().ok_or_else(|| {
        Error::Validation(format!(
            "maintenance request `{}` is missing detected_release metadata",
            envelope.request.relative_path
        ))
    })?;
    let run_id = args.run_id.clone().unwrap_or_else(generate_run_id);
    let run_dir_rel = format!("{EXECUTION_RUNS_ROOT}/{run_id}");
    let closeout_command = format!(
        "cargo run -p xtask -- close-agent-maintenance --request {} --closeout {}",
        envelope.request.relative_path, execution_contract.closeout_path
    );
    let input_contract = if args.dry_run {
        Some(InputContract {
            workflow_version: WORKFLOW_VERSION.to_string(),
            generated_at: now_rfc3339()?,
            run_id: run_id.clone(),
            request_path: envelope.request.relative_path.clone(),
            request_sha256: envelope.request.sha256.clone(),
            maintenance_root: envelope.request.maintenance_root.clone(),
            agent_id: envelope.request.agent_id.clone(),
            target_version: detected_release.target_version.clone(),
            branch_name: detected_release.branch_name.clone(),
            executor: execution_contract.executor.clone(),
            prompt_sha256: execution_contract.prompt_sha256.clone(),
            closeout_path: execution_contract.closeout_path.clone(),
            closeout_command: closeout_command.clone(),
            writable_surfaces: execution_contract.writable_surfaces.clone(),
            read_only_inputs: execution_contract.read_only_inputs.clone(),
            ordered_commands: execution_contract.ordered_commands.clone(),
            green_gates: execution_contract.green_gates.clone(),
            recovery: RecoveryContract {
                recreate_packet_command: execution_contract
                    .recovery
                    .recreate_packet_command
                    .clone(),
                reopen_pr_body_path: execution_contract.recovery.reopen_pr_body_path.clone(),
                reopen_pr_branch: execution_contract.recovery.reopen_pr_branch.clone(),
                notes: execution_contract.recovery.notes.clone(),
            },
            ignored_diff_roots: vec![EXECUTION_RUNS_ROOT.to_string()],
            baseline: snapshot_workspace(workspace_root, &[Path::new(EXECUTION_RUNS_ROOT)])?,
        })
    } else {
        None
    };

    Ok(Context {
        run_id,
        codex_binary: resolve_codex_binary(args),
        run_dir: workspace_root.join(&run_dir_rel),
        run_dir_rel,
        envelope,
        execution_contract,
        rendered_packet,
        closeout_command,
        input_contract,
    })
}

pub(super) fn execute_dry_run<W: Write>(
    workspace_root: &Path,
    context: &Context,
    writer: &mut W,
) -> Result<(), Error> {
    let current_reconciliation =
        load_current_support_surface_reconciliation(workspace_root, context)?;
    let preflight = run_codex_preflight(workspace_root, context)?;
    let preflight_ok = preflight.errors.is_empty();
    let mut checks = preflight.checks.clone();
    if let Some(reconciliation) = current_reconciliation {
        checks.push(ValidationCheck {
            name: "support_surface_audit".to_string(),
            ok: true,
            message: format!("support_surface_audit: {}", reconciliation.human_status()),
        });
    }
    let report = ValidationReport {
        workflow_version: WORKFLOW_VERSION.to_string(),
        run_id: context.run_id.clone(),
        status: if preflight_ok {
            "pass".to_string()
        } else {
            "fail".to_string()
        },
        checks,
        errors: preflight.errors.clone(),
        preflight: Some(preflight.evidence.clone()),
        codex_execution: None,
        gate_results: Vec::new(),
    };
    let status = build_run_status(
        context,
        "dry_run",
        if preflight_ok {
            "dry_run_ready"
        } else {
            "dry_run_failed"
        },
        preflight_ok,
        Vec::new(),
        report.errors.clone(),
    )?;
    persist_run_packet(context, &report, &status, &[], None, "dry_run")?;

    if !preflight_ok {
        return Err(Error::Validation(report.errors.join("\n")));
    }

    writeln!(
        writer,
        "OK: execute-agent-maintenance dry-run packet prepared."
    )
    .map_err(|err| Error::Internal(format!("write stdout: {err}")))?;
    writeln!(writer, "run_id: {}", context.run_id)
        .map_err(|err| Error::Internal(format!("write stdout: {err}")))?;
    writeln!(writer, "run_dir: {}", context.run_dir_rel)
        .map_err(|err| Error::Internal(format!("write stdout: {err}")))?;
    writeln!(writer, "closeout_command: {}", context.closeout_command)
        .map_err(|err| Error::Internal(format!("write stdout: {err}")))?;
    if let Some(reconciliation) = current_reconciliation {
        writeln!(
            writer,
            "support_surface_audit: {}",
            reconciliation.human_status()
        )
        .map_err(|err| Error::Internal(format!("write stdout: {err}")))?;
    }
    writeln!(
        writer,
        "closeout remains manual; it is not run automatically."
    )
    .map_err(|err| Error::Internal(format!("write stdout: {err}")))?;
    Ok(())
}

pub(super) fn execute_write_mode<W: Write>(
    workspace_root: &Path,
    context: &Context,
    writer: &mut W,
) -> Result<(), Error> {
    let (prepared_contract, frozen_prompt) = load_prepared_packet(context)?;
    validate_prepared_packet(context, &prepared_contract, &frozen_prompt)?;

    let preflight = run_codex_preflight(workspace_root, context)?;
    let mut checks = preflight.checks.clone();
    let mut errors = preflight.errors.clone();
    let mut gate_results = Vec::new();
    let mut post_write_reconciliation =
        load_current_support_surface_reconciliation(workspace_root, context)?;
    let diff_exclusions =
        operator_governance_diff_exclusions(&context.envelope.request.maintenance_root);

    let codex_execution = if errors.is_empty() {
        Some(execute_codex_write(
            workspace_root,
            context,
            &frozen_prompt,
        )?)
    } else {
        None
    };

    let mut written_paths = Vec::new();
    if let Some(execution) = codex_execution.as_ref() {
        if execution.exit_code != 0 {
            errors.push(format!(
                "codex execution failed with exit code {}; inspect `{}` and `{}`",
                execution.exit_code, execution.stdout_path, execution.stderr_path
            ));
            checks.push(ValidationCheck {
                name: "codex_execution_exit".to_string(),
                ok: false,
                message: format!("codex exited with {}", execution.exit_code),
            });
        } else {
            checks.push(ValidationCheck {
                name: "codex_execution_exit".to_string(),
                ok: true,
                message: "codex exited successfully".to_string(),
            });
        }

        let snapshot_after_codex =
            snapshot_workspace(workspace_root, &[Path::new(EXECUTION_RUNS_ROOT)])?;
        let diff_after_codex = exclude_explicit_diff_paths(
            &diff_snapshots(&prepared_contract.baseline, &snapshot_after_codex),
            &diff_exclusions,
        );
        validate_written_paths(
            workspace_root,
            context,
            &diff_after_codex,
            "post_codex",
            &mut checks,
            &mut errors,
        )?;

        if errors.is_empty() {
            gate_results = run_green_gates(
                workspace_root,
                &prepared_contract.green_gates,
                &mut checks,
                &mut errors,
            )?;
            let snapshot_after_gates =
                snapshot_workspace(workspace_root, &[Path::new(EXECUTION_RUNS_ROOT)])?;
            written_paths = exclude_explicit_diff_paths(
                &diff_snapshots(&prepared_contract.baseline, &snapshot_after_gates),
                &diff_exclusions,
            );
            validate_written_paths(
                workspace_root,
                context,
                &written_paths,
                "post_gates",
                &mut checks,
                &mut errors,
            )?;
            if errors.is_empty() {
                post_write_reconciliation = revalidate_request_after_write(
                    workspace_root,
                    context,
                    &mut checks,
                    &mut errors,
                )?;
            }
        } else {
            written_paths = diff_after_codex;
        }
    }

    let passed = errors.is_empty();
    let report = ValidationReport {
        workflow_version: WORKFLOW_VERSION.to_string(),
        run_id: context.run_id.clone(),
        status: if passed { "pass" } else { "fail" }.to_string(),
        checks,
        errors: errors.clone(),
        preflight: Some(preflight.evidence),
        codex_execution: codex_execution.clone(),
        gate_results,
    };
    let status = build_run_status(
        context,
        "write",
        if passed {
            "write_validated"
        } else {
            "write_failed"
        },
        passed,
        written_paths.clone(),
        errors.clone(),
    )?;
    persist_run_packet(
        context,
        &report,
        &status,
        &written_paths,
        codex_execution.as_ref(),
        "write",
    )?;

    if !passed {
        return Err(Error::Validation(errors.join("\n")));
    }

    writeln!(
        writer,
        "OK: execute-agent-maintenance write validation complete."
    )
    .map_err(|err| Error::Internal(format!("write stdout: {err}")))?;
    writeln!(writer, "run_id: {}", context.run_id)
        .map_err(|err| Error::Internal(format!("write stdout: {err}")))?;
    writeln!(writer, "written_paths: {}", written_paths.len())
        .map_err(|err| Error::Internal(format!("write stdout: {err}")))?;
    writeln!(writer, "closeout_command: {}", context.closeout_command)
        .map_err(|err| Error::Internal(format!("write stdout: {err}")))?;
    if let Some(reconciliation) = post_write_reconciliation {
        writeln!(
            writer,
            "support_surface_audit: {}",
            reconciliation.human_status()
        )
        .map_err(|err| Error::Internal(format!("write stdout: {err}")))?;
    }
    writeln!(
        writer,
        "closeout remains manual; it is not run automatically."
    )
    .map_err(|err| Error::Internal(format!("write stdout: {err}")))?;
    Ok(())
}

fn revalidate_request_after_write(
    workspace_root: &Path,
    context: &Context,
    checks: &mut Vec<ValidationCheck>,
    errors: &mut Vec<String>,
) -> Result<Option<request::AuditReconciliation>, Error> {
    let request_path = Path::new(&context.envelope.request.relative_path);
    match request::load_request_envelope_validated(workspace_root, request_path) {
        Ok(validated_envelope) => {
            let reconciliation = validated_envelope
                .support_surface_audit_reconciliation
                .map(|state| state.human_status())
                .unwrap_or("not_applicable");
            checks.push(ValidationCheck {
                name: "request_revalidation".to_string(),
                ok: true,
                message: format!(
                    "frozen request revalidated after runtime writes and green gates; support_surface_audit: {reconciliation}"
                ),
            });
            Ok(validated_envelope.support_surface_audit_reconciliation)
        }
        Err(request::MaintenanceRequestError::Validation(message)) => {
            checks.push(ValidationCheck {
                name: "request_revalidation".to_string(),
                ok: false,
                message: "frozen request became stale after runtime writes and green gates"
                    .to_string(),
            });
            errors.push(message);
            Ok(None)
        }
        Err(request::MaintenanceRequestError::Internal(message)) => Err(Error::Internal(message)),
    }
}

fn load_current_support_surface_reconciliation(
    workspace_root: &Path,
    context: &Context,
) -> Result<Option<request::AuditReconciliation>, Error> {
    let request_path = Path::new(&context.envelope.request.relative_path);
    let validated_envelope = request::load_request_envelope_validated(workspace_root, request_path)
        .map_err(Error::from)?;
    Ok(validated_envelope.support_surface_audit_reconciliation)
}
