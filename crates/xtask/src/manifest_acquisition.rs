//! Agent-agnostic multi-target parity acquisition planning.
//!
//! One reusable acquisition workflow needs one machine-readable answer to "for agent X at version
//! V, which targets do I build, on which runners, from which URLs, with which snapshot command?".
//! This module is that answer. It reads committed truth only — the agent registry for enrollment
//! and manifest root, and `<root>/RULES.json` for the union target model and the `acquisition`
//! descriptor — and emits a resolved plan. No release detection, no version selection.

use std::path::PathBuf;
use std::{fs, io};

use clap::Parser;
use serde::Deserialize;
use thiserror::Error;

use crate::agent_registry::{AgentRegistry, AgentRegistryError};

pub mod descriptor;
pub mod plan;

pub use descriptor::AcquisitionDescriptor;
pub use plan::{resolve, AcquisitionPlan};

#[derive(Debug, Parser)]
pub struct Args {
    /// Registry agent id, e.g. `codex`. Resolves the manifest root and the enrollment gate.
    #[arg(long)]
    pub agent: String,

    /// Bare upstream semantic version to plan for, e.g. `0.145.0`.
    #[arg(long)]
    pub version: String,

    /// Write the resolved plan JSON here instead of stdout.
    #[arg(long)]
    pub emit_json: Option<PathBuf>,

    /// Workspace root containing `crates/xtask/data/agent_registry.toml` (default: cwd).
    #[arg(long)]
    pub workspace_root: Option<PathBuf>,
}

#[derive(Debug, Error)]
pub enum AcquisitionError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("failed to parse JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("agent registry error: {0}")]
    Registry(#[from] AgentRegistryError),
    #[error("unknown agent `{0}` (not present in the committed agent registry)")]
    UnknownAgent(String),
    #[error(
        "agent `{0}` is not enrolled in maintenance.release_watch; multi-target acquisition is \
         enabled only for release-watch-enrolled agents that carry an `acquisition` block"
    )]
    NotEnrolled(String),
    #[error(
        "agent `{agent}` has no `acquisition` block in {path}; add one to enable multi-target \
         acquisition, or leave it absent to keep the agent on the docs-only maintenance path"
    )]
    NoAcquisitionBlock { agent: String, path: String },
    #[error("invalid acquisition descriptor: {0}")]
    Descriptor(String),
    #[error("invalid version `{0}`: expected a bare semantic version such as 1.2.3")]
    InvalidVersion(String),
}

/// Exit code meaning "this agent is simply not on the acquisition lane".
///
/// Callers use the planner as a gate. Collapsing an expected gate miss and a genuine regression
/// (a malformed descriptor, a broken registry) into one non-zero code would let a real defect
/// silently downgrade an enrolled agent to the docs-only path.
pub const EXIT_NOT_ELIGIBLE: i32 = 3;

impl AcquisitionError {
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::UnknownAgent(_) | Self::NotEnrolled(_) | Self::NoAcquisitionBlock { .. } => {
                EXIT_NOT_ELIGIBLE
            }
            _ => 1,
        }
    }
}

/// Minimal view of `RULES.json`: the union target model plus the optional acquisition descriptor.
#[derive(Debug, Deserialize)]
struct RulesFile {
    union: RulesUnion,
    #[serde(default)]
    acquisition: Option<AcquisitionDescriptor>,
}

#[derive(Debug, Deserialize)]
struct RulesUnion {
    required_target: String,
    expected_targets: Vec<String>,
    #[serde(default)]
    promotion_policy: Option<RulesPromotionPolicy>,
}

#[derive(Debug, Deserialize)]
struct RulesPromotionPolicy {
    /// Whether this agent permits promoting a version whose union is not complete.
    ///
    /// Absent policy means absent permission: an agent that has not declared its stance does not
    /// get one by default.
    #[serde(default)]
    allow_promote_when_incomplete: bool,
}

pub fn run(args: Args) -> Result<(), AcquisitionError> {
    let workspace_root = match args.workspace_root {
        Some(root) => root,
        None => std::env::current_dir()?,
    };

    let plan = plan_for_agent(&workspace_root, &args.agent, &args.version)?;
    let rendered = format!("{}\n", serde_json::to_string_pretty(&plan)?);

    match args.emit_json {
        Some(path) => {
            if let Some(parent) = path.parent() {
                if !parent.as_os_str().is_empty() {
                    fs::create_dir_all(parent)?;
                }
            }
            fs::write(path, rendered)?;
        }
        None => print!("{rendered}"),
    }
    Ok(())
}

/// Build the acquisition plan for one agent at one version, enforcing the support-tier gate.
///
/// The gate is deliberately data-driven rather than a hardcoded agent list: an agent participates
/// in multi-target acquisition exactly when it is enrolled in `maintenance.release_watch` **and**
/// its manifest carries an `acquisition` descriptor. Docs-only agents satisfy neither and keep
/// their existing single-host maintenance behavior untouched.
pub fn plan_for_agent(
    workspace_root: &std::path::Path,
    agent_id: &str,
    version: &str,
) -> Result<AcquisitionPlan, AcquisitionError> {
    validate_bare_semver(version)?;

    let registry = AgentRegistry::load(workspace_root)?;
    let entry = registry
        .find(agent_id)
        .ok_or_else(|| AcquisitionError::UnknownAgent(agent_id.to_string()))?;

    if entry.maintenance.release_watch.is_none() {
        return Err(AcquisitionError::NotEnrolled(agent_id.to_string()));
    }

    let manifest_root = entry.manifest_root.clone();
    let rules_path = workspace_root.join(&manifest_root).join("RULES.json");
    let rules: RulesFile = serde_json::from_slice(&fs::read(&rules_path)?)?;

    let descriptor = rules
        .acquisition
        .ok_or_else(|| AcquisitionError::NoAcquisitionBlock {
            agent: agent_id.to_string(),
            path: rules_path.display().to_string(),
        })?;

    descriptor.validate(&rules.union.expected_targets, &rules.union.required_target)?;

    resolve(
        agent_id,
        &manifest_root,
        version,
        &rules.union.required_target,
        &rules.union.expected_targets,
        rules
            .union
            .promotion_policy
            .map(|policy| policy.allow_promote_when_incomplete)
            .unwrap_or(false),
        &descriptor,
    )
}

/// Reject anything that is not a bare `MAJOR.MINOR.PATCH`.
///
/// Acquisition only ever runs against a stable upstream release the watcher already selected;
/// accepting prerelease or `v`-prefixed input here would let a malformed packet reach the runner.
fn validate_bare_semver(version: &str) -> Result<(), AcquisitionError> {
    let mut parts = version.split('.');
    let ok = matches!(
        (parts.next(), parts.next(), parts.next(), parts.next()),
        (Some(a), Some(b), Some(c), None)
            if [a, b, c].iter().all(|p| {
                !p.is_empty() && p.chars().all(|ch| ch.is_ascii_digit())
            })
    );
    if ok {
        Ok(())
    } else {
        Err(AcquisitionError::InvalidVersion(version.to_string()))
    }
}
