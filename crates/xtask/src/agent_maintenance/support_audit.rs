use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use crate::agent_registry::AgentRegistryEntry;

use super::request::DetectedRelease;

pub(crate) const NON_TUI_SUPPORT_DEBT_PATH: &str =
    "docs/specs/unified-agent-api/non-tui-support-debt.md";
pub(crate) const SUPPORT_MATRIX_DOC_PATH: &str = "docs/specs/unified-agent-api/support-matrix.md";
const SURFACE_KINDS: [&str; 5] = [
    "commands",
    "subcommands",
    "flags",
    "global_flags",
    "positional_args",
];
const EXCLUDED_SURFACE_KINDS: [&str; 1] = ["tui_only"];
const ALLOWED_DEFERRALS: [&str; 5] = [
    "upstream_not_machine_exposed",
    "platform_evidence_missing",
    "requires_new_infra",
    "requires_new_architectural_seam",
    "outside_registry_maintenance_write_envelope",
];

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SurfaceIdentity {
    pub surface_kind: String,
    pub command_path: String,
    pub surface_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceBackedSurface {
    pub surface_kind: String,
    pub command_path: String,
    pub surface_id: String,
    pub evidence_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebtBackedSurface {
    pub surface_kind: String,
    pub command_path: String,
    pub surface_id: String,
    pub debt_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EligibleSurface {
    pub surface_kind: String,
    pub command_path: String,
    pub surface_id: String,
    pub eligibility_reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequiredUplift {
    pub surface_kind: String,
    pub command_path: String,
    pub surface_id: String,
    pub reason: String,
    pub required_writes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeferredGap {
    pub surface_kind: String,
    pub command_path: String,
    pub surface_id: String,
    pub defer_reason: String,
    pub blocking_follow_on: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicationImpact {
    pub surface_kind: String,
    pub command_path: String,
    pub surface_id: String,
    pub surface_doc: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupportSurfaceAudit {
    pub required: bool,
    pub surface_kinds: Vec<String>,
    pub excluded_surface_kinds: Vec<String>,
    pub allowed_deferrals: Vec<String>,
    pub pre_run_debt_count: usize,
    pub expected_post_run_debt_count: usize,
    pub discovered_upstream_surface: Vec<EvidenceBackedSurface>,
    pub removed_upstream_surface: Vec<EvidenceBackedSurface>,
    pub preexisting_unsupported_surface: Vec<DebtBackedSurface>,
    pub eligible_preexisting_surface: Vec<EligibleSurface>,
    pub missing_wrapper_support: Vec<SurfaceIdentity>,
    pub missing_backend_support: Vec<SurfaceIdentity>,
    pub required_uplifts_this_run: Vec<RequiredUplift>,
    pub deferred_preexisting_gaps: Vec<DeferredGap>,
    pub publication_impacts: Vec<PublicationImpact>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebtInventoryRow {
    pub row_id: String,
    pub agent_id: String,
    pub surface_kind: String,
    pub command_path: String,
    pub surface_id: String,
    pub current_reason: String,
    pub blocker_class: String,
    pub owner: String,
    pub milestone: String,
    pub follow_on: String,
    pub evidence_ref: String,
}

impl SurfaceIdentity {
    pub fn new(surface_kind: String, command_path: String, surface_id: String) -> Self {
        Self {
            surface_kind,
            command_path,
            surface_id,
        }
    }

    pub(crate) fn describe(&self) -> String {
        format!(
            "surface_kind={} command_path={} surface_id={}",
            self.surface_kind, self.command_path, self.surface_id
        )
    }
}

impl EvidenceBackedSurface {
    pub(crate) fn identity(&self) -> SurfaceIdentity {
        SurfaceIdentity::new(
            self.surface_kind.clone(),
            self.command_path.clone(),
            self.surface_id.clone(),
        )
    }
}

impl DebtBackedSurface {
    pub(crate) fn identity(&self) -> SurfaceIdentity {
        SurfaceIdentity::new(
            self.surface_kind.clone(),
            self.command_path.clone(),
            self.surface_id.clone(),
        )
    }
}

impl EligibleSurface {
    pub(crate) fn identity(&self) -> SurfaceIdentity {
        SurfaceIdentity::new(
            self.surface_kind.clone(),
            self.command_path.clone(),
            self.surface_id.clone(),
        )
    }
}

impl RequiredUplift {
    pub(crate) fn identity(&self) -> SurfaceIdentity {
        SurfaceIdentity::new(
            self.surface_kind.clone(),
            self.command_path.clone(),
            self.surface_id.clone(),
        )
    }
}

impl DeferredGap {
    pub(crate) fn identity(&self) -> SurfaceIdentity {
        SurfaceIdentity::new(
            self.surface_kind.clone(),
            self.command_path.clone(),
            self.surface_id.clone(),
        )
    }
}

impl PublicationImpact {
    pub(crate) fn identity(&self) -> SurfaceIdentity {
        SurfaceIdentity::new(
            self.surface_kind.clone(),
            self.command_path.clone(),
            self.surface_id.clone(),
        )
    }
}

impl DebtInventoryRow {
    pub fn identity(&self) -> SurfaceIdentity {
        SurfaceIdentity::new(
            self.surface_kind.clone(),
            self.command_path.clone(),
            self.surface_id.clone(),
        )
    }

    pub fn debt_ref(&self) -> String {
        format!("{NON_TUI_SUPPORT_DEBT_PATH}#{}", self.row_id)
    }
}

pub(crate) fn derive_support_surface_audit(
    workspace_root: &Path,
    entry: &AgentRegistryEntry,
    detected_release: &DetectedRelease,
) -> Result<SupportSurfaceAudit, String> {
    let debt_rows = load_debt_inventory(workspace_root)?
        .into_iter()
        .filter(|row| row.agent_id == entry.agent_id)
        .collect::<Vec<_>>();
    let debt_by_identity = debt_rows
        .iter()
        .map(|row| (row.identity(), row))
        .collect::<BTreeMap<_, _>>();
    let report = load_current_gap_surfaces_if_present(
        workspace_root,
        entry,
        &detected_release.target_version,
    )?;
    let report_ref = report
        .as_ref()
        .map(|(path, _)| repo_relative(workspace_root, path))
        .transpose()?;
    let surfaces = report
        .as_ref()
        .map(|(_, surfaces)| surfaces.clone())
        .unwrap_or_else(|| debt_rows.iter().map(DebtInventoryRow::identity).collect());

    let mut preexisting = Vec::new();
    let mut discovered = Vec::new();
    let mut deferred = Vec::new();
    let mut publication_impacts = Vec::new();
    let mut removed = Vec::new();
    let missing_wrapper_support = surfaces.clone();
    let missing_backend_support = surfaces.clone();

    for surface in &surfaces {
        publication_impacts.push(PublicationImpact {
            surface_kind: surface.surface_kind.clone(),
            command_path: surface.command_path.clone(),
            surface_id: surface.surface_id.clone(),
            surface_doc: SUPPORT_MATRIX_DOC_PATH.to_string(),
        });
        if let Some(row) = debt_by_identity.get(surface) {
            preexisting.push(DebtBackedSurface {
                surface_kind: surface.surface_kind.clone(),
                command_path: surface.command_path.clone(),
                surface_id: surface.surface_id.clone(),
                debt_ref: row.debt_ref(),
            });
            deferred.push(DeferredGap {
                surface_kind: surface.surface_kind.clone(),
                command_path: surface.command_path.clone(),
                surface_id: surface.surface_id.clone(),
                defer_reason: row.blocker_class.clone(),
                blocking_follow_on: Some(row.follow_on.clone()),
            });
        } else {
            let evidence_ref = report_ref
                .clone()
                .unwrap_or_else(|| NON_TUI_SUPPORT_DEBT_PATH.to_string());
            discovered.push(EvidenceBackedSurface {
                surface_kind: surface.surface_kind.clone(),
                command_path: surface.command_path.clone(),
                surface_id: surface.surface_id.clone(),
                evidence_ref,
            });
        }
    }

    if report.is_some() {
        for row in &debt_rows {
            let identity = row.identity();
            if !surfaces.iter().any(|surface| surface == &identity) {
                removed.push(EvidenceBackedSurface {
                    surface_kind: identity.surface_kind,
                    command_path: identity.command_path,
                    surface_id: identity.surface_id,
                    evidence_ref: row.evidence_ref.clone(),
                });
            }
        }
    }

    let required_uplifts_this_run = discovered
        .iter()
        .map(|surface| RequiredUplift {
            surface_kind: surface.surface_kind.clone(),
            command_path: surface.command_path.clone(),
            surface_id: surface.surface_id.clone(),
            reason: "new_upstream_surface".to_string(),
            required_writes: vec![
                "wrapper".to_string(),
                "backend".to_string(),
                "manifest".to_string(),
                "publication".to_string(),
                "packet_docs".to_string(),
            ],
        })
        .collect::<Vec<_>>();

    Ok(SupportSurfaceAudit {
        required: true,
        surface_kinds: SURFACE_KINDS.iter().map(ToString::to_string).collect(),
        excluded_surface_kinds: EXCLUDED_SURFACE_KINDS
            .iter()
            .map(ToString::to_string)
            .collect(),
        allowed_deferrals: ALLOWED_DEFERRALS.iter().map(ToString::to_string).collect(),
        pre_run_debt_count: debt_rows.len(),
        expected_post_run_debt_count: preexisting.len(),
        discovered_upstream_surface: discovered,
        removed_upstream_surface: removed,
        preexisting_unsupported_surface: preexisting,
        eligible_preexisting_surface: Vec::new(),
        missing_wrapper_support,
        missing_backend_support,
        required_uplifts_this_run,
        deferred_preexisting_gaps: deferred,
        publication_impacts,
    })
}

pub(crate) fn coverage_report_present_for_target(
    workspace_root: &Path,
    entry: &AgentRegistryEntry,
    target_version: &str,
) -> Result<bool, String> {
    let version_dir = coverage_report_version_dir(workspace_root, entry, target_version);
    if !version_dir.is_dir() {
        return Ok(false);
    }

    let report_path = match select_report_path(&version_dir) {
        Ok(path) => path,
        Err(error) if error.starts_with("no coverage report found under") => return Ok(false),
        Err(error) => return Err(error),
    };
    fs::File::open(&report_path).map_err(|err| format!("open {}: {err}", report_path.display()))?;
    Ok(true)
}

pub(crate) fn load_debt_inventory(workspace_root: &Path) -> Result<Vec<DebtInventoryRow>, String> {
    let path = workspace_root.join(NON_TUI_SUPPORT_DEBT_PATH);
    let text =
        fs::read_to_string(&path).map_err(|err| format!("read {}: {err}", path.display()))?;
    let mut rows = Vec::new();
    let mut current_row_id: Option<String> = None;
    let mut current_fields = BTreeMap::<String, String>::new();

    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("### `") {
            if let Some(row_id) = current_row_id.take() {
                rows.push(build_debt_row(&row_id, &current_fields)?);
                current_fields.clear();
            }
            let row_id = rest
                .strip_suffix('`')
                .ok_or_else(|| format!("invalid debt heading in {}", path.display()))?;
            current_row_id = Some(row_id.to_string());
            continue;
        }
        if current_row_id.is_some() {
            if let Some(rest) = line.strip_prefix("- `") {
                let (key, value) = parse_key_value(rest).ok_or_else(|| {
                    format!("invalid debt row field `{line}` in {}", path.display())
                })?;
                current_fields.insert(key.to_string(), value.to_string());
            }
        }
    }

    if let Some(row_id) = current_row_id.take() {
        rows.push(build_debt_row(&row_id, &current_fields)?);
    }

    Ok(rows)
}

fn load_current_gap_surfaces_if_present(
    workspace_root: &Path,
    entry: &AgentRegistryEntry,
    target_version: &str,
) -> Result<Option<(PathBuf, Vec<SurfaceIdentity>)>, String> {
    let version_dir = coverage_report_version_dir(workspace_root, entry, target_version);
    if !version_dir.is_dir() {
        return Ok(None);
    }

    match load_current_gap_surfaces(workspace_root, entry, target_version) {
        Ok(result) => Ok(Some(result)),
        Err(error)
            if error.starts_with("no coverage report found under")
                || error.starts_with("read_dir(") =>
        {
            Ok(None)
        }
        Err(error) => Err(error),
    }
}

pub(crate) fn allowed_deferrals() -> Vec<String> {
    ALLOWED_DEFERRALS.iter().map(ToString::to_string).collect()
}

pub(crate) fn surface_kinds() -> Vec<String> {
    SURFACE_KINDS.iter().map(ToString::to_string).collect()
}

pub(crate) fn excluded_surface_kinds() -> Vec<String> {
    EXCLUDED_SURFACE_KINDS
        .iter()
        .map(ToString::to_string)
        .collect()
}

fn build_debt_row(
    row_id: &str,
    fields: &BTreeMap<String, String>,
) -> Result<DebtInventoryRow, String> {
    let get = |key: &str| {
        fields
            .get(key)
            .cloned()
            .ok_or_else(|| format!("debt row `{row_id}` is missing required field `{key}`"))
    };
    let blocker_class = get("blocker_class")?;
    if !ALLOWED_DEFERRALS
        .iter()
        .any(|candidate| *candidate == blocker_class)
    {
        return Err(format!(
            "debt row `{row_id}` has invalid blocker_class `{blocker_class}`"
        ));
    }
    Ok(DebtInventoryRow {
        row_id: row_id.to_string(),
        agent_id: get("agent_id")?,
        surface_kind: get("surface_kind")?,
        command_path: get("command_path")?,
        surface_id: get("surface_id")?,
        current_reason: get("current_reason")?,
        blocker_class,
        owner: get("owner")?,
        milestone: get("milestone")?,
        follow_on: get("follow_on")?,
        evidence_ref: get("evidence_ref")?,
    })
}

fn parse_key_value(input: &str) -> Option<(&str, &str)> {
    let (key, rest) = input.split_once("`: `")?;
    let value = rest.strip_suffix('`')?;
    Some((key, value))
}

fn load_current_gap_surfaces(
    workspace_root: &Path,
    entry: &AgentRegistryEntry,
    target_version: &str,
) -> Result<(PathBuf, Vec<SurfaceIdentity>), String> {
    let version_dir = coverage_report_version_dir(workspace_root, entry, target_version);
    let report_path = select_report_path(&version_dir)?;
    let text = fs::read_to_string(&report_path)
        .map_err(|err| format!("read {}: {err}", report_path.display()))?;
    let json = serde_json::from_str::<serde_json::Value>(&text)
        .map_err(|err| format!("parse {}: {err}", report_path.display()))?;
    let deltas = json
        .get("deltas")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| format!("{} is missing `deltas` object", report_path.display()))?;

    let mut surfaces = BTreeSet::new();
    for key in [
        "missing_commands",
        "missing_flags",
        "missing_args",
        "intentionally_unsupported",
    ] {
        let rows = deltas
            .get(key)
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| format!("{} is missing `deltas.{key}` array", report_path.display()))?;
        for row in rows {
            surfaces.insert(surface_from_report_value(&entry.agent_id, row)?);
        }
    }

    Ok((report_path, surfaces.into_iter().collect()))
}

fn coverage_report_version_dir(
    workspace_root: &Path,
    entry: &AgentRegistryEntry,
    target_version: &str,
) -> PathBuf {
    workspace_root
        .join(&entry.manifest_root)
        .join("reports")
        .join(target_version)
}

fn select_report_path(version_dir: &Path) -> Result<PathBuf, String> {
    for preferred in ["coverage.any.json", "coverage.all.json"] {
        let path = version_dir.join(preferred);
        if path.is_file() {
            return Ok(path);
        }
    }
    let mut candidates = fs::read_dir(version_dir)
        .map_err(|err| format!("read_dir({}): {err}", version_dir.display()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.starts_with("coverage.") && name.ends_with(".json"))
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();
    candidates.sort();
    candidates
        .into_iter()
        .next()
        .ok_or_else(|| format!("no coverage report found under {}", version_dir.display()))
}

fn surface_from_report_value(
    agent_id: &str,
    row: &serde_json::Value,
) -> Result<SurfaceIdentity, String> {
    let object = row
        .as_object()
        .ok_or_else(|| "support-audit report row must be an object".to_string())?;
    let path = object
        .get("path")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "support-audit report row missing `path`".to_string())?
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(ToString::to_string)
                .ok_or_else(|| "support-audit report path value must be a string".to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;

    let key = object.get("key").and_then(serde_json::Value::as_str);
    let name = object.get("name").and_then(serde_json::Value::as_str);
    let command_path = if path.is_empty() {
        agent_id.to_string()
    } else {
        format!("{agent_id} {}", path.join(" "))
    };

    let (surface_kind, surface_id) = if let Some(flag) = key {
        (
            if path.is_empty() {
                "global_flags"
            } else {
                "flags"
            },
            flag.to_string(),
        )
    } else if let Some(arg_name) = name {
        ("positional_args", arg_name.to_string())
    } else {
        let surface_id = path
            .last()
            .cloned()
            .ok_or_else(|| "support-audit command row must not use an empty path".to_string())?;
        (
            if path.len() > 1 {
                "subcommands"
            } else {
                "commands"
            },
            surface_id,
        )
    };

    Ok(SurfaceIdentity::new(
        surface_kind.to_string(),
        command_path,
        surface_id,
    ))
}

fn repo_relative(workspace_root: &Path, path: &Path) -> Result<String, String> {
    path.strip_prefix(workspace_root)
        .map(|relative| relative.to_string_lossy().to_string())
        .map_err(|_| format!("{} is outside workspace root", path.display()))
}
