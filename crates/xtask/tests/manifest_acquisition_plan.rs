//! Contract tests for the agent-agnostic acquisition planner.
//!
//! Two layers are covered: the committed descriptors for every enrolled union-model agent must
//! resolve into a complete, correctly-ordered matrix, and the descriptor schema must reject the
//! malformed shapes that would otherwise only surface as a failed CI job on a real runner.

use std::fs;
use std::path::{Path, PathBuf};

use xtask::manifest_acquisition::{plan_for_agent, AcquisitionError};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("CARGO_MANIFEST_DIR has crates/<crate> parent structure")
        .to_path_buf()
}

fn expected_targets(agent: &str) -> Vec<String> {
    let rules: serde_json::Value = serde_json::from_slice(
        &fs::read(
            repo_root()
                .join("cli_manifests")
                .join(agent)
                .join("RULES.json"),
        )
        .expect("read RULES.json"),
    )
    .expect("parse RULES.json");
    rules["union"]["expected_targets"]
        .as_array()
        .expect("expected_targets array")
        .iter()
        .map(|v| v.as_str().expect("target string").to_string())
        .collect()
}

#[test]
fn every_enrolled_union_model_agent_plans_a_complete_matrix() {
    for agent in ["codex", "claude_code", "opencode"] {
        let plan = plan_for_agent(&repo_root(), agent, "1.2.3")
            .unwrap_or_else(|err| panic!("{agent} must resolve an acquisition plan: {err}"));

        let expected = expected_targets(agent);
        assert_eq!(
            plan.target_triples(),
            expected.iter().map(String::as_str).collect::<Vec<_>>(),
            "{agent}: plan matrix must cover union.expected_targets in the same order"
        );
        assert!(
            expected.contains(&plan.required_target),
            "{agent}: required_target must be part of the planned matrix"
        );

        for target in &plan.include {
            assert!(
                target.download_url.starts_with("https://"),
                "{agent}/{}: download URL must be https",
                target.target_triple
            );
            assert!(
                !target.download_url.contains('{'),
                "{agent}/{}: download URL must be fully resolved (got {})",
                target.target_triple,
                target.download_url
            );
            assert!(
                !target.binary_path.contains('{'),
                "{agent}/{}: binary path must be fully resolved",
                target.target_triple
            );
            for env in &target.validation_env {
                assert!(
                    !env.value.contains("{binary_path}"),
                    "{agent}/{}: validation env {} must have binary_path resolved",
                    target.target_triple,
                    env.name
                );
            }
        }
    }
}

#[test]
fn codex_resolves_github_release_assets_for_every_target() {
    let plan = plan_for_agent(&repo_root(), "codex", "0.145.0").expect("codex plan");
    assert_eq!(plan.source_kind, "github_releases");
    assert_eq!(plan.tag.as_deref(), Some("rust-v0.145.0"));
    assert_eq!(
        plan.release_metadata_url,
        "https://api.github.com/repos/openai/codex/releases/tags/rust-v0.145.0"
    );

    let linux = plan
        .include
        .iter()
        .find(|t| t.target_triple == "x86_64-unknown-linux-musl")
        .expect("required linux target");
    assert_eq!(
        linux.download_url,
        "https://github.com/openai/codex/releases/download/rust-v0.145.0/codex-x86_64-unknown-linux-musl.tar.gz"
    );
    assert_eq!(linux.archive, "tar_gz");

    let windows = plan
        .include
        .iter()
        .find(|t| t.target_triple == "x86_64-pc-windows-msvc")
        .expect("windows target");
    assert_eq!(windows.archive, "none", "the windows asset is a bare .exe");
    assert!(windows.binary_path.ends_with(".exe"));
}

#[test]
fn npm_agents_resolve_platform_package_tarballs() {
    let claude = plan_for_agent(&repo_root(), "claude_code", "2.1.219").expect("claude plan");
    assert_eq!(claude.source_kind, "npm");
    assert_eq!(
        claude.release_metadata_url,
        "https://registry.npmjs.org/@anthropic-ai/claude-code"
    );
    let linux = claude
        .include
        .iter()
        .find(|t| t.target_triple == "linux-x64")
        .expect("linux-x64");
    assert_eq!(
        linux.download_url,
        "https://registry.npmjs.org/@anthropic-ai/claude-code-linux-x64/-/claude-code-linux-x64-2.1.219.tgz",
        "scoped npm tarballs drop the scope from the filename only"
    );
    assert_eq!(linux.archive_member.as_deref(), Some("package/claude"));

    // opencode publishes `windows-x64`, not `win32-x64`: the descriptor owns that mapping so the
    // union target model never has to rename its committed targets.
    let opencode = plan_for_agent(&repo_root(), "opencode", "1.18.4").expect("opencode plan");
    let windows = opencode
        .include
        .iter()
        .find(|t| t.target_triple == "win32-x64")
        .expect("win32-x64");
    assert_eq!(
        windows.download_url,
        "https://registry.npmjs.org/opencode-windows-x64/-/opencode-windows-x64-1.18.4.tgz"
    );
    assert_eq!(
        windows.archive_member.as_deref(),
        Some("package/bin/opencode.exe")
    );
}

#[test]
fn acquisition_is_gated_on_release_watch_enrollment_and_a_descriptor() {
    let err = plan_for_agent(&repo_root(), "not-a-real-agent", "1.2.3")
        .expect_err("unknown agents must be rejected");
    assert!(
        matches!(err, AcquisitionError::UnknownAgent(_)),
        "got {err:?}"
    );

    // `gemini_cli` is registered but carries no `maintenance.release_watch` block.
    let err = plan_for_agent(&repo_root(), "gemini_cli", "1.2.3")
        .expect_err("unenrolled agents must not acquire");
    assert!(
        matches!(err, AcquisitionError::NotEnrolled(_)),
        "got {err:?}"
    );
}

#[test]
fn enrolled_agent_without_an_acquisition_block_is_reported_as_such() {
    let fixture = tempfile::tempdir().expect("tempdir");
    write_fixture_workspace(fixture.path(), &rules_without_acquisition());

    let err = plan_for_agent(fixture.path(), "fixture", "1.2.3")
        .expect_err("a missing descriptor must be a distinct, actionable error");
    assert!(
        matches!(err, AcquisitionError::NoAcquisitionBlock { .. }),
        "got {err:?}"
    );
}

#[test]
fn version_input_must_be_a_bare_semver() {
    for bad in ["v1.2.3", "1.2", "1.2.3-rc.1", "1.2.3+build", "", "latest"] {
        let Err(err) = plan_for_agent(&repo_root(), "codex", bad) else {
            panic!("version `{bad}` must be rejected");
        };
        assert!(
            matches!(err, AcquisitionError::InvalidVersion(_)),
            "version `{bad}` rejected for the wrong reason: {err:?}"
        );
    }
}

#[test]
fn descriptor_target_set_must_match_the_union_target_model() {
    let fixture = tempfile::tempdir().expect("tempdir");
    let mut rules = rules_with_acquisition();
    // Drop a target the union model expects.
    rules["acquisition"]["targets"]
        .as_object_mut()
        .expect("targets object")
        .remove("darwin-arm64");
    write_fixture_workspace(fixture.path(), &rules);

    let err = plan_for_agent(fixture.path(), "fixture", "1.2.3")
        .expect_err("an under-specified matrix must fail before it reaches a runner");
    let message = err.to_string();
    assert!(
        message.contains("missing union.expected_targets entries")
            && message.contains("darwin-arm64"),
        "got {message}"
    );
}

#[test]
fn npm_descriptor_rejects_github_only_target_fields() {
    let fixture = tempfile::tempdir().expect("tempdir");
    let mut rules = rules_with_acquisition();
    rules["acquisition"]["targets"]["linux-x64"]["asset_name"] =
        serde_json::Value::String("fixture-linux".into());
    write_fixture_workspace(fixture.path(), &rules);

    let err = plan_for_agent(fixture.path(), "fixture", "1.2.3")
        .expect_err("source-kind-foreign fields must be rejected");
    assert!(
        err.to_string().contains("asset_name") && err.to_string().contains("must not be set when"),
        "got {err}"
    );
}

#[test]
fn npm_target_requires_an_archive_member() {
    let fixture = tempfile::tempdir().expect("tempdir");
    let mut rules = rules_with_acquisition();
    rules["acquisition"]["targets"]["linux-x64"]
        .as_object_mut()
        .expect("target object")
        .remove("archive_member");
    write_fixture_workspace(fixture.path(), &rules);

    let err = plan_for_agent(fixture.path(), "fixture", "1.2.3")
        .expect_err("an npm tarball without a member path cannot be unpacked");
    assert!(err.to_string().contains("archive_member"), "got {err}");
}

// ---------------------------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------------------------

const FIXTURE_REGISTRY: &str = r#"
[[agents]]
agent_id = "fixture"
display_name = "Fixture Agent"
crate_path = "crates/fixture"
backend_module = "crates/agent_api/src/backends/fixture"
manifest_root = "cli_manifests/fixture"
package_name = "unified-agent-api-fixture"
canonical_targets = ["linux-x64"]

[agents.wrapper_coverage]
binding_kind = "generated_from_wrapper_crate"
source_path = "crates/fixture"

[agents.capability_declaration]
always_on = ["agent_api.run"]
backend_extensions = []

[agents.publication]
support_matrix_enabled = true
capability_matrix_enabled = true

[agents.release]
docs_release_track = "crates-io"

[agents.scaffold]
onboarding_pack_prefix = "fixture-onboarding"

[agents.maintenance]
[agents.maintenance.release_watch]
enabled = true
version_policy = "upstream_stable_pointer"
dispatch_kind = "packet_pr"

[agents.maintenance.release_watch.upstream]
source_kind = "npm_dist_tag"
package = "fixture"
dist_tag = "latest"
"#;

fn rules_without_acquisition() -> serde_json::Value {
    serde_json::json!({
        "rules_schema_version": 1,
        "union": {
            "tool_name": "fixture-cli",
            "raw_help_layout": "path_tokens",
            "required_target": "linux-x64",
            "expected_targets": ["linux-x64", "darwin-arm64"]
        }
    })
}

fn rules_with_acquisition() -> serde_json::Value {
    let mut rules = rules_without_acquisition();
    rules["acquisition"] = serde_json::json!({
        "acquisition_schema_version": 1,
        "source_kind": "npm",
        "npm": { "package": "fixture" },
        "snapshot": {
            "command": "fixture-snapshot",
            "binary_arg": "--fixture-binary",
            "extra_args": []
        },
        "lockfile_version_key": "semantic_version",
        "union_command": "manifest-union",
        "targets": {
            "linux-x64": {
                "runs_on": "ubuntu-latest",
                "binary_path": "./fixture-{target}",
                "archive": "npm_tgz",
                "platform_package": "fixture-linux-x64",
                "archive_member": "package/bin/fixture"
            },
            "darwin-arm64": {
                "runs_on": "macos-latest",
                "binary_path": "./fixture-{target}",
                "archive": "npm_tgz",
                "platform_package": "fixture-darwin-arm64",
                "archive_member": "package/bin/fixture"
            }
        },
        "validation": {
            "env": { "FIXTURE_BINARY": "{binary_path}" },
            "commands": ["cargo test -p unified-agent-api-fixture"]
        }
    });
    rules
}

fn write_fixture_workspace(root: &Path, rules: &serde_json::Value) {
    let registry_dir = root.join("crates/xtask/data");
    fs::create_dir_all(&registry_dir).expect("create registry dir");
    fs::write(registry_dir.join("agent_registry.toml"), FIXTURE_REGISTRY)
        .expect("write fixture registry");

    let manifest_dir = root.join("cli_manifests/fixture");
    fs::create_dir_all(&manifest_dir).expect("create manifest dir");
    fs::write(
        manifest_dir.join("RULES.json"),
        serde_json::to_string_pretty(rules).expect("render fixture rules"),
    )
    .expect("write fixture rules");
}

#[test]
fn every_descriptor_names_a_real_xtask_subcommand() {
    // A descriptor that names a nonexistent command would otherwise only fail after four runners
    // had each downloaded a release binary.
    for agent in ["codex", "claude_code", "opencode"] {
        let plan = plan_for_agent(&repo_root(), agent, "1.2.3").expect("plan");

        let mut commands = vec![plan.snapshot.command.clone(), plan.union_command.clone()];
        commands.extend(plan.wrapper_coverage_command.clone());

        for command in commands {
            let status = std::process::Command::new(env!("CARGO_BIN_EXE_xtask"))
                .args([command.as_str(), "--help"])
                .output()
                .expect("run xtask");
            assert!(
                status.status.success(),
                "{agent}: acquisition descriptor names `{command}`, which xtask does not provide"
            );
        }
    }
}

#[test]
fn every_enrolled_agent_carries_the_rules_blocks_its_engines_need() {
    // `manifest-union` and `manifest-report` deserialize these blocks; a manifest missing them
    // resolves a valid-looking plan and then dies mid-run.
    for agent in ["codex", "claude_code", "opencode"] {
        let rules: serde_json::Value = serde_json::from_slice(
            &fs::read(
                repo_root()
                    .join("cli_manifests")
                    .join(agent)
                    .join("RULES.json"),
            )
            .expect("read RULES.json"),
        )
        .expect("parse RULES.json");

        for block in [
            "union",
            "sorting",
            "report",
            "version_metadata",
            "acquisition",
        ] {
            assert!(
                rules.get(block).is_some(),
                "{agent}: RULES.json must carry the `{block}` block the shared engines read"
            );
        }
    }
}

#[test]
fn path_shaped_descriptor_fields_cannot_escape_the_workspace() {
    for (field, value) in [
        ("binary_path", "../../../etc/cron.d/pwn"),
        ("archive_member", "/etc/passwd"),
    ] {
        let fixture = tempfile::tempdir().expect("tempdir");
        let mut rules = rules_with_acquisition();
        rules["acquisition"]["targets"]["linux-x64"][field] =
            serde_json::Value::String(value.to_string());
        write_fixture_workspace(fixture.path(), &rules);

        let Err(err) = plan_for_agent(fixture.path(), "fixture", "1.2.3") else {
            panic!("{field}={value} must be rejected");
        };
        assert!(
            err.to_string().contains("workspace-relative path"),
            "{field}={value} rejected for the wrong reason: {err}"
        );
    }
}
