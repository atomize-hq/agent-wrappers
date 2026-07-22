use std::{
    collections::BTreeSet,
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

use clap::{ArgGroup, Parser};
use thiserror::Error;

use crate::{
    publication_refresh, release_doc,
    workspace_mutation::{
        apply_mutations, plan_create_or_replace, ApplySummary, WorkspaceMutationError,
        WorkspacePathJail,
    },
};

use super::{
    docs::build_packet_docs,
    request::{
        load_request_envelope_validated, AuditReconciliation, MaintenanceAction,
        MaintenanceRequest, MaintenanceRequestError,
    },
};

#[derive(Debug, Parser, Clone)]
#[command(group(
    ArgGroup::new("mode")
        .required(true)
        .args(["dry_run", "write"])
        .multiple(false)
))]
pub struct Args {
    #[arg(long)]
    pub request: PathBuf,

    #[arg(long)]
    pub dry_run: bool,

    #[arg(long)]
    pub write: bool,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}")]
    Validation(String),
    #[error("{0}")]
    Internal(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefreshPlan {
    pub request: MaintenanceRequest,
    pub support_surface_audit_reconciliation: Option<AuditReconciliation>,
    pub files: Vec<PlannedFile>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedFile {
    pub action: MaintenanceAction,
    pub relative_path: String,
    pub contents: Vec<u8>,
}

impl Error {
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Validation(_) => 2,
            Self::Internal(_) => 1,
        }
    }
}

impl RefreshPlan {
    pub fn planned_paths(&self) -> Vec<&str> {
        self.files
            .iter()
            .map(|file| file.relative_path.as_str())
            .collect()
    }
}

impl From<WorkspaceMutationError> for Error {
    fn from(err: WorkspaceMutationError) -> Self {
        match err {
            WorkspaceMutationError::Validation(message) => Self::Validation(message),
            WorkspaceMutationError::Internal(message) => Self::Internal(message),
        }
    }
}

impl From<MaintenanceRequestError> for Error {
    fn from(err: MaintenanceRequestError) -> Self {
        match err {
            MaintenanceRequestError::Validation(message) => Self::Validation(message),
            MaintenanceRequestError::Internal(message) => Self::Internal(message),
        }
    }
}

pub fn run(args: Args) -> Result<(), Error> {
    let workspace_root = resolve_workspace_root()?;
    let mut stdout = io::stdout();
    run_in_workspace(&workspace_root, args, &mut stdout)
}

pub fn run_in_workspace<W: Write>(
    workspace_root: &Path,
    args: Args,
    writer: &mut W,
) -> Result<(), Error> {
    let plan = build_refresh_plan(workspace_root, &args.request)?;
    write_plan_preview(writer, &plan, args.write)?;

    if args.write {
        let summary = apply_refresh_plan(workspace_root, &plan)?;
        writeln!(
            writer,
            "Applied {} planned files: {} written, {} identical.",
            summary.total, summary.written, summary.identical
        )
        .map_err(|err| Error::Internal(format!("write stdout: {err}")))?;
    } else {
        writeln!(
            writer,
            "Dry-run only; no files were written. {} planned files ready.",
            plan.files.len()
        )
        .map_err(|err| Error::Internal(format!("write stdout: {err}")))?;
    }

    Ok(())
}

pub fn build_refresh_plan(
    workspace_root: &Path,
    request_path: &Path,
) -> Result<RefreshPlan, Error> {
    let validated_envelope = load_request_envelope_validated(workspace_root, request_path)?;
    let request = validated_envelope.envelope.request;
    let support_surface_audit_reconciliation =
        validated_envelope.support_surface_audit_reconciliation;
    let mut files = Vec::new();
    let mut seen_paths = BTreeSet::new();

    for action in request.requested_control_plane_actions.iter().copied() {
        match action {
            MaintenanceAction::PacketDocRefresh => {
                for doc in build_packet_docs(workspace_root, &request).map_err(|err| {
                    Error::Internal(format!("render maintenance packet docs: {err}"))
                })? {
                    push_file(
                        &request,
                        &mut seen_paths,
                        &mut files,
                        action,
                        doc.relative_path,
                        doc.contents.into_bytes(),
                    )?;
                }
            }
            MaintenanceAction::SupportMatrixRefresh => {
                push_publication_action_files(
                    workspace_root,
                    &request,
                    &mut seen_paths,
                    &mut files,
                    action,
                    true,
                    false,
                )?;
            }
            MaintenanceAction::CapabilityMatrixRefresh => {
                push_publication_action_files(
                    workspace_root,
                    &request,
                    &mut seen_paths,
                    &mut files,
                    action,
                    false,
                    true,
                )?;
            }
            MaintenanceAction::ReleaseDocRefresh => {
                let markdown =
                    release_doc::render_release_doc(workspace_root).map_err(Error::Validation)?;
                push_file(
                    &request,
                    &mut seen_paths,
                    &mut files,
                    action,
                    release_doc::RELEASE_DOC_PATH.to_string(),
                    markdown.into_bytes(),
                )?;
            }
        }
    }

    Ok(RefreshPlan {
        request,
        support_surface_audit_reconciliation,
        files,
    })
}

pub fn apply_refresh_plan(
    workspace_root: &Path,
    plan: &RefreshPlan,
) -> Result<ApplySummary, Error> {
    let jail = WorkspacePathJail::new(workspace_root)?;
    let mutations = plan
        .files
        .iter()
        .map(|file| {
            plan_create_or_replace(
                &jail,
                PathBuf::from(&file.relative_path),
                file.contents.clone(),
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    apply_mutations(workspace_root, &mutations).map_err(Into::into)
}

fn resolve_workspace_root() -> Result<PathBuf, Error> {
    let current_dir = std::env::current_dir()
        .map_err(|err| Error::Internal(format!("resolve current directory: {err}")))?;
    for candidate in current_dir.ancestors() {
        let cargo_toml = candidate.join("Cargo.toml");
        let Ok(text) = fs::read_to_string(&cargo_toml) else {
            continue;
        };
        if text.contains("[workspace]") {
            return Ok(candidate.to_path_buf());
        }
    }

    Err(Error::Internal(format!(
        "could not resolve workspace root from {}",
        current_dir.display()
    )))
}

fn write_plan_preview<W: Write>(
    writer: &mut W,
    plan: &RefreshPlan,
    write_mode: bool,
) -> Result<(), Error> {
    writeln!(
        writer,
        "== REFRESH-AGENT {} ==",
        if write_mode { "WRITE" } else { "DRY RUN" }
    )
    .map_err(|err| Error::Internal(format!("write stdout: {err}")))?;
    writeln!(writer, "request: {}", plan.request.relative_path)
        .map_err(|err| Error::Internal(format!("write stdout: {err}")))?;
    writeln!(writer, "agent_id: {}", plan.request.agent_id)
        .map_err(|err| Error::Internal(format!("write stdout: {err}")))?;
    writeln!(
        writer,
        "trigger_kind: {}",
        plan.request.trigger_kind.as_str()
    )
    .map_err(|err| Error::Internal(format!("write stdout: {err}")))?;
    if let Some(reconciliation) = plan.support_surface_audit_reconciliation {
        writeln!(
            writer,
            "support_surface_audit: {}",
            reconciliation.human_status()
        )
        .map_err(|err| Error::Internal(format!("write stdout: {err}")))?;
    }
    writeln!(writer, "requested_control_plane_actions:")
        .map_err(|err| Error::Internal(format!("write stdout: {err}")))?;
    for action in &plan.request.requested_control_plane_actions {
        writeln!(writer, "- {}", action.as_str())
            .map_err(|err| Error::Internal(format!("write stdout: {err}")))?;
    }
    writeln!(writer, "planned_files:")
        .map_err(|err| Error::Internal(format!("write stdout: {err}")))?;
    for file in &plan.files {
        writeln!(
            writer,
            "- {} [{}]",
            file.relative_path,
            file.action.as_str()
        )
        .map_err(|err| Error::Internal(format!("write stdout: {err}")))?;
    }
    Ok(())
}

fn push_publication_action_files(
    workspace_root: &Path,
    request: &MaintenanceRequest,
    seen_paths: &mut BTreeSet<String>,
    files: &mut Vec<PlannedFile>,
    action: MaintenanceAction,
    support_enabled: bool,
    capability_enabled: bool,
) -> Result<(), Error> {
    for file in publication_refresh::build_publication_artifact_plan(
        workspace_root,
        support_enabled,
        capability_enabled,
    )
    .map_err(Error::Validation)?
    {
        push_file(
            request,
            seen_paths,
            files,
            action,
            file.relative_path,
            file.contents,
        )?;
    }
    Ok(())
}

fn push_file(
    request: &MaintenanceRequest,
    seen_paths: &mut BTreeSet<String>,
    files: &mut Vec<PlannedFile>,
    action: MaintenanceAction,
    relative_path: String,
    contents: Vec<u8>,
) -> Result<(), Error> {
    ensure_allowed_write_path(request, &relative_path)?;
    if !seen_paths.insert(relative_path.clone()) {
        return Err(Error::Internal(format!(
            "refresh plan attempted to write `{relative_path}` more than once"
        )));
    }
    files.push(PlannedFile {
        action,
        relative_path,
        contents,
    });
    Ok(())
}

fn ensure_allowed_write_path(
    request: &MaintenanceRequest,
    relative_path: &str,
) -> Result<(), Error> {
    let allowed_packet_paths = [
        format!("{}/README.md", request.maintenance_root),
        format!("{}/scope_brief.md", request.maintenance_root),
        format!("{}/seam_map.md", request.maintenance_root),
        format!("{}/threading.md", request.maintenance_root),
        format!("{}/review_surfaces.md", request.maintenance_root),
        format!("{}/OPS_PLAYBOOK.md", request.maintenance_root),
        format!("{}/CI_WORKFLOWS_PLAN.md", request.maintenance_root),
        format!("{}/HANDOFF.md", request.maintenance_root),
        format!(
            "{}/governance/execute-agent-maintenance-prompt.md",
            request.maintenance_root
        ),
        format!("{}/governance/pr-summary.md", request.maintenance_root),
        format!("{}/governance/remediation-log.md", request.maintenance_root),
    ];
    let is_generated_surface = matches!(
        relative_path,
        publication_refresh::SUPPORT_MATRIX_JSON_OUTPUT_PATH
            | publication_refresh::SUPPORT_MATRIX_MARKDOWN_OUTPUT_PATH
            | publication_refresh::AGENT_API_RUNTIME_SUPPORT_DATA_OUTPUT_PATH
            | publication_refresh::CAPABILITY_MATRIX_OUTPUT_PATH
    ) || relative_path == release_doc::RELEASE_DOC_PATH;

    if allowed_packet_paths
        .iter()
        .any(|path| path == relative_path)
        || is_generated_surface
    {
        return Ok(());
    }

    Err(Error::Validation(format!(
        "refresh plan attempted out-of-bounds write `{relative_path}`; maintenance refresh is limited to maintenance packet docs and generated publication surfaces"
    )))
}
