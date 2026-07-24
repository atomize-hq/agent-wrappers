//! The committed `acquisition` block of a `cli_manifests/<agent>/RULES.json`.
//!
//! This is the single per-agent description of *how to obtain* an upstream release's binaries for
//! every target the union model expects. It deliberately mirrors the shape and validation style of
//! `maintenance.release_watch.upstream` in the agent registry: one `source_kind` discriminant, and
//! source-specific fields that must be present for the selected kind and absent for every other.
//!
//! Watch and acquire stay independent: an agent may watch npm and acquire from GitHub releases, or
//! any other combination. Nothing here re-derives release detection.

use std::collections::BTreeMap;

use serde::Deserialize;

use super::AcquisitionError;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcquisitionDescriptor {
    pub acquisition_schema_version: u32,
    pub source_kind: AcquisitionSourceKind,
    #[serde(default)]
    pub github_releases: Option<GithubReleasesSource>,
    #[serde(default)]
    pub npm: Option<NpmSource>,
    pub snapshot: SnapshotCommand,
    /// Per-row version key used by this agent's committed `artifacts.lock.json`.
    ///
    /// The three shipped lockfiles predate any shared schema (`codex_version`,
    /// `claude_code_version`, `semantic_version`); naming the key here lets one workflow refresh
    /// all of them without rewriting committed artifacts into a new shape.
    pub lockfile_version_key: String,
    pub union_command: String,
    /// Optional: agents without a generated wrapper-coverage manifest keep their committed file.
    #[serde(default)]
    pub wrapper_coverage_command: Option<String>,
    pub targets: BTreeMap<String, AcquisitionTarget>,
    pub validation: ValidationSpec,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AcquisitionSourceKind {
    GithubReleases,
    Npm,
}

impl AcquisitionSourceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::GithubReleases => "github_releases",
            Self::Npm => "npm",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GithubReleasesSource {
    pub owner: String,
    pub repo: String,
    /// Template producing the release tag from a bare semver, e.g. `rust-v{version}`.
    pub tag_template: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NpmSource {
    /// Umbrella package whose packument carries the publish time for a version.
    pub package: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SnapshotCommand {
    /// xtask subcommand that captures this CLI's surface, e.g. `codex-snapshot`.
    pub command: String,
    /// Flag that command uses to receive the binary path, e.g. `--codex-binary`.
    pub binary_arg: String,
    #[serde(default)]
    pub extra_args: Vec<String>,
    /// Environment applied whenever the acquired binary is executed during acquisition.
    ///
    /// This is where an agent disables a self-updater. Without it, a CLI could replace itself
    /// between the pin check and the capture, making the recorded sha256 a lie about the surface
    /// that was actually snapshotted.
    #[serde(default)]
    pub env: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcquisitionTarget {
    /// GitHub Actions runner label capable of executing this target's binary natively.
    pub runs_on: String,
    /// Local path the acquired binary is installed to before snapshotting.
    pub binary_path: String,
    pub archive: ArchiveKind,
    /// Release asset filename. Required for `github_releases`, forbidden for `npm`.
    #[serde(default)]
    pub asset_name: Option<String>,
    /// npm platform package holding this target's binary. Required for `npm`, forbidden otherwise.
    #[serde(default)]
    pub platform_package: Option<String>,
    /// Path of the binary inside the archive. Required for `npm_tgz`.
    #[serde(default)]
    pub archive_member: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveKind {
    /// The downloaded file is the binary itself.
    None,
    /// A gzipped tarball; `archive_member` selects the binary, else the first regular file.
    TarGz,
    /// An npm package tarball; `archive_member` is required.
    NpmTgz,
}

impl ArchiveKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::TarGz => "tar_gz",
            Self::NpmTgz => "npm_tgz",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValidationSpec {
    /// Environment applied to every validation command. Values may use `{binary_path}` (resolved
    /// per target by the planner) and `{scratch_dir}` (resolved by the caller at runtime).
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    pub commands: Vec<String>,
}

impl AcquisitionDescriptor {
    /// Validate everything that does not depend on a concrete version.
    ///
    /// `expected_targets` and `required_target` come from the same file's `union` block: the two
    /// must describe the same target set or acquisition would silently under- or over-build.
    pub fn validate(
        &self,
        expected_targets: &[String],
        required_target: &str,
    ) -> Result<(), AcquisitionError> {
        if self.acquisition_schema_version != 1 {
            return Err(AcquisitionError::Descriptor(format!(
                "acquisition.acquisition_schema_version must be 1 (got {})",
                self.acquisition_schema_version
            )));
        }

        self.validate_source_config()?;
        self.validate_target_set(expected_targets, required_target)?;
        for (target, spec) in &self.targets {
            self.validate_target(target, spec)?;
        }
        self.validate_commands()?;
        Ok(())
    }

    fn validate_source_config(&self) -> Result<(), AcquisitionError> {
        match self.source_kind {
            AcquisitionSourceKind::GithubReleases => {
                let source = self.github_releases.as_ref().ok_or_else(|| {
                    AcquisitionError::Descriptor(
                        "acquisition.github_releases is required when source_kind = `github_releases`"
                            .to_string(),
                    )
                })?;
                safe_relative_path("acquisition.github_releases.owner", &source.owner)?;
                safe_relative_path("acquisition.github_releases.repo", &source.repo)?;
                non_empty(
                    "acquisition.github_releases.tag_template",
                    &source.tag_template,
                )?;
                if !source.tag_template.contains("{version}") {
                    return Err(AcquisitionError::Descriptor(
                        "acquisition.github_releases.tag_template must contain `{version}`"
                            .to_string(),
                    ));
                }
                forbid("acquisition.npm", self.npm.is_some(), self.source_kind)?;
            }
            AcquisitionSourceKind::Npm => {
                let source = self.npm.as_ref().ok_or_else(|| {
                    AcquisitionError::Descriptor(
                        "acquisition.npm is required when source_kind = `npm`".to_string(),
                    )
                })?;
                safe_relative_path("acquisition.npm.package", &source.package)?;
                forbid(
                    "acquisition.github_releases",
                    self.github_releases.is_some(),
                    self.source_kind,
                )?;
            }
        }
        Ok(())
    }

    fn validate_target_set(
        &self,
        expected_targets: &[String],
        required_target: &str,
    ) -> Result<(), AcquisitionError> {
        let missing: Vec<&String> = expected_targets
            .iter()
            .filter(|t| !self.targets.contains_key(*t))
            .collect();
        if !missing.is_empty() {
            return Err(AcquisitionError::Descriptor(format!(
                "acquisition.targets is missing union.expected_targets entries: {}",
                join(&missing)
            )));
        }

        let extra: Vec<&String> = self
            .targets
            .keys()
            .filter(|t| !expected_targets.contains(t))
            .collect();
        if !extra.is_empty() {
            return Err(AcquisitionError::Descriptor(format!(
                "acquisition.targets declares targets absent from union.expected_targets: {}",
                join(&extra)
            )));
        }

        if !self.targets.contains_key(required_target) {
            return Err(AcquisitionError::Descriptor(format!(
                "acquisition.targets is missing union.required_target `{required_target}`"
            )));
        }
        Ok(())
    }

    fn validate_target(
        &self,
        target: &str,
        spec: &AcquisitionTarget,
    ) -> Result<(), AcquisitionError> {
        safe_runner_label(
            &format!("acquisition.targets.{target}.runs_on"),
            &spec.runs_on,
        )?;
        safe_relative_path(
            &format!("acquisition.targets.{target}.binary_path"),
            &spec.binary_path,
        )?;
        if let Some(member) = spec.archive_member.as_deref() {
            safe_relative_path(
                &format!("acquisition.targets.{target}.archive_member"),
                member,
            )?;
        }

        match self.source_kind {
            AcquisitionSourceKind::GithubReleases => {
                let asset = spec.asset_name.as_deref().ok_or_else(|| {
                    AcquisitionError::Descriptor(format!(
                        "acquisition.targets.{target}.asset_name is required for source_kind = `github_releases`"
                    ))
                })?;
                safe_relative_path(&format!("acquisition.targets.{target}.asset_name"), asset)?;
                forbid(
                    &format!("acquisition.targets.{target}.platform_package"),
                    spec.platform_package.is_some(),
                    self.source_kind,
                )?;
                if spec.archive == ArchiveKind::NpmTgz {
                    return Err(AcquisitionError::Descriptor(format!(
                        "acquisition.targets.{target}.archive = `npm_tgz` is only valid for source_kind = `npm`"
                    )));
                }
            }
            AcquisitionSourceKind::Npm => {
                let package = spec.platform_package.as_deref().ok_or_else(|| {
                    AcquisitionError::Descriptor(format!(
                        "acquisition.targets.{target}.platform_package is required for source_kind = `npm`"
                    ))
                })?;
                safe_relative_path(
                    &format!("acquisition.targets.{target}.platform_package"),
                    package,
                )?;
                forbid(
                    &format!("acquisition.targets.{target}.asset_name"),
                    spec.asset_name.is_some(),
                    self.source_kind,
                )?;
                if spec.archive != ArchiveKind::NpmTgz {
                    return Err(AcquisitionError::Descriptor(format!(
                        "acquisition.targets.{target}.archive must be `npm_tgz` for source_kind = `npm` (got `{}`)",
                        spec.archive.as_str()
                    )));
                }
            }
        }

        if spec.archive == ArchiveKind::NpmTgz && spec.archive_member.is_none() {
            return Err(AcquisitionError::Descriptor(format!(
                "acquisition.targets.{target}.archive_member is required when archive = `npm_tgz`"
            )));
        }
        if spec.archive == ArchiveKind::None && spec.archive_member.is_some() {
            return Err(AcquisitionError::Descriptor(format!(
                "acquisition.targets.{target}.archive_member must be omitted when archive = `none`"
            )));
        }
        Ok(())
    }

    fn validate_commands(&self) -> Result<(), AcquisitionError> {
        non_empty("acquisition.snapshot.command", &self.snapshot.command)?;
        non_empty("acquisition.snapshot.binary_arg", &self.snapshot.binary_arg)?;
        non_empty("acquisition.union_command", &self.union_command)?;
        non_empty(
            "acquisition.lockfile_version_key",
            &self.lockfile_version_key,
        )?;
        if let Some(cmd) = self.wrapper_coverage_command.as_deref() {
            non_empty("acquisition.wrapper_coverage_command", cmd)?;
        }
        if self.validation.commands.is_empty() {
            return Err(AcquisitionError::Descriptor(
                "acquisition.validation.commands must not be empty".to_string(),
            ));
        }
        for (idx, command) in self.validation.commands.iter().enumerate() {
            non_empty(&format!("acquisition.validation.commands[{idx}]"), command)?;
        }
        Ok(())
    }
}

/// Constrain runner labels to the shape GitHub-hosted and self-hosted labels actually take.
///
/// `runs_on` is interpolated into the workflow's job definition, so it should not be able to
/// carry arbitrary text out of a manifest edit.
fn safe_runner_label(field: &str, value: &str) -> Result<(), AcquisitionError> {
    non_empty(field, value)?;
    let ok = value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'));
    if !ok {
        return Err(AcquisitionError::Descriptor(format!(
            "{field} must be a plain runner label (alphanumerics, `-`, `_`, `.`); got `{value}`"
        )));
    }
    Ok(())
}

/// Reject path-shaped values that could escape the runner workspace.
///
/// These fields are interpolated into `curl -o`, `tar -x` and file copies on the runner. The
/// threat model is committed, reviewed manifest data rather than attacker input, so this is
/// defense in depth — but a typo'd `../` is just as destructive as a malicious one.
fn safe_relative_path(field: &str, value: &str) -> Result<(), AcquisitionError> {
    non_empty(field, value)?;
    let normalized = value.strip_prefix("./").unwrap_or(value);
    let bad = normalized.starts_with('/')
        || normalized.contains('\\')
        || normalized.split('/').any(|part| part == "..")
        || normalized.contains('\0');
    if bad {
        return Err(AcquisitionError::Descriptor(format!(
            "{field} must be a workspace-relative path without `..`, a leading `/`, or backslashes (got `{value}`)"
        )));
    }
    Ok(())
}

fn non_empty(field: &str, value: &str) -> Result<(), AcquisitionError> {
    if value.trim().is_empty() {
        return Err(AcquisitionError::Descriptor(format!(
            "{field} must be a non-empty string"
        )));
    }
    Ok(())
}

fn forbid(
    field: &str,
    present: bool,
    source_kind: AcquisitionSourceKind,
) -> Result<(), AcquisitionError> {
    if present {
        return Err(AcquisitionError::Descriptor(format!(
            "{field} must not be set when acquisition.source_kind = `{}`",
            source_kind.as_str()
        )));
    }
    Ok(())
}

fn join(values: &[&String]) -> String {
    values
        .iter()
        .map(|v| v.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}
