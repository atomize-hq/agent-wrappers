//! Resolution of an [`AcquisitionDescriptor`] into a concrete, version-pinned acquisition plan.
//!
//! The emitted plan is the whole contract between the committed manifest data and the reusable
//! `parity-acquire` / `parity-promote` workflows: `include` drops straight into a GitHub Actions
//! `strategy.matrix`, and every URL is already resolved. Keeping the resolution here — rather than
//! in workflow `jq`/`case` blocks — is what lets it be unit-tested and shared by every agent.

use serde::Serialize;

use super::descriptor::{
    AcquisitionDescriptor, AcquisitionSourceKind, AcquisitionTarget, ArchiveKind,
};
use super::AcquisitionError;

const NPM_REGISTRY: &str = "https://registry.npmjs.org";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AcquisitionPlan {
    pub plan_schema_version: u32,
    pub agent_id: String,
    pub manifest_root: String,
    pub version: String,
    pub source_kind: &'static str,
    /// Upstream document describing the release; used to derive a deterministic timestamp.
    pub release_metadata_url: String,
    /// Release tag, for `github_releases` only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    pub required_target: String,
    /// Whether the manifest permits promoting a version whose union is incomplete.
    ///
    /// `promotion_policy.allow_promote_when_incomplete` in `RULES.json`; false when unstated.
    pub allow_promote_when_incomplete: bool,
    pub snapshot: PlannedSnapshotCommand,
    pub lockfile_version_key: String,
    pub union_command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wrapper_coverage_command: Option<String>,
    pub validation_commands: Vec<String>,
    pub include: Vec<PlannedTarget>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlannedSnapshotCommand {
    pub command: String,
    pub binary_arg: String,
    pub extra_args: Vec<String>,
    pub env: Vec<PlannedEnvVar>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlannedTarget {
    pub target_triple: String,
    pub runs_on: String,
    pub asset_name: String,
    pub binary_path: String,
    pub archive: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archive_member: Option<String>,
    /// npm platform package this target's binary came from; recorded as lockfile provenance.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform_package: Option<String>,
    pub download_url: String,
    /// Validation environment with `{binary_path}` already resolved for this target.
    ///
    /// `{scratch_dir}` is intentionally left for the caller: it names a directory that only exists
    /// once the job is running.
    pub validation_env: Vec<PlannedEnvVar>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlannedEnvVar {
    pub name: String,
    pub value: String,
}

/// Resolve a validated descriptor against one concrete version.
#[allow(clippy::too_many_arguments)]
pub fn resolve(
    agent_id: &str,
    manifest_root: &str,
    version: &str,
    required_target: &str,
    expected_targets: &[String],
    allow_promote_when_incomplete: bool,
    descriptor: &AcquisitionDescriptor,
) -> Result<AcquisitionPlan, AcquisitionError> {
    let tag = match descriptor.source_kind {
        AcquisitionSourceKind::GithubReleases => {
            let source = descriptor
                .github_releases
                .as_ref()
                .expect("validated descriptor has github_releases");
            Some(source.tag_template.replace("{version}", version))
        }
        AcquisitionSourceKind::Npm => None,
    };

    let release_metadata_url = match descriptor.source_kind {
        AcquisitionSourceKind::GithubReleases => {
            let source = descriptor
                .github_releases
                .as_ref()
                .expect("validated descriptor has github_releases");
            let tag = tag.as_deref().expect("github_releases resolves a tag");
            format!(
                "https://api.github.com/repos/{}/{}/releases/tags/{tag}",
                source.owner, source.repo
            )
        }
        AcquisitionSourceKind::Npm => {
            let source = descriptor
                .npm
                .as_ref()
                .expect("validated descriptor has npm");
            format!("{NPM_REGISTRY}/{}", source.package)
        }
    };

    // Emit in `union.expected_targets` order so the matrix, the union inputs, and the committed
    // pointers all agree on target ordering.
    let mut include = Vec::with_capacity(expected_targets.len());
    for target in expected_targets {
        let spec = descriptor
            .targets
            .get(target)
            .ok_or_else(|| AcquisitionError::Descriptor(format!("unknown target `{target}`")))?;
        include.push(resolve_target(
            target,
            spec,
            version,
            tag.as_deref(),
            descriptor,
        )?);
    }

    Ok(AcquisitionPlan {
        plan_schema_version: 1,
        agent_id: agent_id.to_string(),
        manifest_root: manifest_root.to_string(),
        version: version.to_string(),
        source_kind: descriptor.source_kind.as_str(),
        release_metadata_url,
        tag,
        required_target: required_target.to_string(),
        allow_promote_when_incomplete,
        snapshot: PlannedSnapshotCommand {
            command: descriptor.snapshot.command.clone(),
            binary_arg: descriptor.snapshot.binary_arg.clone(),
            extra_args: descriptor.snapshot.extra_args.clone(),
            env: descriptor
                .snapshot
                .env
                .iter()
                .map(|(name, value)| PlannedEnvVar {
                    name: name.clone(),
                    value: value.clone(),
                })
                .collect(),
        },
        lockfile_version_key: descriptor.lockfile_version_key.clone(),
        union_command: descriptor.union_command.clone(),
        wrapper_coverage_command: descriptor.wrapper_coverage_command.clone(),
        validation_commands: descriptor.validation.commands.clone(),
        include,
    })
}

fn resolve_target(
    target: &str,
    spec: &AcquisitionTarget,
    version: &str,
    tag: Option<&str>,
    descriptor: &AcquisitionDescriptor,
) -> Result<PlannedTarget, AcquisitionError> {
    let binary_path = expand(&spec.binary_path, target, version, tag);

    let (asset_name, download_url) = match descriptor.source_kind {
        AcquisitionSourceKind::GithubReleases => {
            let source = descriptor
                .github_releases
                .as_ref()
                .expect("validated descriptor has github_releases");
            let tag = tag.expect("github_releases resolves a tag");
            let asset = expand(
                spec.asset_name
                    .as_deref()
                    .expect("validated github target has asset_name"),
                target,
                version,
                Some(tag),
            );
            let url = format!(
                "https://github.com/{}/{}/releases/download/{tag}/{asset}",
                source.owner, source.repo
            );
            (asset, url)
        }
        AcquisitionSourceKind::Npm => {
            let package = spec
                .platform_package
                .as_deref()
                .expect("validated npm target has platform_package");
            // npm tarballs are always `<registry>/<pkg>/-/<unscoped-name>-<version>.tgz`, and the
            // platform packages are version-locked to the umbrella package by exact
            // `optionalDependencies` pins, so the umbrella version resolves them directly.
            let unscoped = package.rsplit('/').next().unwrap_or(package);
            let asset = format!("{unscoped}-{version}.tgz");
            let url = format!("{NPM_REGISTRY}/{package}/-/{asset}");
            (asset, url)
        }
    };

    let validation_env = descriptor
        .validation
        .env
        .iter()
        .map(|(name, value)| PlannedEnvVar {
            name: name.clone(),
            value: value.replace("{binary_path}", &binary_path),
        })
        .collect();

    Ok(PlannedTarget {
        target_triple: target.to_string(),
        runs_on: spec.runs_on.clone(),
        asset_name,
        binary_path,
        archive: spec.archive.as_str(),
        archive_member: spec
            .archive_member
            .as_deref()
            .map(|member| expand(member, target, version, tag)),
        platform_package: spec.platform_package.clone(),
        download_url,
        validation_env,
    })
}

/// Substitute the descriptor's supported placeholders.
///
/// Deliberately limited to `{target}`, `{version}` and `{tag}`: an unknown placeholder is left
/// literal rather than guessed at. Note this constrains the *placeholder* vocabulary only — the
/// safety of the surrounding literal text is enforced separately by `safe_relative_path` in
/// `descriptor`.
fn expand(template: &str, target: &str, version: &str, tag: Option<&str>) -> String {
    let mut out = template
        .replace("{target}", target)
        .replace("{version}", version);
    if let Some(tag) = tag {
        out = out.replace("{tag}", tag);
    }
    out
}

impl AcquisitionPlan {
    /// Targets that must be present for a union to be considered complete, in matrix order.
    pub fn target_triples(&self) -> Vec<&str> {
        self.include
            .iter()
            .map(|t| t.target_triple.as_str())
            .collect()
    }

    pub fn uses_npm_tgz(&self) -> bool {
        self.include
            .iter()
            .any(|t| t.archive == ArchiveKind::NpmTgz.as_str())
    }
}
