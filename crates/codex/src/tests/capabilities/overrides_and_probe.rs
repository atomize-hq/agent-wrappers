use super::*;

#[tokio::test]
async fn capability_snapshot_short_circuits_probes() {
    let _guard = env_guard_async().await;
    clear_capability_cache();

    let temp = tempfile::tempdir().unwrap();
    let log_path = temp.path().join("probe.log");
    let script = format!(
        r#"#!/bin/bash
echo "$@" >> "{log}"
exit 99
"#,
        log = log_path.display()
    );
    let binary = write_fake_codex(temp.path(), &script);

    let snapshot = CodexCapabilities {
        cache_key: CapabilityCacheKey {
            binary_path: PathBuf::from("codex"),
        },
        fingerprint: None,
        version: Some(version::parse_version_output("codex 9.9.9-custom")),
        features: CodexFeatureFlags {
            supports_features_list: true,
            supports_output_schema: true,
            supports_add_dir: false,
            supports_mcp_login: true,
        },
        probe_plan: CapabilityProbePlan::default(),
        collected_at: SystemTime::now(),
    };

    let client = CodexClient::builder()
        .binary(&binary)
        .capability_snapshot(snapshot)
        .timeout(Duration::from_secs(5))
        .build();

    let capabilities = client.probe_capabilities().await;
    assert_eq!(
        capabilities.cache_key.binary_path,
        std_fs::canonicalize(&binary).unwrap()
    );
    assert!(capabilities.fingerprint.is_some());
    assert!(capabilities.features.supports_output_schema);
    assert!(capabilities.features.supports_mcp_login);
    assert_eq!(
        capabilities.version.as_ref().and_then(|v| v.semantic),
        Some((9, 9, 9))
    );
    assert!(capabilities
        .probe_plan
        .steps
        .contains(&CapabilityProbeStep::ManualOverride));
    assert!(!log_path.exists());
}

#[tokio::test]
async fn capability_feature_overrides_apply_to_cached_entries() {
    let _guard = env_guard_async().await;
    clear_capability_cache();

    let temp = tempfile::tempdir().unwrap();
    let script = r#"#!/bin/bash
if [[ "$1" == "--version" ]]; then
  echo "codex 1.0.0"
elif [[ "$1" == "features" && "$2" == "list" && "$3" == "--json" ]]; then
  echo '{"features":[]}'
elif [[ "$1" == "features" && "$2" == "list" ]]; then
  echo "features list"
elif [[ "$1" == "--help" ]]; then
  echo "Usage: codex exec"
fi
"#;
    let binary = write_fake_codex(temp.path(), script);

    let base_client = CodexClient::builder()
        .binary(&binary)
        .timeout(Duration::from_secs(5))
        .build();
    let base_capabilities = base_client.probe_capabilities().await;
    assert!(base_capabilities.features.supports_features_list);
    assert!(!base_capabilities.features.supports_output_schema);

    let overrides = CapabilityFeatureOverrides::enabling(CodexFeatureFlags {
        supports_features_list: false,
        supports_output_schema: true,
        supports_add_dir: false,
        supports_mcp_login: true,
    });

    let client = CodexClient::builder()
        .binary(&binary)
        .capability_feature_overrides(overrides)
        .timeout(Duration::from_secs(5))
        .build();

    let capabilities = client.probe_capabilities().await;
    assert!(capabilities.features.supports_output_schema);
    assert!(capabilities.features.supports_mcp_login);
    assert!(capabilities
        .probe_plan
        .steps
        .contains(&CapabilityProbeStep::ManualOverride));
    assert_eq!(
        capabilities.guard_output_schema().support,
        CapabilitySupport::Supported
    );
}

#[tokio::test]
async fn capability_version_override_replaces_probe_version() {
    let _guard = env_guard_async().await;
    clear_capability_cache();

    let temp = tempfile::tempdir().unwrap();
    let script = r#"#!/bin/bash
if [[ "$1" == "--version" ]]; then
  echo "codex 0.1.0"
elif [[ "$1" == "features" && "$2" == "list" && "$3" == "--json" ]]; then
  echo '{"features":["add_dir"]}'
elif [[ "$1" == "features" && "$2" == "list" ]]; then
  echo "add_dir"
elif [[ "$1" == "--help" ]]; then
  echo "Usage: codex add-dir"
fi
	"#;
    let binary = write_fake_codex(temp.path(), script);
    let version_override = version::parse_version_output("codex 9.9.9-nightly (commit beefcafe)");

    let client = CodexClient::builder()
        .binary(&binary)
        .timeout(Duration::from_secs(5))
        .capability_version_override(version_override)
        .build();

    let capabilities = client.probe_capabilities().await;
    assert_eq!(
        capabilities.version.as_ref().and_then(|v| v.semantic),
        Some((9, 9, 9))
    );
    assert!(matches!(
        capabilities.version.as_ref().map(|v| v.channel),
        Some(CodexReleaseChannel::Nightly)
    ));
    assert!(capabilities.features.supports_add_dir);
    assert!(capabilities
        .probe_plan
        .steps
        .contains(&CapabilityProbeStep::ManualOverride));
    assert_eq!(
        capabilities.guard_add_dir().support,
        CapabilitySupport::Supported
    );
}
