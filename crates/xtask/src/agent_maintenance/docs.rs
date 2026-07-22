use std::path::Path;

use sha2::Digest;

use crate::agent_registry::{AgentRegistry, AgentRegistryEntry};

use super::contract_policy::{
    build_execution_contract_for_request, packet_owned_ops_playbook_path,
    packet_owned_prompt_template_path, packet_owned_workflow_plan_path, packet_pr_prompt_template,
    render_prompt_template_for_contract, EXECUTE_HOST_SURFACE, EXECUTION_HOST_LABEL,
};
use super::request::{ExecutionContract, MaintenanceRequest, MaintenanceRequestEnvelope};

const OWNERSHIP_MARKER: &str =
    "<!-- generated-by: xtask agent-maintenance renderer; source-of-truth: governance/maintenance-request.toml -->";
const PR_SUMMARY_FILE_NAME: &str = "governance/pr-summary.md";
const CLOSEOUT_FILE_NAME: &str = "governance/maintenance-closeout.json";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedPacketDoc {
    pub relative_path: String,
    pub contents: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedExecutionPacket {
    pub prompt_contents: String,
    pub prompt_sha256: String,
    pub handoff_relative_path: String,
    pub handoff_contents: String,
    pub pr_summary_relative_path: String,
    pub pr_summary_contents: String,
}

pub fn build_packet_docs(
    workspace_root: &Path,
    request: &MaintenanceRequest,
) -> Result<Vec<RenderedPacketDoc>, String> {
    if request.is_automated_watch_request() {
        build_automated_packet_docs(workspace_root, request, None)
    } else {
        Ok(build_manual_packet_docs(request))
    }
}

pub fn packet_doc_relative_paths(
    workspace_root: &Path,
    request: &MaintenanceRequest,
) -> Result<Vec<String>, String> {
    build_packet_docs(workspace_root, request).map(|docs| {
        docs.into_iter()
            .map(|doc| doc.relative_path)
            .collect::<Vec<_>>()
    })
}

pub fn build_packet_docs_from_envelope(
    workspace_root: &Path,
    envelope: &MaintenanceRequestEnvelope,
) -> Result<Vec<RenderedPacketDoc>, String> {
    if envelope.request.is_automated_watch_request() {
        build_automated_packet_docs(
            workspace_root,
            &envelope.request,
            envelope.execution_contract.as_ref(),
        )
    } else {
        Ok(build_manual_packet_docs(&envelope.request))
    }
}

fn build_automated_packet_docs(
    workspace_root: &Path,
    request: &MaintenanceRequest,
    execution_contract: Option<&ExecutionContract>,
) -> Result<Vec<RenderedPacketDoc>, String> {
    let entry = load_registry_entry(workspace_root, &request.agent_id)?;
    let derived_contract = build_execution_contract_for_request(workspace_root, &entry, request)?;
    let execution_contract = match execution_contract {
        Some(contract) if contract.executor == EXECUTE_HOST_SURFACE => contract,
        _ => &derived_contract,
    };
    build_automated_packet_docs_from_contract(workspace_root, request, execution_contract)
}

fn build_manual_packet_docs(request: &MaintenanceRequest) -> Vec<RenderedPacketDoc> {
    let root = &request.maintenance_root;
    let actions = markdown_list(
        &request
            .requested_control_plane_actions
            .iter()
            .map(|action| format!("`{}`", action.as_str()))
            .collect::<Vec<_>>(),
    );
    let runtime_followup = if request.runtime_followup_required.required {
        markdown_list(&request.runtime_followup_required.items)
    } else {
        "- none recorded".to_string()
    };
    let trigger_context = render_trigger_context(request);
    let threading = if request.is_automated_watch_request() {
        format!(
            "# Threading\n\n1. Review the auto-generated request at `{}`.\n2. Confirm the detected release metadata and staged branch name under `{}`.\n3. Follow the packet to complete the maintenance work and close the lane with `close-agent-maintenance`.\n",
            request.relative_path, request.maintenance_root
        )
    } else {
        format!(
            "# Threading\n\n1. Run `check-agent-drift --agent {}`.\n2. Record the maintainer-authored request at `{}`.\n3. Apply `refresh-agent --dry-run` and `refresh-agent --write` using that request.\n4. Close the maintenance run with `close-agent-maintenance` once findings are resolved or explicitly deferred.\n",
            request.agent_id, request.relative_path
        )
    };

    vec![
        RenderedPacketDoc {
            relative_path: format!("{root}/README.md"),
            contents: wrap_markdown(&format!(
                "# {} maintenance\n\nThis packet tracks control-plane maintenance for `{}`.\n\n## Request\n\n- request artifact: `{}`\n- trigger kind: `{}`\n- basis ref: `{}`\n- opened from: `{}`\n- recorded at: `{}`\n- request commit: `{}`\n\n## Trigger context\n\n{}\n\n## Requested control-plane actions\n\n{}\n",
                request.agent_id,
                request.agent_id,
                request.relative_path,
                request.trigger_kind.as_str(),
                request.basis_ref,
                request.opened_from,
                request.request_recorded_at,
                request.request_commit,
                trigger_context,
                actions
            )),
        },
        RenderedPacketDoc {
            relative_path: format!("{root}/scope_brief.md"),
            contents: wrap_markdown(&format!(
                "# Scope brief\n\nThis maintenance lane is limited to control-plane docs and generated publication surfaces for `{}`.\n\nAllowed write envelope:\n\n- maintenance packet docs under `{}`\n- `cli_manifests/support_matrix/current.json`\n- `docs/specs/unified-agent-api/support-matrix.md`\n- `crates/agent_api/src/runtime_support_data.rs`\n- `docs/specs/unified-agent-api/capability-matrix.md`\n- `docs/crates-io-release.md`\n\nHistorical onboarding and implementation packet docs remain read-only inputs.\n",
                request.agent_id,
                request.maintenance_root
            )),
        },
        RenderedPacketDoc {
            relative_path: format!("{root}/seam_map.md"),
            contents: wrap_markdown(
                "# Seam map\n\nThis maintenance packet has one bounded seam: reconcile maintenance-owned docs and generated publication surfaces with the detector-emitted drift basis.\n",
            ),
        },
        RenderedPacketDoc {
            relative_path: format!("{root}/threading.md"),
            contents: wrap_markdown(&threading),
        },
        RenderedPacketDoc {
            relative_path: format!("{root}/review_surfaces.md"),
            contents: wrap_markdown(&format!(
                "# Review surfaces\n\n- `{}`\n- `{}`\n- `docs/specs/unified-agent-api/support-matrix.md`\n- `cli_manifests/support_matrix/current.json`\n- `crates/agent_api/src/runtime_support_data.rs`\n- `docs/specs/unified-agent-api/capability-matrix.md`\n- `docs/crates-io-release.md`\n- historical packet docs are detector inputs only and remain read-only\n",
                request.basis_ref, request.opened_from
            )),
        },
        RenderedPacketDoc {
            relative_path: format!("{root}/HANDOFF.md"),
            contents: wrap_markdown(&format!(
                "# Handoff\n\nCurrent maintenance request: `{}`.\n\n## Packet origin\n\n{}\n\n## Runtime follow-up\n\n- required: `{}`\n{}\n\n## Operator note\n\nMaintenance closeout is not finalized by `refresh-agent`; runtime-owned changes, if any, stay outside this write set.\n",
                request.relative_path,
                trigger_context,
                if request.runtime_followup_required.required {
                    "true"
                } else {
                    "false"
                },
                runtime_followup
            )),
        },
        RenderedPacketDoc {
            relative_path: format!("{root}/governance/remediation-log.md"),
            contents: wrap_markdown(&format!(
                "# Remediation log\n\nRefresh planned from `{}`.\n\n- basis ref: `{}`\n- trigger kind: `{}`\n- request sha256: `{}`\n\nNo maintenance closeout has been recorded yet.\n",
                request.relative_path,
                request.basis_ref,
                request.trigger_kind.as_str(),
                request.sha256
            )),
        },
    ]
}

pub fn render_execution_packet(
    workspace_root: &Path,
    request: &MaintenanceRequest,
    execution_contract: &ExecutionContract,
) -> Result<RenderedExecutionPacket, String> {
    if !request.is_automated_watch_request() {
        return Err(format!(
            "execution packet renderer requires trigger_kind `upstream_release_detected` for `{}`",
            request.relative_path
        ));
    }

    let detected_release = request.detected_release.as_ref().ok_or_else(|| {
        format!(
            "execution packet renderer requires detected_release metadata for `{}`",
            request.relative_path
        )
    })?;
    let entry = load_registry_entry(workspace_root, &request.agent_id)?;
    let prompt_contents = render_prompt_template_for_contract(
        workspace_root,
        &entry,
        &request.maintenance_root,
        &execution_contract.prompt_template_path,
        &detected_release.target_version,
    )?;
    let prompt_sha256 = hex::encode(sha2::Sha256::digest(prompt_contents.as_bytes()));
    if prompt_sha256 != execution_contract.prompt_sha256 {
        return Err(format!(
            "execution packet renderer prompt digest mismatch for `{}`: request truth expects `{}`, rendered `{prompt_sha256}`",
            request.relative_path, execution_contract.prompt_sha256
        ));
    }

    let handoff_relative_path = format!("{}/HANDOFF.md", request.maintenance_root);
    let expected_pr_summary_path = format!("{}/{}", request.maintenance_root, PR_SUMMARY_FILE_NAME);
    if execution_contract.pr_summary_path != expected_pr_summary_path {
        return Err(format!(
            "execution packet renderer pr-summary path mismatch for `{}`: expected `{expected_pr_summary_path}` under maintenance root `{}`",
            request.relative_path, request.maintenance_root
        ));
    }

    let expected_closeout_path = format!("{}/{}", request.maintenance_root, CLOSEOUT_FILE_NAME);
    if execution_contract.closeout_path != expected_closeout_path {
        return Err(format!(
            "execution packet renderer closeout path mismatch for `{}`: expected `{expected_closeout_path}` under maintenance root `{}`",
            request.relative_path, request.maintenance_root
        ));
    }
    if !execution_contract.requires_manual_closeout {
        return Err(format!(
            "execution packet renderer requires manual closeout for `{}`",
            request.relative_path
        ));
    }
    if execution_contract.recovery.reopen_pr_body_path != execution_contract.pr_summary_path {
        return Err(format!(
            "execution packet renderer recovery pr body mismatch for `{}`: expected `{}`",
            request.relative_path, execution_contract.pr_summary_path
        ));
    }
    if execution_contract.recovery.reopen_pr_branch != detected_release.branch_name {
        return Err(format!(
            "execution packet renderer branch linkage mismatch for `{}`: expected `{}`",
            request.relative_path, detected_release.branch_name
        ));
    }

    let trigger_context = render_trigger_context(request);
    let support_audit = render_support_surface_audit(request);
    let dry_run_command = format!(
        "cargo run -p xtask -- execute-agent-maintenance --dry-run --request {}",
        request.relative_path
    );
    let write_command = format!(
        "cargo run -p xtask -- execute-agent-maintenance --write --request {} --run-id RUN_ID_FROM_DRY_RUN",
        request.relative_path
    );
    let handoff_contents = wrap_markdown(&format!(
        "# Handoff\n\nThis file is the canonical contributor execution contract for `{}` maintenance.\n\n## Packet origin\n\n{}\n\n## Support-surface audit\n\n{}\n\n## Relay contract\n\n- maintained agent packet: `{}`\n- local execution host: `{}`\n- executor surface: `{}`\n- request artifact: `{}`\n- prompt template path: `{}`\n- prompt sha256: `{}`\n- canonical handoff: `{}`\n- derivative pr summary: `{}`\n- exact closeout artifact: `{}`\n- branch linkage: `{}`\n- manual closeout required: `{}`\n\n## Writable surfaces\n\n{}\n\n## Read-only inputs\n\n{}\n\n## Ordered repo commands\n\n{}\n\n## Exact green gates\n\n{}\n\n## Recovery\n\n- recreate packet command: `{}`\n- reopen pr body path: `{}`\n- reopen pr branch: `{}`\n- notes:\n{}\n\n## Dry-run to write relay\n\nUse the `run_id` printed by the dry-run output, replacing `RUN_ID_FROM_DRY_RUN` before invoking write mode.\n\n```sh\n{}\n{}\n```\n\n## Exact closeout command\n\n```sh\ncargo run -p xtask -- close-agent-maintenance --request {} --closeout {}\n```\n\n## Exact maintained-agent prompt\n\n```md\n{}\n```\n",
        request.agent_id,
        trigger_context,
        support_audit,
        request.agent_id,
        EXECUTION_HOST_LABEL,
        execution_contract.executor,
        request.relative_path,
        execution_contract.prompt_template_path,
        execution_contract.prompt_sha256,
        handoff_relative_path,
        execution_contract.pr_summary_path,
        execution_contract.closeout_path,
        detected_release.branch_name,
        if execution_contract.requires_manual_closeout {
            "true"
        } else {
            "false"
        },
        markdown_repo_path_list(&execution_contract.writable_surfaces),
        markdown_repo_path_list(&execution_contract.read_only_inputs),
        markdown_command_list(&execution_contract.ordered_commands),
        markdown_command_list(&execution_contract.green_gates),
        execution_contract.recovery.recreate_packet_command,
        execution_contract.recovery.reopen_pr_body_path,
        execution_contract.recovery.reopen_pr_branch,
        markdown_list(&execution_contract.recovery.notes),
        dry_run_command,
        write_command,
        request.relative_path,
        execution_contract.closeout_path,
        prompt_contents
    ));

    let pr_summary_contents = wrap_markdown(&format!(
        "# PR summary\n\nAutomated maintenance packet for `{}` target `{}`.\n\n- canonical execution contract: `{}`\n- request artifact: `{}`\n- branch: `{}`\n- opened from: `{}`\n- prompt sha256: `{}`\n\n## Support-surface audit\n\n{}\n\n## Next step\n\nFollow `{}` exactly. This PR summary is derivative from the same execution-packet renderer.\n\n## Exact maintained-agent prompt\n\n```md\n{}\n```\n",
        request.agent_id,
        detected_release.target_version,
        handoff_relative_path,
        request.relative_path,
        detected_release.branch_name,
        request.opened_from,
        execution_contract.prompt_sha256,
        support_audit,
        handoff_relative_path,
        prompt_contents
    ));

    Ok(RenderedExecutionPacket {
        prompt_contents,
        prompt_sha256,
        handoff_relative_path,
        handoff_contents,
        pr_summary_relative_path: execution_contract.pr_summary_path.clone(),
        pr_summary_contents,
    })
}

fn build_automated_packet_docs_from_contract(
    workspace_root: &Path,
    request: &MaintenanceRequest,
    execution_contract: &ExecutionContract,
) -> Result<Vec<RenderedPacketDoc>, String> {
    let rendered_execution_packet =
        render_execution_packet(workspace_root, request, execution_contract)?;
    let packet_owned_contract_inputs =
        build_packet_owned_contract_inputs(workspace_root, request, execution_contract)?;
    let root = &request.maintenance_root;
    let trigger_context = render_trigger_context(request);
    let support_audit = render_support_surface_audit(request);

    let mut docs = vec![
        RenderedPacketDoc {
            relative_path: format!("{root}/README.md"),
            contents: wrap_markdown(&format!(
                "# {} maintenance\n\nThis packet tracks automated upstream-release maintenance for `{}`.\n\n## Request\n\n- request artifact: `{}`\n- trigger kind: `{}`\n- basis ref: `{}`\n- opened from: `{}`\n- recorded at: `{}`\n- request commit: `{}`\n\n## Trigger context\n\n{}\n\n## Support-surface audit\n\n{}\n\n## Canonical execution contract\n\nUse `{}` as the exact contributor execution contract for this lane. The PR body summary under `{}` is derivative only.\n",
                request.agent_id,
                request.agent_id,
                request.relative_path,
                request.trigger_kind.as_str(),
                request.basis_ref,
                request.opened_from,
                request.request_recorded_at,
                request.request_commit,
                trigger_context,
                support_audit,
                rendered_execution_packet.handoff_relative_path,
                rendered_execution_packet.pr_summary_relative_path
            )),
        },
        RenderedPacketDoc {
            relative_path: format!("{root}/scope_brief.md"),
            contents: wrap_markdown(&format!(
                "# Scope brief\n\nThis automated maintenance lane is limited to the frozen shared packet for `{}` and the declared writable surfaces below.\n\n## Support uplift rule\n\nNewly discovered non-TUI surface must land in this run unless the frozen packet records one allowed deferral. Preexisting non-TUI gaps remain valid only when the packet ties them to the committed debt inventory.\n\n## Writable surfaces\n\n{}\n",
                request.agent_id,
                markdown_repo_path_list(&execution_contract.writable_surfaces)
            )),
        },
        RenderedPacketDoc {
            relative_path: format!("{root}/seam_map.md"),
            contents: wrap_markdown(
                "# Seam map\n\nThis maintenance packet has one bounded seam: render a contributor-ready execution contract for the detected upstream release while keeping `HANDOFF.md` canonical and `governance/pr-summary.md` derivative, and while freezing one exact support-surface audit for non-TUI uplift.\n",
            ),
        },
        RenderedPacketDoc {
            relative_path: format!("{root}/threading.md"),
            contents: wrap_markdown(&format!(
                "# Threading\n\n1. Review the auto-generated request at `{}` and the canonical contract at `{}`.\n2. Apply the exact coding-agent prompt from `HANDOFF.md` against branch `{}`.\n3. Author `{}` and run the exact closeout command from `HANDOFF.md` after the green gates pass.\n",
                request.relative_path,
                rendered_execution_packet.handoff_relative_path,
                execution_contract.recovery.reopen_pr_branch,
                execution_contract.closeout_path
            )),
        },
        RenderedPacketDoc {
            relative_path: format!("{root}/review_surfaces.md"),
            contents: wrap_markdown(&format!(
                "# Review surfaces\n\n## Writable surfaces\n\n{}\n\n## Read-only inputs\n\n{}\n\n## Support debt baseline\n\n- `docs/specs/unified-agent-api/non-tui-support-debt.md`\n",
                markdown_repo_path_list(&execution_contract.writable_surfaces),
                markdown_repo_path_list(&execution_contract.read_only_inputs)
            )),
        },
        RenderedPacketDoc {
            relative_path: format!("{root}/governance/remediation-log.md"),
            contents: wrap_markdown(&format!(
                "# Remediation log\n\nRefresh planned from `{}`.\n\n- basis ref: `{}`\n- trigger kind: `{}`\n- request sha256: `{}`\n- canonical handoff: `{}`\n- derivative pr summary: `{}`\n\nNo maintenance closeout has been recorded yet.\n",
                request.relative_path,
                request.basis_ref,
                request.trigger_kind.as_str(),
                request.sha256,
                rendered_execution_packet.handoff_relative_path,
                rendered_execution_packet.pr_summary_relative_path
            )),
        },
    ];
    docs.extend(packet_owned_contract_inputs);
    docs.push(RenderedPacketDoc {
        relative_path: rendered_execution_packet.handoff_relative_path.clone(),
        contents: rendered_execution_packet.handoff_contents,
    });
    docs.push(RenderedPacketDoc {
        relative_path: rendered_execution_packet.pr_summary_relative_path.clone(),
        contents: rendered_execution_packet.pr_summary_contents,
    });
    Ok(docs)
}

fn build_packet_owned_contract_inputs(
    workspace_root: &Path,
    request: &MaintenanceRequest,
    execution_contract: &ExecutionContract,
) -> Result<Vec<RenderedPacketDoc>, String> {
    let prompt_path = packet_owned_prompt_template_path(&request.maintenance_root);
    if execution_contract.prompt_template_path != prompt_path {
        return Ok(Vec::new());
    }

    let entry = load_registry_entry(workspace_root, &request.agent_id)?;
    let detected_release = request.detected_release.as_ref().ok_or_else(|| {
        format!(
            "packet-owned contract inputs require detected_release metadata for `{}`",
            request.relative_path
        )
    })?;
    let handoff_path = format!("{}/HANDOFF.md", request.maintenance_root);

    Ok(vec![
        RenderedPacketDoc {
            relative_path: packet_owned_ops_playbook_path(&request.maintenance_root),
            contents: wrap_markdown(&format!(
                "# Ops playbook\n\nThis packet-owned playbook freezes operator context for `{}` target `{}`.\n\n- request artifact: `{}`\n- basis ref: `{}`\n- opened from: `{}`\n- branch linkage: `{}`\n- canonical handoff: `{}`\n- recovery packet regeneration: `{}`\n",
                request.agent_id,
                detected_release.target_version,
                request.relative_path,
                request.basis_ref,
                request.opened_from,
                detected_release.branch_name,
                handoff_path,
                execution_contract.recovery.recreate_packet_command
            )),
        },
        RenderedPacketDoc {
            relative_path: packet_owned_workflow_plan_path(&request.maintenance_root),
            contents: wrap_markdown(&format!(
                "# CI workflows plan\n\nThis packet is opened from `{}` and relayed through `{}`.\n\n## Ordered repo commands\n\n{}\n\n## Exact green gates\n\n{}\n",
                request.opened_from,
                handoff_path,
                markdown_command_list(&execution_contract.ordered_commands),
                markdown_command_list(&execution_contract.green_gates)
            )),
        },
        RenderedPacketDoc {
            relative_path: prompt_path,
            contents: packet_pr_prompt_template(&entry, &request.maintenance_root),
        },
    ])
}

fn load_registry_entry(
    workspace_root: &Path,
    agent_id: &str,
) -> Result<AgentRegistryEntry, String> {
    let registry =
        AgentRegistry::load(workspace_root).map_err(|err| format!("load agent registry: {err}"))?;
    registry
        .find(agent_id)
        .cloned()
        .ok_or_else(|| format!("unknown agent_id `{agent_id}` in maintenance packet rendering"))
}

fn wrap_markdown(body: &str) -> String {
    format!("{OWNERSHIP_MARKER}\n\n{body}")
}

fn markdown_list(items: &[String]) -> String {
    items
        .iter()
        .map(|item| format!("- {item}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn markdown_repo_path_list(items: &[String]) -> String {
    items
        .iter()
        .map(|item| format!("- `{item}`"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn markdown_command_list(items: &[String]) -> String {
    items
        .iter()
        .map(|item| format!("- `{item}`"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_trigger_context(request: &MaintenanceRequest) -> String {
    if let Some(detected_release) = &request.detected_release {
        format!(
            "- detected_by: `{}`\n- current_validated: `{}`\n- target_version: `{}`\n- latest_stable: `{}`\n- version_policy: `{}`\n- source_kind: `{}`\n- source_ref: `{}`\n- dispatch_kind: `{}`\n- dispatch_workflow: `{}`\n- branch_name: `{}`",
            detected_release.detected_by,
            detected_release.current_validated,
            detected_release.target_version,
            detected_release.latest_stable,
            detected_release.version_policy,
            detected_release.source_kind,
            detected_release.source_ref,
            detected_release.dispatch_kind,
            detected_release.dispatch_workflow,
            detected_release.branch_name
        )
    } else {
        "- no automated release detection metadata recorded".to_string()
    }
}

fn render_support_surface_audit(request: &MaintenanceRequest) -> String {
    let Some(audit) = request.support_surface_audit.as_ref() else {
        return "- no support-surface audit recorded".to_string();
    };

    let deferred = if audit.deferred_preexisting_gaps.is_empty() {
        "- none".to_string()
    } else {
        audit
            .deferred_preexisting_gaps
            .iter()
            .map(|gap| {
                let follow_on = gap
                    .blocking_follow_on
                    .as_deref()
                    .unwrap_or("external blocker");
                format!(
                    "- `{}` `{}` via `{}` ({follow_on})",
                    gap.command_path, gap.surface_id, gap.defer_reason
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    let required = if audit.required_uplifts_this_run.is_empty() {
        "- none".to_string()
    } else {
        audit
            .required_uplifts_this_run
            .iter()
            .map(|gap| {
                format!(
                    "- `{}` `{}` via `{}`",
                    gap.command_path, gap.surface_id, gap.reason
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    format!(
        concat!(
            "- required: `{}`\n",
            "- pre-run debt count: `{}`\n",
            "- expected post-run debt count: `{}`\n",
            "- discovered upstream surface rows: `{}`\n",
            "- preexisting unsupported rows: `{}`\n",
            "- required uplifts this run:\n{}\n",
            "- deferred preexisting gaps:\n{}\n"
        ),
        if audit.required { "true" } else { "false" },
        audit.pre_run_debt_count,
        audit.expected_post_run_debt_count,
        audit.discovered_upstream_surface.len(),
        audit.preexisting_unsupported_surface.len(),
        required,
        deferred
    )
}
