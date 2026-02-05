use super::*;

fn capabilities_with_version(raw_version: &str) -> CodexCapabilities {
    CodexCapabilities {
        cache_key: CapabilityCacheKey {
            binary_path: PathBuf::from("codex"),
        },
        fingerprint: None,
        version: Some(version::parse_version_output(raw_version)),
        features: CodexFeatureFlags::default(),
        probe_plan: CapabilityProbePlan::default(),
        collected_at: SystemTime::now(),
    }
}

fn capabilities_without_version() -> CodexCapabilities {
    CodexCapabilities {
        cache_key: CapabilityCacheKey {
            binary_path: PathBuf::from("codex"),
        },
        fingerprint: None,
        version: None,
        features: CodexFeatureFlags::default(),
        probe_plan: CapabilityProbePlan::default(),
        collected_at: SystemTime::now(),
    }
}

fn capabilities_with_feature_flags(features: CodexFeatureFlags) -> CodexCapabilities {
    CodexCapabilities {
        cache_key: CapabilityCacheKey {
            binary_path: PathBuf::from("codex"),
        },
        fingerprint: None,
        version: None,
        features,
        probe_plan: CapabilityProbePlan::default(),
        collected_at: SystemTime::now(),
    }
}

fn sample_capabilities_snapshot() -> CodexCapabilities {
    CodexCapabilities {
        cache_key: CapabilityCacheKey {
            binary_path: PathBuf::from("/tmp/codex"),
        },
        fingerprint: Some(BinaryFingerprint {
            canonical_path: Some(PathBuf::from("/tmp/codex")),
            modified: Some(SystemTime::UNIX_EPOCH + Duration::from_secs(5)),
            len: Some(1234),
        }),
        version: Some(CodexVersionInfo {
            raw: "codex 3.4.5-beta (commit cafe)".to_string(),
            semantic: Some((3, 4, 5)),
            commit: Some("cafe".to_string()),
            channel: CodexReleaseChannel::Beta,
        }),
        features: CodexFeatureFlags {
            supports_features_list: true,
            supports_output_schema: true,
            supports_add_dir: false,
            supports_mcp_login: true,
        },
        probe_plan: CapabilityProbePlan {
            steps: vec![
                CapabilityProbeStep::VersionFlag,
                CapabilityProbeStep::FeaturesListJson,
                CapabilityProbeStep::ManualOverride,
            ],
        },
        collected_at: SystemTime::UNIX_EPOCH + Duration::from_secs(10),
    }
}

fn sample_capability_overrides() -> CapabilityOverrides {
    CapabilityOverrides {
        snapshot: Some(sample_capabilities_snapshot()),
        version: Some(version::parse_version_output("codex 9.9.9-nightly")),
        features: CapabilityFeatureOverrides {
            supports_features_list: Some(true),
            supports_output_schema: Some(true),
            supports_add_dir: Some(true),
            supports_mcp_login: None,
        },
    }
}

fn capability_snapshot_with_metadata(
    collected_at: SystemTime,
    fingerprint: Option<BinaryFingerprint>,
) -> CodexCapabilities {
    CodexCapabilities {
        cache_key: CapabilityCacheKey {
            binary_path: PathBuf::from("/tmp/codex"),
        },
        fingerprint,
        version: None,
        features: CodexFeatureFlags::default(),
        probe_plan: CapabilityProbePlan::default(),
        collected_at,
    }
}

#[test]
fn parses_version_output_fields() {
    let parsed = version::parse_version_output("codex v3.4.5-nightly (commit abc1234)");
    assert_eq!(parsed.semantic, Some((3, 4, 5)));
    assert_eq!(parsed.channel, CodexReleaseChannel::Nightly);
    assert_eq!(parsed.commit.as_deref(), Some("abc1234"));
    assert_eq!(
        parsed.raw,
        "codex v3.4.5-nightly (commit abc1234)".to_string()
    );
}

#[test]
fn update_advisory_detects_newer_release() {
    let capabilities = capabilities_with_version("codex 1.0.0");
    let latest = CodexLatestReleases {
        stable: Some(Version::parse("1.1.0").unwrap()),
        ..Default::default()
    };
    let advisory = update_advisory_from_capabilities(&capabilities, &latest);
    assert_eq!(advisory.status, CodexUpdateStatus::UpdateRecommended);
    assert!(advisory.is_update_recommended());
    assert_eq!(
        advisory
            .latest_release
            .as_ref()
            .map(|release| release.version.clone()),
        latest.stable
    );
}

#[test]
fn update_advisory_handles_unknown_local_version() {
    let capabilities = capabilities_without_version();
    let latest = CodexLatestReleases {
        stable: Some(Version::parse("3.2.1").unwrap()),
        ..Default::default()
    };
    let advisory = update_advisory_from_capabilities(&capabilities, &latest);
    assert_eq!(advisory.status, CodexUpdateStatus::UnknownLocalVersion);
    assert!(advisory.is_update_recommended());
    assert!(advisory
        .notes
        .iter()
        .any(|note| note.contains("could not be parsed")));
}

#[test]
fn update_advisory_marks_up_to_date() {
    let capabilities = capabilities_with_version("codex 2.0.1");
    let latest = CodexLatestReleases {
        stable: Some(Version::parse("2.0.1").unwrap()),
        ..Default::default()
    };
    let advisory = update_advisory_from_capabilities(&capabilities, &latest);
    assert_eq!(advisory.status, CodexUpdateStatus::UpToDate);
    assert!(!advisory.is_update_recommended());
}

#[test]
fn update_advisory_falls_back_when_channel_missing() {
    let capabilities = capabilities_with_version("codex 2.0.0-beta");
    let latest = CodexLatestReleases {
        stable: Some(Version::parse("2.0.1").unwrap()),
        ..Default::default()
    };
    let advisory = update_advisory_from_capabilities(&capabilities, &latest);
    assert_eq!(advisory.comparison_channel, CodexReleaseChannel::Stable);
    assert_eq!(advisory.status, CodexUpdateStatus::UpdateRecommended);
    assert!(advisory
        .notes
        .iter()
        .any(|note| note.contains("comparing against stable")));
}

#[test]
fn update_advisory_handles_local_newer_than_known() {
    let capabilities = capabilities_with_version("codex 2.0.0");
    let latest = CodexLatestReleases {
        stable: Some(Version::parse("1.9.9").unwrap()),
        ..Default::default()
    };
    let advisory = update_advisory_from_capabilities(&capabilities, &latest);
    assert_eq!(advisory.status, CodexUpdateStatus::LocalNewerThanKnown);
    assert!(!advisory.is_update_recommended());
    assert!(advisory
        .notes
        .iter()
        .any(|note| note.contains("newer than provided")));
}

#[test]
fn update_advisory_handles_missing_latest_metadata() {
    let capabilities = capabilities_with_version("codex 1.0.0");
    let latest = CodexLatestReleases::default();
    let advisory = update_advisory_from_capabilities(&capabilities, &latest);
    assert_eq!(advisory.status, CodexUpdateStatus::UnknownLatestVersion);
    assert!(!advisory.is_update_recommended());
    assert!(advisory
        .notes
        .iter()
        .any(|note| note.contains("advisory unavailable")));
}

#[test]
fn capability_snapshots_serialize_to_json_and_toml() {
    let snapshot = sample_capabilities_snapshot();

    let json = serialize_capabilities_snapshot(&snapshot, CapabilitySnapshotFormat::Json)
        .expect("serialize json");
    let parsed_json = deserialize_capabilities_snapshot(&json, CapabilitySnapshotFormat::Json)
        .expect("parse json");
    assert_eq!(parsed_json, snapshot);

    let toml = serialize_capabilities_snapshot(&snapshot, CapabilitySnapshotFormat::Toml)
        .expect("serialize toml");
    let parsed_toml = deserialize_capabilities_snapshot(&toml, CapabilitySnapshotFormat::Toml)
        .expect("parse toml");
    assert_eq!(parsed_toml, snapshot);
}

#[test]
fn capability_snapshots_and_overrides_round_trip_via_files() {
    let snapshot = sample_capabilities_snapshot();
    let overrides = sample_capability_overrides();
    let temp = tempfile::tempdir().unwrap();

    let snapshot_path = temp.path().join("capabilities.toml");
    write_capabilities_snapshot(&snapshot_path, &snapshot, None).unwrap();
    let loaded_snapshot = read_capabilities_snapshot(&snapshot_path, None).unwrap();
    assert_eq!(loaded_snapshot, snapshot);

    let overrides_path = temp.path().join("overrides.json");
    write_capability_overrides(&overrides_path, &overrides, None).unwrap();
    let loaded_overrides = read_capability_overrides(&overrides_path, None).unwrap();
    assert_eq!(loaded_overrides, overrides);
}

#[test]
fn capability_snapshot_match_checks_fingerprint() {
    let temp = tempfile::tempdir().unwrap();
    let script = "#!/bin/bash\necho ok";
    let binary = write_fake_codex(temp.path(), script);
    let cache_key = capability_cache_key(&binary);
    let fingerprint = current_fingerprint(&cache_key);

    let snapshot = CodexCapabilities {
        cache_key: cache_key.clone(),
        fingerprint: fingerprint.clone(),
        version: None,
        features: CodexFeatureFlags::default(),
        probe_plan: CapabilityProbePlan::default(),
        collected_at: SystemTime::UNIX_EPOCH,
    };

    assert!(capability_snapshot_matches_binary(&snapshot, &binary));
    let mut missing_fingerprint = snapshot.clone();
    missing_fingerprint.fingerprint = None;
    assert!(!capability_snapshot_matches_binary(
        &missing_fingerprint,
        &binary
    ));

    std_fs::write(&binary, "#!/bin/bash\necho changed").unwrap();
    let mut perms = std_fs::metadata(&binary).unwrap().permissions();
    perms.set_mode(0o755);
    std_fs::set_permissions(&binary, perms).unwrap();

    assert!(!capability_snapshot_matches_binary(&snapshot, &binary));
}

#[test]
fn capability_cache_entries_exposes_cache_state() {
    let _guard = env_guard();
    clear_capability_cache();

    let temp = tempfile::tempdir().unwrap();
    let binary = write_fake_codex(temp.path(), "#!/bin/bash\necho ok");
    let cache_key = capability_cache_key(&binary);
    let fingerprint = current_fingerprint(&cache_key);

    let snapshot = CodexCapabilities {
        cache_key: cache_key.clone(),
        fingerprint: fingerprint.clone(),
        version: Some(version::parse_version_output("codex 0.0.1")),
        features: CodexFeatureFlags {
            supports_features_list: true,
            supports_output_schema: true,
            supports_add_dir: false,
            supports_mcp_login: false,
        },
        probe_plan: CapabilityProbePlan {
            steps: vec![CapabilityProbeStep::VersionFlag],
        },
        collected_at: SystemTime::UNIX_EPOCH,
    };

    update_capability_cache(snapshot.clone());

    let entries = capability_cache_entries();
    assert!(entries.iter().any(|entry| entry.cache_key == cache_key));

    let fetched = capability_cache_entry(&binary).expect("expected cache entry");
    assert_eq!(fetched.cache_key, cache_key);
    assert!(clear_capability_cache_entry(&binary));
    assert!(capability_cache_entry(&binary).is_none());
    assert!(capability_cache_entries().is_empty());
    clear_capability_cache();
}

#[test]
fn capability_ttl_decision_reuses_fresh_snapshot() {
    let collected_at = SystemTime::UNIX_EPOCH + Duration::from_secs(10);
    let snapshot = capability_snapshot_with_metadata(
        collected_at,
        Some(BinaryFingerprint {
            canonical_path: Some(PathBuf::from("/tmp/codex")),
            modified: Some(SystemTime::UNIX_EPOCH + Duration::from_secs(1)),
            len: Some(123),
        }),
    );

    let decision = capability_cache_ttl_decision(
        Some(&snapshot),
        Duration::from_secs(300),
        SystemTime::UNIX_EPOCH + Duration::from_secs(100),
    );
    assert!(!decision.should_probe);
    assert_eq!(decision.policy, CapabilityCachePolicy::PreferCache);
}

#[test]
fn capability_ttl_decision_refreshes_after_ttl_with_fingerprint() {
    let collected_at = SystemTime::UNIX_EPOCH + Duration::from_secs(1);
    let snapshot = capability_snapshot_with_metadata(
        collected_at,
        Some(BinaryFingerprint {
            canonical_path: Some(PathBuf::from("/tmp/codex")),
            modified: Some(SystemTime::UNIX_EPOCH + Duration::from_secs(1)),
            len: Some(321),
        }),
    );

    let decision = capability_cache_ttl_decision(
        Some(&snapshot),
        Duration::from_secs(5),
        SystemTime::UNIX_EPOCH + Duration::from_secs(10),
    );
    assert!(decision.should_probe);
    assert_eq!(decision.policy, CapabilityCachePolicy::Refresh);
}

#[test]
fn capability_ttl_decision_bypasses_when_metadata_missing() {
    let collected_at = SystemTime::UNIX_EPOCH + Duration::from_secs(2);
    let snapshot = capability_snapshot_with_metadata(collected_at, None);

    let decision = capability_cache_ttl_decision(
        Some(&snapshot),
        Duration::from_secs(5),
        SystemTime::UNIX_EPOCH + Duration::from_secs(10),
    );
    assert!(decision.should_probe);
    assert_eq!(decision.policy, CapabilityCachePolicy::Bypass);
}

#[tokio::test]
async fn probe_reprobes_when_metadata_missing() {
    let _guard = env_guard_async().await;
    clear_capability_cache();

    let temp = tempfile::tempdir().unwrap();
    let binary = temp.path().join("missing_codex");
    let cache_key = capability_cache_key(&binary);

    {
        let mut cache = capability_cache().lock().unwrap();
        cache.insert(
            cache_key.clone(),
            CodexCapabilities {
                cache_key: cache_key.clone(),
                fingerprint: None,
                version: Some(version::parse_version_output("codex 9.9.9")),
                features: CodexFeatureFlags {
                    supports_features_list: true,
                    supports_output_schema: true,
                    supports_add_dir: true,
                    supports_mcp_login: true,
                },
                probe_plan: CapabilityProbePlan::default(),
                collected_at: SystemTime::UNIX_EPOCH,
            },
        );
    }

    let client = CodexClient::builder()
        .binary(&binary)
        .timeout(Duration::from_secs(1))
        .build();

    let capabilities = client.probe_capabilities().await;
    assert!(!capabilities.features.supports_output_schema);
    assert!(capabilities
        .probe_plan
        .steps
        .contains(&CapabilityProbeStep::VersionFlag));

    clear_capability_cache();
}

#[tokio::test]
async fn probe_refresh_policy_forces_new_snapshot() {
    let _guard = env_guard_async().await;
    clear_capability_cache();

    let temp = tempfile::tempdir().unwrap();
    let log_path = temp.path().join("probe.log");
    let script = format!(
        r#"#!/bin/bash
echo "$@" >> "{log}"
if [[ "$1" == "--version" ]]; then
  echo "codex 1.0.0"
elif [[ "$1" == "features" && "$2" == "list" && "$3" == "--json" ]]; then
  echo '{{"features":["output_schema"]}}'
elif [[ "$1" == "features" && "$2" == "list" ]]; then
  echo "output_schema"
fi
"#,
        log = log_path.display()
    );
    let binary = write_fake_codex(temp.path(), &script);
    let client = CodexClient::builder()
        .binary(&binary)
        .timeout(Duration::from_secs(5))
        .build();

    let first = client.probe_capabilities().await;
    assert!(first.features.supports_output_schema);
    let first_lines = std_fs::read_to_string(&log_path).unwrap().lines().count();
    assert!(first_lines >= 2);

    let refreshed = client
        .probe_capabilities_with_policy(CapabilityCachePolicy::Refresh)
        .await;
    assert!(refreshed.features.supports_output_schema);
    let refreshed_lines = std_fs::read_to_string(&log_path).unwrap().lines().count();
    assert!(
        refreshed_lines > first_lines,
        "expected refresh policy to re-run probes"
    );
    clear_capability_cache();
}

#[tokio::test]
async fn probe_bypass_policy_skips_cache_writes() {
    let _guard = env_guard_async().await;
    clear_capability_cache();

    let temp = tempfile::tempdir().unwrap();
    let script = r#"#!/bin/bash
if [[ "$1" == "--version" ]]; then
  echo "codex 1.0.0"
elif [[ "$1" == "features" && "$2" == "list" && "$3" == "--json" ]]; then
  echo '{"features":["output_schema"]}'
elif [[ "$1" == "features" && "$2" == "list" ]]; then
  echo "output_schema"
fi
"#;
    let binary = write_fake_codex(temp.path(), script);

    let client = CodexClient::builder()
        .binary(&binary)
        .timeout(Duration::from_secs(5))
        .build();

    let capabilities = client
        .probe_capabilities_with_policy(CapabilityCachePolicy::Bypass)
        .await;
    assert!(capabilities.features.supports_output_schema);
    assert!(capability_cache_entry(&binary).is_none());
    clear_capability_cache();
}

#[test]
fn parses_features_from_json_and_text() {
    let json = r#"{"features":["output_schema","add_dir"],"mcp_login":true}"#;
    let parsed_json = version::parse_features_from_json(json).unwrap();
    assert!(parsed_json.supports_output_schema);
    assert!(parsed_json.supports_add_dir);
    assert!(parsed_json.supports_mcp_login);

    let text = "Features: output-schema add-dir login --mcp";
    let parsed_text = version::parse_features_from_text(text);
    assert!(parsed_text.supports_output_schema);
    assert!(parsed_text.supports_add_dir);
    assert!(parsed_text.supports_mcp_login);
}

#[test]
fn parses_feature_list_json_and_text_tables() {
    let json = r#"{"features":[{"name":"json-stream","stage":"stable","enabled":true,"notes":"keep"},{"name":"cloud-exec","stage":"experimental","enabled":false}]}"#;
    let (json_features, json_format) = version::parse_feature_list_output(json, true).unwrap();
    assert_eq!(json_format, FeaturesListFormat::Json);
    assert_eq!(json_features.len(), 2);
    assert_eq!(json_features[0].name, "json-stream");
    assert_eq!(json_features[0].stage, Some(CodexFeatureStage::Stable));
    assert!(json_features[0].enabled);
    assert!(json_features[0].extra.contains_key("notes"));
    assert_eq!(
        json_features[1].stage,
        Some(CodexFeatureStage::Experimental)
    );
    assert!(!json_features[1].enabled);

    let text = r#"
Feature   Stage         Enabled
json-stream stable      true
	cloud-exec experimental false
	"#;
    let (text_features, text_format) = version::parse_feature_list_output(text, false).unwrap();
    assert_eq!(text_format, FeaturesListFormat::Text);
    assert_eq!(text_features.len(), 2);
    assert_eq!(
        text_features[1].stage,
        Some(CodexFeatureStage::Experimental)
    );
    assert!(!text_features[1].enabled);

    let (fallback_features, fallback_format) =
        version::parse_feature_list_output(text, true).unwrap();
    assert_eq!(fallback_format, FeaturesListFormat::Text);
    assert_eq!(fallback_features.len(), 2);
}

#[test]
fn parses_help_output_flags() {
    let help =
        "Usage: codex --output-schema ... add-dir ... login --mcp. See `codex features list`.";
    let parsed = version::parse_help_output(help);
    assert!(parsed.supports_output_schema);
    assert!(parsed.supports_add_dir);
    assert!(parsed.supports_mcp_login);
    assert!(parsed.supports_features_list);
}

#[test]
fn capability_guard_reports_detected_support() {
    let flags = CodexFeatureFlags {
        supports_features_list: true,
        supports_output_schema: true,
        supports_add_dir: true,
        supports_mcp_login: true,
    };
    let capabilities = capabilities_with_feature_flags(flags);

    let output_schema = capabilities.guard_output_schema();
    assert_eq!(output_schema.support, CapabilitySupport::Supported);
    assert!(output_schema.is_supported());

    let add_dir = capabilities.guard_add_dir();
    assert_eq!(add_dir.support, CapabilitySupport::Supported);
    assert!(add_dir.is_supported());

    let mcp_login = capabilities.guard_mcp_login();
    assert_eq!(mcp_login.support, CapabilitySupport::Supported);

    let features_list = capabilities.guard_features_list();
    assert_eq!(features_list.support, CapabilitySupport::Supported);
}

#[test]
fn capability_guard_marks_absent_feature_as_unsupported() {
    let flags = CodexFeatureFlags {
        supports_features_list: true,
        supports_output_schema: false,
        supports_add_dir: false,
        supports_mcp_login: false,
    };
    let capabilities = capabilities_with_feature_flags(flags);

    let output_schema = capabilities.guard_output_schema();
    assert_eq!(output_schema.support, CapabilitySupport::Unsupported);
    assert!(!output_schema.is_supported());
    assert!(output_schema
        .notes
        .iter()
        .any(|note| note.contains("features list")));

    let mcp_login = capabilities.guard_mcp_login();
    assert_eq!(mcp_login.support, CapabilitySupport::Unsupported);
}

#[test]
fn capability_guard_returns_unknown_without_feature_list() {
    let capabilities = capabilities_with_feature_flags(CodexFeatureFlags::default());

    let add_dir = capabilities.guard_add_dir();
    assert_eq!(add_dir.support, CapabilitySupport::Unknown);
    assert!(add_dir.is_unknown());
    assert!(add_dir
        .notes
        .iter()
        .any(|note| note.contains("unknown") || note.contains("unavailable")));

    let features_list = capabilities.guard_features_list();
    assert_eq!(features_list.support, CapabilitySupport::Unknown);
}

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

#[tokio::test]
async fn exec_applies_guarded_flags_when_supported() {
    let _guard = env_guard_async().await;
    clear_capability_cache();

    let temp = tempfile::tempdir().unwrap();
    let log_path = temp.path().join("exec.log");
    let script = format!(
        r#"#!/bin/bash
log="{log}"
if [[ "$1" == "--version" ]]; then
  echo "codex 1.2.3"
elif [[ "$1" == "features" && "$2" == "list" && "$3" == "--json" ]]; then
  echo '{{"features":["output_schema","add_dir","mcp_login"]}}'
elif [[ "$1" == "features" && "$2" == "list" ]]; then
  echo "output_schema add_dir login --mcp"
elif [[ "$1" == "--help" ]]; then
  echo "Usage: codex --output-schema add-dir login --mcp"
elif [[ "$1" == "exec" ]]; then
  echo "$@" >> "$log"
  echo "ok"
fi
"#,
        log = log_path.display()
    );
    let binary = write_fake_codex(temp.path(), &script);
    let client = CodexClient::builder()
        .binary(&binary)
        .timeout(Duration::from_secs(5))
        .add_dir("src")
        .output_schema(true)
        .quiet(true)
        .mirror_stdout(false)
        .build();

    let response = client.send_prompt("hello").await.unwrap();
    assert_eq!(response.trim(), "ok");

    let logged = std_fs::read_to_string(&log_path).unwrap();
    assert!(logged.contains("--add-dir"));
    assert!(logged.contains("src"));
    assert!(logged.contains("--output-schema"));
}

#[tokio::test]
async fn exec_skips_guarded_flags_when_unknown() {
    let _guard = env_guard_async().await;
    clear_capability_cache();

    let temp = tempfile::tempdir().unwrap();
    let log_path = temp.path().join("exec.log");
    let script = format!(
        r#"#!/bin/bash
log="{log}"
if [[ "$1" == "--version" ]]; then
  echo "codex 0.9.0"
elif [[ "$1" == "features" && "$2" == "list" && "$3" == "--json" ]]; then
  echo "feature list unavailable" >&2
  exit 1
elif [[ "$1" == "features" && "$2" == "list" ]]; then
  echo "feature list unavailable" >&2
  exit 1
elif [[ "$1" == "--help" ]]; then
  echo "Usage: codex exec"
elif [[ "$1" == "exec" ]]; then
  echo "$@" >> "$log"
  echo "ok"
fi
"#,
        log = log_path.display()
    );
    let binary = write_fake_codex(temp.path(), &script);
    let client = CodexClient::builder()
        .binary(&binary)
        .timeout(Duration::from_secs(5))
        .add_dir("src")
        .output_schema(true)
        .quiet(true)
        .mirror_stdout(false)
        .build();

    let response = client.send_prompt("hello").await.unwrap();
    assert_eq!(response.trim(), "ok");

    let logged = std_fs::read_to_string(&log_path).unwrap();
    assert!(!logged.contains("--add-dir"));
    assert!(!logged.contains("--output-schema"));
}

#[tokio::test]
async fn mcp_login_skips_when_unsupported() {
    let _guard = env_guard_async().await;
    clear_capability_cache();

    let temp = tempfile::tempdir().unwrap();
    let log_path = temp.path().join("login.log");
    let script = format!(
        r#"#!/bin/bash
log="{log}"
if [[ "$1" == "--version" ]]; then
  echo "codex 1.0.0"
elif [[ "$1" == "features" && "$2" == "list" && "$3" == "--json" ]]; then
  echo '{{"features":["output_schema","add_dir"]}}'
elif [[ "$1" == "features" && "$2" == "list" ]]; then
  echo "output_schema add-dir"
elif [[ "$1" == "--help" ]]; then
  echo "Usage: codex exec"
elif [[ "$1" == "login" ]]; then
  echo "$@" >> "$log"
  echo "login invoked"
fi
"#,
        log = log_path.display()
    );
    let binary = write_fake_codex(temp.path(), &script);
    let client = CodexClient::builder()
        .binary(&binary)
        .timeout(Duration::from_secs(5))
        .build();

    let login = client.spawn_mcp_login_process().await.unwrap();
    assert!(login.is_none());
    assert!(!log_path.exists());
}

#[tokio::test]
async fn mcp_login_runs_when_supported() {
    let _guard = env_guard_async().await;
    clear_capability_cache();

    let temp = tempfile::tempdir().unwrap();
    let log_path = temp.path().join("login.log");
    let script = format!(
        r#"#!/bin/bash
log="{log}"
if [[ "$1" == "--version" ]]; then
  echo "codex 2.0.0"
elif [[ "$1" == "features" && "$2" == "list" && "$3" == "--json" ]]; then
  echo '{{"features":["output_schema","add_dir"],"mcp_login":true}}'
elif [[ "$1" == "features" && "$2" == "list" ]]; then
  echo "output_schema add_dir login --mcp"
elif [[ "$1" == "--help" ]]; then
  echo "Usage: codex --output-schema add-dir login --mcp"
elif [[ "$1" == "login" ]]; then
  echo "$@" >> "$log"
  echo "login invoked"
fi
"#,
        log = log_path.display()
    );
    let binary = write_fake_codex(temp.path(), &script);
    let client = CodexClient::builder()
        .binary(&binary)
        .timeout(Duration::from_secs(5))
        .build();

    let login = client
        .spawn_mcp_login_process()
        .await
        .unwrap()
        .expect("expected login child");
    let output = login.wait_with_output().await.unwrap();
    assert!(output.status.success());

    let logged = std_fs::read_to_string(&log_path).unwrap();
    assert!(logged.contains("login --mcp"));
}

#[tokio::test]
async fn probe_capabilities_caches_and_invalidates() {
    let _guard = env_guard_async().await;
    clear_capability_cache();

    let temp = tempfile::tempdir().unwrap();
    let script_v1 = r#"#!/bin/bash
if [[ "$1" == "--version" ]]; then
  echo "codex 1.2.3-beta (commit cafe123)"
elif [[ "$1" == "features" && "$2" == "list" && "$3" == "--json" ]]; then
  echo '{"features":["output_schema","add_dir","mcp_login"]}'
elif [[ "$1" == "features" && "$2" == "list" ]]; then
  echo "output_schema add-dir login --mcp"
elif [[ "$1" == "--help" ]]; then
  echo "Usage: codex --output-schema add-dir login --mcp"
fi
"#;
    let binary = write_fake_codex(temp.path(), script_v1);
    let client = CodexClient::builder()
        .binary(&binary)
        .timeout(Duration::from_secs(5))
        .build();

    let first = client.probe_capabilities().await;
    assert_eq!(
        first.version.as_ref().and_then(|v| v.semantic),
        Some((1, 2, 3))
    );
    assert_eq!(
        first.version.as_ref().map(|v| v.channel),
        Some(CodexReleaseChannel::Beta)
    );
    assert_eq!(
        first.version.as_ref().and_then(|v| v.commit.as_deref()),
        Some("cafe123")
    );
    assert!(first.features.supports_features_list);
    assert!(first.features.supports_output_schema);
    assert!(first.features.supports_add_dir);
    assert!(first.features.supports_mcp_login);

    let cached = client.probe_capabilities().await;
    assert_eq!(cached, first);

    let script_v2 = r#"#!/bin/bash
if [[ "$1" == "--version" ]]; then
  echo "codex 2.0.0 (commit deadbeef)"
elif [[ "$1" == "features" && "$2" == "list" && "$3" == "--json" ]]; then
  echo '{"features":["add_dir"]}'
elif [[ "$1" == "features" && "$2" == "list" ]]; then
  echo "add-dir"
elif [[ "$1" == "--help" ]]; then
  echo "Usage: codex add-dir"
fi
"#;
    std_fs::write(&binary, script_v2).unwrap();
    let mut perms = std_fs::metadata(&binary).unwrap().permissions();
    perms.set_mode(0o755);
    std_fs::set_permissions(&binary, perms).unwrap();

    let refreshed = client.probe_capabilities().await;
    assert_ne!(refreshed.version, first.version);
    assert_eq!(
        refreshed.version.as_ref().and_then(|v| v.semantic),
        Some((2, 0, 0))
    );
    assert!(refreshed.features.supports_features_list);
    assert!(refreshed.features.supports_add_dir);
    assert!(!refreshed.features.supports_output_schema);
    assert!(!refreshed.features.supports_mcp_login);
    clear_capability_cache();
}
