use std::collections::BTreeMap;

use crate::{
    agent_registry::{
        ReleaseWatchDispatchKind, ReleaseWatchMetadata, ReleaseWatchSourceKind,
        ReleaseWatchVersionPolicy,
    },
    approval_artifact,
    capability_projection::requires_explicit_publication_target,
};

use super::{
    ApprovalMaintenanceModeSummary, ApprovalMaintenanceSummary, ApprovalProvenance, Args,
    ConfigGate, DraftDescriptorInput, Error, TargetGate,
};

impl DraftDescriptorInput {
    pub(super) fn from_raw_args(args: Args) -> Result<Self, Error> {
        Ok(Self {
            agent_id: required_arg(args.agent_id, "--agent-id")?,
            display_name: required_arg(args.display_name, "--display-name")?,
            crate_path: required_arg(args.crate_path, "--crate-path")?,
            backend_module: required_arg(args.backend_module, "--backend-module")?,
            manifest_root: required_arg(args.manifest_root, "--manifest-root")?,
            package_name: required_arg(args.package_name, "--package-name")?,
            canonical_targets: args.canonical_targets,
            wrapper_coverage_binding_kind: required_arg(
                args.wrapper_coverage_binding_kind,
                "--wrapper-coverage-binding-kind",
            )?,
            wrapper_coverage_source_path: required_arg(
                args.wrapper_coverage_source_path,
                "--wrapper-coverage-source-path",
            )?,
            always_on_capabilities: args.always_on_capabilities,
            target_gated_capabilities: args.target_gated_capabilities,
            config_gated_capabilities: args.config_gated_capabilities,
            backend_extensions: args.backend_extensions,
            support_matrix_enabled: required_bool(
                args.support_matrix_enabled,
                "--support-matrix-enabled",
            )?,
            capability_matrix_enabled: required_bool(
                args.capability_matrix_enabled,
                "--capability-matrix-enabled",
            )?,
            capability_matrix_target: args.capability_matrix_target,
            docs_release_track: required_arg(args.docs_release_track, "--docs-release-track")?,
            onboarding_pack_prefix: required_arg(
                args.onboarding_pack_prefix,
                "--onboarding-pack-prefix",
            )?,
            approval_provenance: None,
            approval_maintenance: None,
        })
    }
}

impl From<approval_artifact::ApprovalArtifact> for DraftDescriptorInput {
    fn from(artifact: approval_artifact::ApprovalArtifact) -> Self {
        let approval_artifact::ApprovalArtifact {
            relative_path,
            sha256,
            descriptor,
            maintenance,
            ..
        } = artifact;
        Self {
            agent_id: descriptor.agent_id,
            display_name: descriptor.display_name,
            crate_path: descriptor.crate_path,
            backend_module: descriptor.backend_module,
            manifest_root: descriptor.manifest_root,
            package_name: descriptor.package_name,
            canonical_targets: descriptor.canonical_targets,
            wrapper_coverage_binding_kind: descriptor.wrapper_coverage_binding_kind,
            wrapper_coverage_source_path: descriptor.wrapper_coverage_source_path,
            always_on_capabilities: descriptor.always_on_capabilities,
            target_gated_capabilities: descriptor.target_gated_capabilities,
            config_gated_capabilities: descriptor.config_gated_capabilities,
            backend_extensions: descriptor.backend_extensions,
            support_matrix_enabled: descriptor.support_matrix_enabled,
            capability_matrix_enabled: descriptor.capability_matrix_enabled,
            capability_matrix_target: descriptor.capability_matrix_target,
            docs_release_track: descriptor.docs_release_track,
            onboarding_pack_prefix: descriptor.onboarding_pack_prefix,
            approval_provenance: Some(ApprovalProvenance {
                artifact_path: relative_path,
                artifact_sha256: sha256,
                approval_recorded_at: String::new(),
            }),
            approval_maintenance: Some(maintenance.into()),
        }
    }
}

impl From<approval_artifact::ApprovalMaintenance> for ApprovalMaintenanceSummary {
    fn from(maintenance: approval_artifact::ApprovalMaintenance) -> Self {
        let section_sha256 = maintenance.section_sha256;
        let mode = match maintenance.mode {
            approval_artifact::ApprovalMaintenanceMode::ReleaseWatchEnrolled {
                release_watch,
                release_watch_sha256,
            } => {
                let version_policy = release_watch_version_policy(&release_watch);
                let dispatch_kind = release_watch_dispatch_kind(&release_watch);
                let upstream = release_watch_upstream(&release_watch);
                let dispatch_workflow = release_watch.dispatch_workflow;
                ApprovalMaintenanceModeSummary::ReleaseWatchEnrolled {
                    version_policy,
                    dispatch_kind,
                    dispatch_workflow,
                    upstream,
                    release_watch_sha256,
                }
            }
            approval_artifact::ApprovalMaintenanceMode::ExplicitlyDeferred {
                deferral,
                deferral_sha256,
            } => ApprovalMaintenanceModeSummary::ExplicitlyDeferred {
                reason: deferral.reason,
                follow_up: deferral.follow_up,
                approved_scope: deferral.approved_scope,
                deferral_sha256,
            },
        };
        Self {
            section_sha256,
            mode,
        }
    }
}

pub(super) fn normalize_ordered_unique(
    values: Vec<String>,
    flag_name: &str,
    require_non_empty: bool,
) -> Result<Vec<String>, Error> {
    if require_non_empty && values.is_empty() {
        return Err(Error::Validation(format!(
            "{flag_name} must be provided at least once"
        )));
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut out = Vec::with_capacity(values.len());
    for value in values {
        let trimmed = value.trim().to_string();
        if trimmed.is_empty() {
            return Err(Error::Validation(format!(
                "{flag_name} must not contain empty values"
            )));
        }
        if !seen.insert(trimmed.clone()) {
            return Err(Error::Validation(format!(
                "{flag_name} contains duplicate value `{trimmed}`"
            )));
        }
        out.push(trimmed);
    }
    Ok(out)
}

pub(super) fn normalize_sorted_unique(
    values: Vec<String>,
    flag_name: &str,
) -> Result<Vec<String>, Error> {
    let mut out = normalize_ordered_unique(values, flag_name, false)?;
    out.sort();
    Ok(out)
}

pub(super) fn parse_target_gates(
    values: Vec<String>,
    flag_name: &str,
    canonical_index: &BTreeMap<String, usize>,
) -> Result<Vec<TargetGate>, Error> {
    let mut seen = std::collections::BTreeSet::new();
    let mut out = Vec::with_capacity(values.len());
    for value in values {
        let (capability_id, raw_targets) = value.split_once(':').ok_or_else(|| {
            Error::Validation(format!(
                "{flag_name} must be formatted as <capability-id>:<target>[,<target>...] (got `{value}`)"
            ))
        })?;
        let capability_id = validate_gate_scalar(capability_id, flag_name, &value)?;
        let mut targets = parse_gate_targets(raw_targets, flag_name, &value, canonical_index)?;
        if !seen.insert((capability_id.clone(), targets.clone())) {
            return Err(Error::Validation(format!(
                "{flag_name} contains duplicate entry `{value}`"
            )));
        }
        targets.sort_by_key(|target| canonical_index[target]);
        out.push(TargetGate {
            capability_id,
            targets,
        });
    }
    Ok(out)
}

pub(super) fn parse_config_gates(
    values: Vec<String>,
    flag_name: &str,
    canonical_index: &BTreeMap<String, usize>,
) -> Result<Vec<ConfigGate>, Error> {
    let mut seen = std::collections::BTreeSet::new();
    let mut out = Vec::with_capacity(values.len());
    for value in values {
        let mut parts = value.splitn(3, ':');
        let capability_id =
            validate_gate_scalar(parts.next().unwrap_or_default(), flag_name, &value)?;
        let config_key = validate_gate_scalar(parts.next().unwrap_or_default(), flag_name, &value)?;
        let targets = match parts.next() {
            Some(raw_targets) => {
                let mut targets =
                    parse_gate_targets(raw_targets, flag_name, &value, canonical_index)?;
                targets.sort_by_key(|target| canonical_index[target]);
                Some(targets)
            }
            None => None,
        };
        let signature = format!(
            "{}:{}:{}",
            capability_id,
            config_key,
            targets
                .as_ref()
                .map(|targets| targets.join(","))
                .unwrap_or_default()
        );
        if !seen.insert(signature) {
            return Err(Error::Validation(format!(
                "{flag_name} contains duplicate entry `{value}`"
            )));
        }
        out.push(ConfigGate {
            capability_id,
            config_key,
            targets,
        });
    }
    Ok(out)
}

pub(super) fn normalize_capability_matrix_target(
    value: Option<String>,
    canonical_targets: &[String],
    canonical_index: &BTreeMap<String, usize>,
    capability_matrix_enabled: bool,
    target_gated_capabilities: &[TargetGate],
    config_gated_capabilities: &[ConfigGate],
    allow_legacy_single_target_fallback: bool,
) -> Result<Option<String>, Error> {
    let value = value
        .map(|target| validate_gate_scalar(&target, "--capability-matrix-target", &target))
        .transpose()?;

    if let Some(target) = value.as_deref() {
        if !canonical_index.contains_key(target) {
            return Err(Error::Validation(format!(
                "--capability-matrix-target `{target}` is not present in --canonical-target"
            )));
        }
    }

    let explicit_required = requires_explicit_publication_target(
        capability_matrix_enabled,
        !target_gated_capabilities.is_empty(),
        config_gated_capabilities.iter().any(|gate| {
            gate.targets
                .as_ref()
                .is_some_and(|targets| !targets.is_empty())
        }),
    );

    if explicit_required && value.is_none() {
        if allow_legacy_single_target_fallback && canonical_targets.len() == 1 {
            return Ok(Some(canonical_targets[0].clone()));
        }

        return Err(Error::Validation(
            "--capability-matrix-target is required when capability-matrix publication uses target-scoped declarations".to_string(),
        ));
    }

    Ok(value)
}

pub(super) fn canonical_index(canonical_targets: &[String]) -> BTreeMap<String, usize> {
    canonical_targets
        .iter()
        .enumerate()
        .map(|(index, target)| (target.clone(), index))
        .collect()
}

pub(super) fn validate_gate_scalar(
    value: &str,
    flag_name: &str,
    original: &str,
) -> Result<String, Error> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(Error::Validation(format!(
            "{flag_name} contains an empty field in `{original}`"
        )));
    }
    Ok(trimmed.to_string())
}

fn required_arg(value: Option<String>, flag_name: &str) -> Result<String, Error> {
    let value = value.ok_or_else(|| {
        Error::Validation(format!(
            "{flag_name} must be provided when --approval is not used"
        ))
    })?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(Error::Validation(format!(
            "{flag_name} must not be empty when --approval is not used"
        )));
    }
    Ok(trimmed.to_string())
}

fn release_watch_version_policy(release_watch: &ReleaseWatchMetadata) -> &'static str {
    match release_watch.version_policy {
        ReleaseWatchVersionPolicy::LatestStableMinusOne => "latest_stable_minus_one",
        ReleaseWatchVersionPolicy::UpstreamStablePointer => "upstream_stable_pointer",
    }
}

fn release_watch_dispatch_kind(release_watch: &ReleaseWatchMetadata) -> &'static str {
    match release_watch.dispatch_kind {
        ReleaseWatchDispatchKind::WorkflowDispatch => "workflow_dispatch",
        ReleaseWatchDispatchKind::PacketPr => "packet_pr",
    }
}

fn release_watch_upstream(release_watch: &ReleaseWatchMetadata) -> String {
    match release_watch.upstream.source_kind {
        ReleaseWatchSourceKind::GithubReleases => format!(
            "github_releases:{}/{}{}",
            release_watch.upstream.owner.as_deref().unwrap_or_default(),
            release_watch.upstream.repo.as_deref().unwrap_or_default(),
            release_watch
                .upstream
                .tag_prefix
                .as_deref()
                .map(|tag_prefix| format!(" (tag_prefix: {tag_prefix})"))
                .unwrap_or_default()
        ),
        ReleaseWatchSourceKind::GcsObjectListing => {
            let bucket = release_watch.upstream.bucket.as_deref().unwrap_or_default();
            let prefix = release_watch.upstream.prefix.as_deref().unwrap_or_default();
            let marker = release_watch
                .upstream
                .version_marker
                .as_deref()
                .map(|marker| format!(" (version_marker: {marker})"))
                .unwrap_or_default();
            format!("gcs_object_listing:{bucket}/{prefix}{marker}")
        }
        ReleaseWatchSourceKind::NpmDistTag => {
            let package = release_watch
                .upstream
                .package
                .as_deref()
                .unwrap_or_default();
            let dist_tag = release_watch
                .upstream
                .dist_tag
                .as_deref()
                .unwrap_or_default();
            format!("npm_dist_tag:{package}#{dist_tag}")
        }
    }
}

fn required_bool(value: Option<bool>, flag_name: &str) -> Result<bool, Error> {
    value.ok_or_else(|| {
        Error::Validation(format!(
            "{flag_name} must be provided when --approval is not used"
        ))
    })
}

fn parse_gate_targets(
    raw_targets: &str,
    flag_name: &str,
    original: &str,
    canonical_index: &BTreeMap<String, usize>,
) -> Result<Vec<String>, Error> {
    let targets = raw_targets
        .split(',')
        .map(str::trim)
        .filter(|target| !target.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    if targets.is_empty() {
        return Err(Error::Validation(format!(
            "{flag_name} must declare at least one target in `{original}`"
        )));
    }

    let mut seen = std::collections::BTreeSet::new();
    for target in &targets {
        if !seen.insert(target.clone()) {
            return Err(Error::Validation(format!(
                "{flag_name} contains duplicate target `{target}` in `{original}`"
            )));
        }
        if !canonical_index.contains_key(target) {
            return Err(Error::Validation(format!(
                "{flag_name} target `{target}` is not present in --canonical-target"
            )));
        }
    }
    Ok(targets)
}
