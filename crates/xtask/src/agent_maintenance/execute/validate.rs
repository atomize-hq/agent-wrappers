use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use regex::Regex;
use sha2::{Digest, Sha256};

use crate::workspace_mutation::WorkspacePathJail;

use super::{
    types::{Context, InputContract, SnapshotFile, ValidationCheck, WorkspaceSnapshot},
    Error,
};

pub(super) fn validate_prepared_packet(
    context: &Context,
    prepared: &InputContract,
    frozen_prompt: &str,
) -> Result<(), Error> {
    let mut mismatches = Vec::new();
    if prepared.run_id != context.run_id {
        mismatches.push(format!(
            "run_id mismatch: prepared `{}` vs requested `{}`",
            prepared.run_id, context.run_id
        ));
    }
    if prepared.request_path != context.envelope.request.relative_path {
        mismatches.push(format!(
            "request path mismatch: prepared `{}` vs live `{}`",
            prepared.request_path, context.envelope.request.relative_path
        ));
    }
    if prepared.request_sha256 != context.envelope.request.sha256 {
        mismatches.push(format!(
            "request sha256 mismatch: prepared `{}` vs live `{}`",
            prepared.request_sha256, context.envelope.request.sha256
        ));
    }
    if prepared.agent_id != context.envelope.request.agent_id {
        mismatches.push(format!(
            "agent_id mismatch: prepared `{}` vs live `{}`",
            prepared.agent_id, context.envelope.request.agent_id
        ));
    }
    let detected_release = context
        .envelope
        .request
        .detected_release
        .as_ref()
        .ok_or_else(|| {
            Error::Validation(format!(
                "maintenance request `{}` is missing detected_release metadata",
                context.envelope.request.relative_path
            ))
        })?;
    if prepared.target_version != detected_release.target_version {
        mismatches.push(format!(
            "target_version mismatch: prepared `{}` vs live `{}`",
            prepared.target_version, detected_release.target_version
        ));
    }
    if prepared.branch_name != detected_release.branch_name {
        mismatches.push(format!(
            "branch_name mismatch: prepared `{}` vs live `{}`",
            prepared.branch_name, detected_release.branch_name
        ));
    }
    if prepared.prompt_sha256 != context.execution_contract.prompt_sha256 {
        mismatches.push(format!(
            "prompt digest mismatch: prepared `{}` vs live `{}`",
            prepared.prompt_sha256, context.execution_contract.prompt_sha256
        ));
    }
    let frozen_prompt_sha256 = hex::encode(Sha256::digest(frozen_prompt.as_bytes()));
    if frozen_prompt_sha256 != prepared.prompt_sha256 {
        mismatches.push(format!(
            "frozen prompt digest mismatch: prepared `{}` vs frozen `{frozen_prompt_sha256}`",
            prepared.prompt_sha256
        ));
    }
    if frozen_prompt != context.rendered_packet.prompt_contents {
        mismatches.push(
            "frozen prompt contents diverge from current request truth; rerun execute-agent-maintenance --dry-run before write mode".to_string(),
        );
    }
    if prepared.writable_surfaces != context.execution_contract.writable_surfaces {
        mismatches.push("writable_surfaces diverged from the prepared baseline".to_string());
    }
    if prepared.green_gates != context.execution_contract.green_gates {
        mismatches.push("green_gates diverged from the prepared baseline".to_string());
    }
    if prepared.closeout_path != context.execution_contract.closeout_path {
        mismatches.push("closeout_path diverged from the prepared baseline".to_string());
    }
    if prepared.closeout_command != context.closeout_command {
        mismatches.push("closeout command diverged from the prepared baseline".to_string());
    }
    if !mismatches.is_empty() {
        return Err(Error::Validation(format!(
            "prepared run packet `{}` no longer matches request truth:\n{}",
            context.run_dir_rel,
            mismatches.join("\n")
        )));
    }
    Ok(())
}

pub(super) fn validate_written_paths(
    workspace_root: &Path,
    context: &Context,
    changed_paths: &[String],
    phase: &str,
    checks: &mut Vec<ValidationCheck>,
    errors: &mut Vec<String>,
) -> Result<(), Error> {
    let jail = WorkspacePathJail::new(workspace_root)?;
    let mut violations = Vec::new();
    for path in changed_paths {
        jail.resolve(Path::new(path))?;
        if path == &context.execution_contract.closeout_path {
            violations.push(format!(
                "{path} (closeout remains manual and must not be written by execute-agent-maintenance)"
            ));
            continue;
        }
        if !matches_any_surface(path, &context.execution_contract.writable_surfaces)? {
            violations.push(path.clone());
        }
    }
    let boundary_ok = violations.is_empty();
    checks.push(ValidationCheck {
        name: format!("write_boundary::{phase}"),
        ok: boundary_ok,
        message: if boundary_ok {
            format!("{phase} diff stayed within writable_surfaces")
        } else {
            format!(
                "{phase} diff escaped writable_surfaces: {}",
                violations.join(", ")
            )
        },
    });
    if !boundary_ok {
        errors.push(format!(
            "write boundary violation during {phase}: {}",
            violations.join(", ")
        ));
    }

    let has_runtime_write = !changed_paths.is_empty();
    checks.push(ValidationCheck {
        name: format!("runtime_write::{phase}"),
        ok: has_runtime_write,
        message: if has_runtime_write {
            format!(
                "{phase} diff recorded {} changed paths",
                changed_paths.len()
            )
        } else {
            "no runtime-owned changes were recorded".to_string()
        },
    });
    if !has_runtime_write {
        errors.push(
            "write mode produced no runtime-owned output changes from the prepared baseline"
                .to_string(),
        );
    }
    Ok(())
}

pub(super) fn snapshot_workspace(
    workspace_root: &Path,
    ignored_roots: &[&Path],
) -> Result<WorkspaceSnapshot, Error> {
    let mut files = Vec::new();
    let ignored = ignored_roots
        .iter()
        .map(|path| workspace_root.join(path))
        .collect::<Vec<_>>();
    collect_snapshot_files(workspace_root, workspace_root, &ignored, &mut files)?;
    files.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(WorkspaceSnapshot { files })
}

pub(super) fn diff_snapshots(before: &WorkspaceSnapshot, after: &WorkspaceSnapshot) -> Vec<String> {
    let before_map = before
        .files
        .iter()
        .map(|file| (file.path.as_str(), file.sha256.as_str()))
        .collect::<BTreeMap<_, _>>();
    let after_map = after
        .files
        .iter()
        .map(|file| (file.path.as_str(), file.sha256.as_str()))
        .collect::<BTreeMap<_, _>>();
    before_map
        .keys()
        .chain(after_map.keys())
        .copied()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter(|path| before_map.get(path) != after_map.get(path))
        .map(str::to_string)
        .collect()
}

pub(super) fn operator_governance_diff_exclusions(maintenance_root: &str) -> Vec<String> {
    vec![
        format!("{maintenance_root}/governance/orchestration-friction-log.md"),
        format!("{maintenance_root}/governance/maintainer-follow-up.md"),
    ]
}

pub(super) fn exclude_explicit_diff_paths(
    changed_paths: &[String],
    excluded_paths: &[String],
) -> Vec<String> {
    let excluded = excluded_paths
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    changed_paths
        .iter()
        .filter(|path| !excluded.contains(path.as_str()))
        .cloned()
        .collect()
}

fn matches_any_surface(path: &str, surfaces: &[String]) -> Result<bool, Error> {
    surfaces.iter().try_fold(false, |matched, surface| {
        if matched {
            return Ok(true);
        }
        glob_matches(surface, path)
    })
}

fn glob_matches(pattern: &str, path: &str) -> Result<bool, Error> {
    let mut regex = String::from("^");
    let mut chars = pattern.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '*' => {
                if chars.peek() == Some(&'*') {
                    chars.next();
                    regex.push_str(".*");
                } else {
                    regex.push_str("[^/]*");
                }
            }
            '.' | '+' | '(' | ')' | '|' | '^' | '$' | '{' | '}' | '[' | ']' | '\\' => {
                regex.push('\\');
                regex.push(ch);
            }
            other => regex.push(other),
        }
    }
    regex.push('$');
    let compiled = Regex::new(&regex).map_err(|err| {
        Error::Internal(format!(
            "compile writable surface pattern `{pattern}`: {err}"
        ))
    })?;
    Ok(compiled.is_match(path))
}

fn collect_snapshot_files(
    workspace_root: &Path,
    current: &Path,
    ignored_roots: &[PathBuf],
    files: &mut Vec<SnapshotFile>,
) -> Result<(), Error> {
    let metadata = fs::symlink_metadata(current)
        .map_err(|err| Error::Internal(format!("stat {}: {err}", current.display())))?;
    if metadata.file_type().is_symlink() {
        return Ok(());
    }
    if current != workspace_root {
        if let Ok(relative) = current.strip_prefix(workspace_root) {
            if ignored_roots
                .iter()
                .any(|root| current == root || current.starts_with(root))
            {
                return Ok(());
            }
            if should_ignore_snapshot_path(relative) {
                return Ok(());
            }
        }
    }

    if metadata.is_dir() {
        for entry in fs::read_dir(current)
            .map_err(|err| Error::Internal(format!("read_dir {}: {err}", current.display())))?
        {
            let entry = entry
                .map_err(|err| Error::Internal(format!("read_dir {}: {err}", current.display())))?;
            collect_snapshot_files(workspace_root, &entry.path(), ignored_roots, files)?;
        }
        return Ok(());
    }

    let relative = current
        .strip_prefix(workspace_root)
        .map_err(|err| Error::Internal(format!("strip prefix {}: {err}", current.display())))?;
    let bytes = fs::read(current)
        .map_err(|err| Error::Internal(format!("read {}: {err}", current.display())))?;
    files.push(SnapshotFile {
        path: relative.to_string_lossy().replace('\\', "/"),
        sha256: hex::encode(Sha256::digest(bytes)),
    });
    Ok(())
}

fn should_ignore_snapshot_path(relative: &Path) -> bool {
    relative == Path::new(".git")
        || relative.starts_with("target")
        || relative
            .components()
            .any(|component| component.as_os_str() == "__pycache__")
        || relative
            .extension()
            .is_some_and(|extension| extension == "pyc")
}
