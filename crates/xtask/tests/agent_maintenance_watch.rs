#![allow(dead_code)]

use std::{fs, path::PathBuf};

#[path = "support/onboard_agent_harness.rs"]
mod harness;

mod agent_lifecycle {
    pub use xtask::agent_lifecycle::*;
}

mod agent_registry {
    pub use xtask::agent_registry::*;
}
#[path = "../src/agent_maintenance/contract_policy.rs"]
mod contract_policy;
#[path = "../src/agent_maintenance/request.rs"]
mod request;
#[path = "../src/agent_maintenance/support_audit.rs"]
mod support_audit;
#[path = "../src/workspace_mutation.rs"]
mod workspace_mutation;

#[path = "../src/agent_maintenance/watch.rs"]
mod watch;

use harness::{fixture_root, write_text};
use watch::{build_watch_queue_with_resolver, run_in_workspace_with_resolver, Args, Error};

const SEEDED_REGISTRY: &str = include_str!("../data/agent_registry.toml");
const CLAUDE_NPM_RELEASE_WATCH_UPSTREAM: &str =
    "source_kind = \"npm_dist_tag\"\npackage = \"@anthropic-ai/claude-code\"\ndist_tag = \"stable\"";
const CLAUDE_GCS_RELEASE_WATCH_UPSTREAM: &str = "source_kind = \"gcs_object_listing\"\nbucket = \"claude-code-dist-86c565f3-f756-42ad-8dfa-d59b1c096819\"\nprefix = \"claude-code-releases\"\nversion_marker = \"manifest.json\"";

#[test]
fn build_watch_queue_emits_frozen_fields_for_stale_agents() {
    let fixture = fixture_root("agent-maintenance-watch-queue");
    seed_registry(&fixture);
    seed_latest_validated(&fixture, "cli_manifests/codex", "0.97.0");
    seed_latest_validated(&fixture, "cli_manifests/claude_code", "1.2.3");
    seed_latest_validated(&fixture, "cli_manifests/opencode", "1.4.9");

    let queue = build_watch_queue_with_resolver(&fixture, resolver_for_queue).expect("queue");

    assert_eq!(queue.schema_version, 1);
    assert!(!queue.generated_at.is_empty());
    assert!(queue.failed_agents.is_empty());
    assert_eq!(
        queue.stale_agents,
        vec![
            watch::MaintenanceWatchQueueEntry {
                agent_id: "codex".to_string(),
                manifest_root: "cli_manifests/codex".to_string(),
                current_validated: "0.97.0".to_string(),
                latest_stable: "0.99.0".to_string(),
                target_version: "0.98.0".to_string(),
                version_policy: "latest_stable_minus_one".to_string(),
                dispatch_kind: "packet_pr".to_string(),
                dispatch_workflow: "agent-maintenance-open-pr.yml".to_string(),
                maintenance_root: "docs/agents/lifecycle/codex-maintenance".to_string(),
                request_path:
                    "docs/agents/lifecycle/codex-maintenance/governance/maintenance-request.toml"
                        .to_string(),
                opened_from: ".github/workflows/agent-maintenance-open-pr.yml".to_string(),
                detected_by: ".github/workflows/agent-maintenance-release-watch.yml".to_string(),
                branch_name: "automation/codex-maintenance-0.98.0".to_string(),
            },
            watch::MaintenanceWatchQueueEntry {
                agent_id: "claude_code".to_string(),
                manifest_root: "cli_manifests/claude_code".to_string(),
                current_validated: "1.2.3".to_string(),
                latest_stable: "1.2.5".to_string(),
                target_version: "1.2.5".to_string(),
                version_policy: "upstream_stable_pointer".to_string(),
                dispatch_kind: "packet_pr".to_string(),
                dispatch_workflow: "agent-maintenance-open-pr.yml".to_string(),
                maintenance_root: "docs/agents/lifecycle/claude_code-maintenance".to_string(),
                request_path:
                    "docs/agents/lifecycle/claude_code-maintenance/governance/maintenance-request.toml"
                        .to_string(),
                opened_from: ".github/workflows/agent-maintenance-open-pr.yml".to_string(),
                detected_by: ".github/workflows/agent-maintenance-release-watch.yml".to_string(),
                branch_name: "automation/claude_code-maintenance-1.2.5".to_string(),
            },
            watch::MaintenanceWatchQueueEntry {
                agent_id: "opencode".to_string(),
                manifest_root: "cli_manifests/opencode".to_string(),
                current_validated: "1.4.9".to_string(),
                latest_stable: "1.4.12".to_string(),
                target_version: "1.4.11".to_string(),
                version_policy: "latest_stable_minus_one".to_string(),
                dispatch_kind: "packet_pr".to_string(),
                dispatch_workflow: "agent-maintenance-open-pr.yml".to_string(),
                maintenance_root: "docs/agents/lifecycle/opencode-maintenance".to_string(),
                request_path:
                    "docs/agents/lifecycle/opencode-maintenance/governance/maintenance-request.toml"
                        .to_string(),
                opened_from: ".github/workflows/agent-maintenance-open-pr.yml".to_string(),
                detected_by: ".github/workflows/agent-maintenance-release-watch.yml".to_string(),
                branch_name: "automation/opencode-maintenance-1.4.11".to_string(),
            },
        ]
    );
}

#[test]
fn run_in_workspace_emits_json_queue_file() {
    let fixture = fixture_root("agent-maintenance-watch-emit-json");
    seed_registry(&fixture);
    seed_latest_validated(&fixture, "cli_manifests/codex", "0.97.0");
    seed_latest_validated(&fixture, "cli_manifests/claude_code", "1.2.3");
    seed_latest_validated(&fixture, "cli_manifests/opencode", "1.4.9");

    let mut stdout = Vec::new();
    run_in_workspace_with_resolver(
        &fixture,
        Args {
            check: false,
            emit_json: Some(PathBuf::from("_ci_tmp/maintenance-watch.json")),
            agent: None,
        },
        &mut stdout,
        resolver_for_queue,
    )
    .expect("emit queue");

    let output = String::from_utf8(stdout).expect("stdout utf8");
    assert!(output.contains("stale_agents: 3"));
    assert!(output.contains("failed_agents: 0"));
    assert!(output.contains("emitted_json: _ci_tmp/maintenance-watch.json"));

    let written = fs::read_to_string(fixture.join("_ci_tmp/maintenance-watch.json"))
        .expect("read queue json");
    let parsed: watch::MaintenanceWatchQueue =
        serde_json::from_str(&written).expect("parse queue json");
    assert_eq!(parsed.schema_version, 1);
    assert_eq!(parsed.stale_agents.len(), 3);
    assert!(parsed.failed_agents.is_empty());
}

#[test]
fn run_in_workspace_check_fails_when_stale_agents_are_present() {
    let fixture = fixture_root("agent-maintenance-watch-check");
    seed_registry(&fixture);
    seed_latest_validated(&fixture, "cli_manifests/codex", "0.97.0");
    seed_latest_validated(&fixture, "cli_manifests/claude_code", "1.2.3");
    seed_latest_validated(&fixture, "cli_manifests/opencode", "1.4.9");

    let mut stdout = Vec::new();
    let err = run_in_workspace_with_resolver(
        &fixture,
        Args {
            check: true,
            emit_json: None,
            agent: None,
        },
        &mut stdout,
        resolver_for_queue,
    )
    .expect_err("check mode should fail when stale agents exist");

    assert!(matches!(err, Error::Validation(_)));
    assert!(err.to_string().contains("found 3 stale enrolled agent"));

    let output = String::from_utf8(stdout).expect("stdout utf8");
    assert!(output.contains("stale_agents: 3"));
    assert!(output.contains("failed_agents: 0"));
}

#[test]
fn clean_or_not_newer_agents_are_not_emitted() {
    let fixture = fixture_root("agent-maintenance-watch-clean");
    seed_registry(&fixture);
    seed_latest_validated(&fixture, "cli_manifests/codex", "0.98.0");
    seed_latest_validated(&fixture, "cli_manifests/claude_code", "1.2.5");
    seed_latest_validated(&fixture, "cli_manifests/opencode", "1.4.11");

    let queue = build_watch_queue_with_resolver(&fixture, resolver_for_queue).expect("queue");
    assert!(queue.stale_agents.is_empty());
    assert!(queue.failed_agents.is_empty());
}

#[test]
fn partial_upstream_failures_are_isolated_into_failed_agents() {
    let fixture = fixture_root("agent-maintenance-watch-partial-failure");
    seed_registry(&fixture);
    seed_latest_validated(&fixture, "cli_manifests/codex", "0.97.0");
    seed_latest_validated(&fixture, "cli_manifests/claude_code", "1.2.3");
    seed_latest_validated(&fixture, "cli_manifests/opencode", "1.4.9");

    let queue = build_watch_queue_with_resolver(&fixture, |entry, _| {
        if entry.agent_id == "codex" {
            Err(Error::Validation("synthetic upstream failure".to_string()))
        } else if entry.agent_id == "opencode" {
            Ok(vec!["1.4.12".parse().unwrap(), "1.4.11".parse().unwrap()])
        } else {
            Ok(vec!["1.2.5".parse().unwrap()])
        }
    })
    .expect("partial failures should not abort queue");

    assert_eq!(
        queue
            .stale_agents
            .iter()
            .map(|entry| entry.agent_id.as_str())
            .collect::<Vec<_>>(),
        vec!["claude_code", "opencode"]
    );
    assert_eq!(
        queue.failed_agents,
        vec![watch::MaintenanceWatchQueueFailure {
            agent_id: "codex".to_string(),
            error: "synthetic upstream failure".to_string(),
        }]
    );
}

#[test]
fn enrolled_agents_use_generic_open_pr_workflow() {
    let fixture = fixture_root("agent-maintenance-watch-open-pr");
    seed_registry(&fixture);
    seed_latest_validated(&fixture, "cli_manifests/codex", "0.97.0");
    seed_latest_validated(&fixture, "cli_manifests/claude_code", "1.2.3");
    seed_latest_validated(&fixture, "cli_manifests/opencode", "1.4.9");

    let queue = build_watch_queue_with_resolver(&fixture, resolver_for_queue).expect("queue");
    assert!(queue.failed_agents.is_empty());
    for agent_id in ["codex", "claude_code", "opencode"] {
        let entry = queue
            .stale_agents
            .iter()
            .find(|entry| entry.agent_id == agent_id)
            .unwrap_or_else(|| panic!("missing stale agent {agent_id}"));
        assert_eq!(entry.dispatch_kind, "packet_pr");
        assert_eq!(entry.dispatch_workflow, "agent-maintenance-open-pr.yml");
        assert_eq!(
            entry.opened_from,
            ".github/workflows/agent-maintenance-open-pr.yml"
        );
    }
}

#[test]
fn gcs_page_tokens_are_percent_encoded_for_pagination() {
    let fixture = fixture_root("agent-maintenance-watch-gcs-page-token");
    seed_registry_with(
        &fixture,
        &SEEDED_REGISTRY.replacen(
            CLAUDE_NPM_RELEASE_WATCH_UPSTREAM,
            CLAUDE_GCS_RELEASE_WATCH_UPSTREAM,
            1,
        ),
    );

    let registry =
        xtask::agent_registry::AgentRegistry::load(&fixture).expect("seeded registry loads");
    let entry = registry
        .agents
        .iter()
        .find(|entry| entry.agent_id == "claude_code")
        .expect("claude_code registry entry");
    let release_watch = entry
        .maintenance
        .release_watch
        .as_ref()
        .expect("claude_code release watch");

    let mut urls = Vec::new();
    let versions = watch::fetch_gcs_versions_with_fetcher(entry, release_watch, |url| {
        urls.push(url.to_string());
        if url.contains("pageToken=") {
            Ok(r#"{"items":[{"name":"claude-code-releases/1.2.5/manifest.json"}]}"#.to_string())
        } else {
            Ok(
                r#"{"items":[{"name":"claude-code-releases/1.2.4/manifest.json"}],"nextPageToken":"token+/="}"#
                    .to_string(),
            )
        }
    })
    .expect("gcs pagination fetch succeeds");

    assert_eq!(
        versions,
        vec!["1.2.4".parse().unwrap(), "1.2.5".parse().unwrap(),]
    );
    assert_eq!(urls.len(), 2);
    assert!(urls[1].contains("pageToken=token%2B%2F%3D"));
}

#[test]
fn agent_filter_scopes_queue_to_single_enrolled_agent() {
    let fixture = fixture_root("agent-maintenance-watch-agent-filter");
    seed_registry(&fixture);
    seed_latest_validated(&fixture, "cli_manifests/codex", "0.97.0");
    seed_latest_validated(&fixture, "cli_manifests/claude_code", "1.2.3");
    seed_latest_validated(&fixture, "cli_manifests/opencode", "1.4.9");

    let mut stdout = Vec::new();
    run_in_workspace_with_resolver(
        &fixture,
        Args {
            check: false,
            emit_json: None,
            agent: Some("claude_code".to_string()),
        },
        &mut stdout,
        |entry, _| match entry.agent_id.as_str() {
            "claude_code" => Ok(vec!["1.2.5".parse().unwrap()]),
            other => panic!("unexpected filtered agent {other}"),
        },
    )
    .expect("agent-scoped queue");

    let output = String::from_utf8(stdout).expect("stdout utf8");
    assert!(output.contains("stale_agents: 1"));
    assert!(output.contains("failed_agents: 0"));
    assert!(output.contains("claude_code -> 1.2.5"));
    assert!(!output.contains("codex ->"));
    assert!(!output.contains("opencode ->"));
}

#[test]
fn npm_dist_tag_fetcher_returns_requested_stable_pointer() {
    let fixture = fixture_root("agent-maintenance-watch-npm-dist-tag");
    seed_registry(&fixture);

    let registry =
        xtask::agent_registry::AgentRegistry::load(&fixture).expect("seeded registry loads");
    let entry = registry
        .agents
        .iter()
        .find(|entry| entry.agent_id == "claude_code")
        .expect("claude_code registry entry");
    let release_watch = entry
        .maintenance
        .release_watch
        .as_ref()
        .expect("claude_code release watch");

    let versions = watch::fetch_npm_dist_tag_version_with_fetcher(entry, release_watch, |url| {
        assert_eq!(
            url,
            "https://registry.npmjs.org/%40anthropic-ai%2Fclaude-code"
        );
        Ok(r#"{"dist-tags":{"stable":"2.1.206","latest":"2.1.218"}}"#.to_string())
    })
    .expect("npm dist-tag fetch succeeds");

    assert_eq!(versions, vec!["2.1.206".parse().unwrap()]);
}

#[test]
fn npm_dist_tag_fetcher_fails_when_requested_tag_is_missing() {
    let fixture = fixture_root("agent-maintenance-watch-npm-missing-tag");
    seed_registry(&fixture);

    let registry =
        xtask::agent_registry::AgentRegistry::load(&fixture).expect("seeded registry loads");
    let entry = registry
        .agents
        .iter()
        .find(|entry| entry.agent_id == "claude_code")
        .expect("claude_code registry entry");
    let release_watch = entry
        .maintenance
        .release_watch
        .as_ref()
        .expect("claude_code release watch");

    let err = watch::fetch_npm_dist_tag_version_with_fetcher(entry, release_watch, |_| {
        Ok(r#"{"dist-tags":{"latest":"2.1.218"}}"#.to_string())
    })
    .expect_err("missing dist-tag should fail");

    assert!(matches!(err, Error::Validation(_)));
    assert!(err.to_string().contains("npm dist-tag `stable` missing"));
}

#[test]
fn npm_dist_tag_fetcher_fails_closed_on_malformed_json() {
    let fixture = fixture_root("agent-maintenance-watch-npm-malformed-json");
    seed_registry(&fixture);

    let registry =
        xtask::agent_registry::AgentRegistry::load(&fixture).expect("seeded registry loads");
    let entry = registry
        .agents
        .iter()
        .find(|entry| entry.agent_id == "claude_code")
        .expect("claude_code registry entry");
    let release_watch = entry
        .maintenance
        .release_watch
        .as_ref()
        .expect("claude_code release watch");

    let err = watch::fetch_npm_dist_tag_version_with_fetcher(entry, release_watch, |_| {
        Ok("{not json".to_string())
    })
    .expect_err("malformed npm metadata should fail");

    assert!(matches!(err, Error::Validation(_)));
    assert!(err.to_string().contains("parse npm metadata"));
}

#[test]
fn npm_dist_tag_fetcher_fails_closed_on_blank_version() {
    let fixture = fixture_root("agent-maintenance-watch-npm-blank-version");
    seed_registry(&fixture);

    let registry =
        xtask::agent_registry::AgentRegistry::load(&fixture).expect("seeded registry loads");
    let entry = registry
        .agents
        .iter()
        .find(|entry| entry.agent_id == "claude_code")
        .expect("claude_code registry entry");
    let release_watch = entry
        .maintenance
        .release_watch
        .as_ref()
        .expect("claude_code release watch");

    let err = watch::fetch_npm_dist_tag_version_with_fetcher(entry, release_watch, |_| {
        Ok(r#"{"dist-tags":{"stable":"  "}}"#.to_string())
    })
    .expect_err("blank dist-tag version should fail");

    assert!(matches!(err, Error::Validation(_)));
}

#[test]
fn npm_dist_tag_fetcher_fails_closed_when_dist_tags_object_missing() {
    let fixture = fixture_root("agent-maintenance-watch-npm-no-dist-tags");
    seed_registry(&fixture);

    let registry =
        xtask::agent_registry::AgentRegistry::load(&fixture).expect("seeded registry loads");
    let entry = registry
        .agents
        .iter()
        .find(|entry| entry.agent_id == "claude_code")
        .expect("claude_code registry entry");
    let release_watch = entry
        .maintenance
        .release_watch
        .as_ref()
        .expect("claude_code release watch");

    let err = watch::fetch_npm_dist_tag_version_with_fetcher(entry, release_watch, |_| {
        Ok(r#"{"versions":{}}"#.to_string())
    })
    .expect_err("missing dist-tags object should fail");

    assert!(matches!(err, Error::Validation(_)));
}

#[test]
fn upstream_stable_pointer_selects_last_sorted_version() {
    let selected = watch::select_target_version(
        &["2.1.206".parse().unwrap()],
        xtask::agent_registry::ReleaseWatchVersionPolicy::UpstreamStablePointer,
    );

    assert_eq!(selected, Some("2.1.206".parse().unwrap()));
}

#[test]
fn github_release_pagination_assembles_all_pages() {
    let fixture = fixture_root("agent-maintenance-watch-github-pagination");
    seed_registry(&fixture);

    let registry =
        xtask::agent_registry::AgentRegistry::load(&fixture).expect("seeded registry loads");
    let entry = registry
        .agents
        .iter()
        .find(|entry| entry.agent_id == "codex")
        .expect("codex registry entry");
    let release_watch = entry
        .maintenance
        .release_watch
        .as_ref()
        .expect("codex release watch");

    let mut urls = Vec::new();
    let page_one = format!(
        "[{}]",
        (0..100)
            .map(|patch| format!(
                r#"{{"tag_name":"rust-v0.98.{patch}","draft":false,"prerelease":false}}"#
            ))
            .collect::<Vec<_>>()
            .join(",")
    );
    let versions = watch::fetch_github_releases_with_fetcher(entry, release_watch, |url| {
        urls.push(url.to_string());
        if url.ends_with("page=1") {
            Ok(page_one.clone())
        } else if url.ends_with("page=2") {
            Ok(
                r#"[{"tag_name":"rust-v0.99.0","draft":false,"prerelease":false},{"tag_name":"rust-v0.99.1","draft":false,"prerelease":false}]"#
                    .to_string(),
            )
        } else {
            panic!("unexpected GitHub releases URL {url}");
        }
    })
    .expect("github pagination fetch succeeds");

    assert_eq!(urls.len(), 2);
    assert!(urls[0].ends_with("per_page=100&page=1"));
    assert!(urls[1].ends_with("per_page=100&page=2"));
    assert_eq!(versions.len(), 102);
    assert!(versions.contains(&"0.98.0".parse().unwrap()));
    assert!(versions.contains(&"0.99.1".parse().unwrap()));
}

fn resolver_for_queue(
    entry: &xtask::agent_registry::AgentRegistryEntry,
    _release_watch: &xtask::agent_registry::ReleaseWatchMetadata,
) -> Result<Vec<semver::Version>, Error> {
    let versions = match entry.agent_id.as_str() {
        "codex" => vec!["0.99.0", "0.98.0", "0.97.0"],
        "claude_code" => vec!["1.2.5"],
        "opencode" => vec!["1.4.12", "1.4.11", "1.4.9"],
        other => panic!("unexpected agent {other}"),
    };
    Ok(versions
        .into_iter()
        .map(|value| value.parse().expect("valid semver"))
        .collect())
}

fn seed_registry(root: &std::path::Path) {
    seed_registry_with(root, SEEDED_REGISTRY);
}

fn seed_registry_with(root: &std::path::Path, registry: &str) {
    write_text(
        &root.join("crates/xtask/data/agent_registry.toml"),
        registry,
    );
}

fn seed_latest_validated(root: &std::path::Path, manifest_root: &str, version: &str) {
    write_text(
        &root.join(manifest_root).join("latest_validated.txt"),
        &format!("{version}\n"),
    );
}
