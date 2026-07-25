use std::fs;
use std::path::PathBuf;

use regex::Regex;

const GENERATED_PR_SUMMARY_SUFFIX: &str = "governance/pr-summary.md";

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("CARGO_MANIFEST_DIR has crates/<crate> parent structure")
        .to_path_buf()
}

fn read_repo_file(relative_path: &str) -> String {
    let path = repo_root().join(relative_path);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path:?}: {e}"))
}

#[test]
fn c4_spec_agent_maintenance_workflows_share_the_release_watch_and_packet_only_pr_contract() {
    let shared_watch = read_repo_file(".github/workflows/agent-maintenance-release-watch.yml");
    let packet_pr = read_repo_file(".github/workflows/agent-maintenance-open-pr.yml");

    assert!(
        shared_watch.contains(
            "cargo run -p xtask -- maintenance-watch --emit-json _ci_tmp/maintenance-watch.json"
        ),
        "shared watcher must delegate stale detection to xtask"
    );
    assert!(
        shared_watch.contains(".stale_agents[]"),
        "shared watcher must fan out from stale_agents queue data"
    );
    assert!(
        !shared_watch.contains("listReleases"),
        "workflow yaml must not reimplement stale detection"
    );
    for required in [
        "concurrency:",
        "group: agent-maintenance-release-watch",
        "cancel-in-progress: false",
    ] {
        assert!(
            shared_watch.contains(required),
            "shared watcher must retain workflow concurrency guard {required}"
        );
    }
    for legacy in [
        ".github/workflows/codex-cli-release-watch.yml",
        ".github/workflows/claude-code-release-watch.yml",
    ] {
        assert!(
            !repo_root().join(legacy).exists(),
            "legacy watcher must be deleted: {legacy}"
        );
    }

    for required in [
        "prepare-agent-maintenance",
        "--current-version",
        "--latest-stable",
        "--target-version",
        "--opened-from",
        "--detected-by",
        "--dispatch-kind",
        "--branch-name",
        "base: staging",
        "add-paths: ${{ inputs.add_paths }}",
        "body-path: docs/agents/lifecycle/${{ inputs.agent_id }}-maintenance/governance/pr-summary.md",
        "concurrency:",
        "group: agent-maintenance-${{ inputs.branch_name }}",
        "cancel-in-progress: false",
        "continue-on-error: true",
        "steps.create_pr.outcome == 'failure'",
        "If PR creation fails after packet generation, rerun packet regeneration from the frozen request and reopen the PR from the generated pr-summary path.",
        "cargo run -p xtask -- refresh-agent --request \"${REQUEST_PATH}\" --write",
        "gh pr create --base staging --head \"${{ inputs.branch_name }}\"",
    ] {
        assert!(
            packet_pr.contains(required),
            "packet PR workflow must retain {required}"
        );
    }
    assert!(
        !packet_pr.contains("\n          body:"),
        "packet PR workflow must not keep an inline body block"
    );
    assert_prepare_step_precedes(
        &packet_pr,
        "prepare-agent-maintenance",
        "body-path: docs/agents/lifecycle/${{ inputs.agent_id }}-maintenance/governance/pr-summary.md",
        ".github/workflows/agent-maintenance-open-pr.yml",
    );

    // The packet-opening job itself still performs no acquisition work: acquisition happens in a
    // separate job that delegates to the reusable lane.
    for forbidden in [
        "actions/download-artifact@v7",
        "codex-snapshot",
        "claude-snapshot",
        "prepare-publication",
        "refresh-publication",
        "artifacts.lock.json",
        "_ci_tmp/codex_cli_pr_body.md",
    ] {
        assert!(
            !packet_pr.contains(forbidden),
            "packet PR workflow must not inline acquisition/generation work: {forbidden}"
        );
    }
}

#[test]
fn c4_spec_packet_pr_delegates_a_complete_union_to_the_reusable_acquisition_lane() {
    let packet_pr = read_repo_file(".github/workflows/agent-maintenance-open-pr.yml");

    for required in [
        // The gate is committed truth (registry enrollment + an `acquisition` block), enforced by
        // the planner, rather than a second per-agent enablement field.
        "manifest-acquisition-plan",
        "acquire=true",
        "acquire=false",
        // A build failure and a planner rejection must stay distinguishable: collapsing both into
        // `acquire=false` would silently downgrade every agent to the docs-only lane.
        "cargo build -p xtask",
        "gate_status=$?",
        "uses: ./.github/workflows/parity-acquire.yml",
        "ref: ${{ inputs.branch_name }}",
        "commit: true",
    ] {
        assert!(
            packet_pr.contains(required),
            "packet PR workflow must route acquisition through the reusable lane: {required}"
        );
    }

    assert!(
        packet_pr.contains("needs.open-pr.outputs.acquire == 'true'")
            && packet_pr.contains("needs.open-pr.outputs.branch_created == 'true'"),
        "acquisition must be gated on both the descriptor gate and a branch that actually exists"
    );
}

#[test]
fn c4_spec_reusable_acquisition_is_agent_parameterized_with_no_per_agent_branching() {
    let yml = read_repo_file(".github/workflows/parity-acquire.yml");

    for required in [
        "workflow_call:",
        "workflow_dispatch:",
        "agent_id:",
        "target_version:",
        "cargo run -p xtask -- manifest-acquisition-plan",
        "--emit-json _ci_tmp/acquisition-plan.json",
        "fromJSON(needs.plan.outputs.snapshot_matrix)",
        "runs-on: ${{ matrix.runs_on }}",
        // Engine invocations are read from the plan, not hardcoded per agent.
        "UNION_COMMAND=",
        "SNAPSHOT_COMMAND=",
        "manifest-report --root",
        "manifest-version-metadata --root",
        "--status reported",
        "manifest-validate --root",
    ] {
        assert!(
            yml.contains(required),
            "reusable acquisition workflow must retain {required}"
        );
    }

    // Both acquisition source kinds and all three archive shapes must be handled generically.
    for required in ["github_releases)", "npm)", "none)", "tar_gz)", "npm_tgz)"] {
        assert!(
            yml.contains(required),
            "reusable acquisition workflow must handle {required}"
        );
    }

    // Integrity: pins are re-verified on the target runner before the binary is executed.
    assert!(
        yml.contains("sha256 mismatch") && yml.contains("size mismatch"),
        "reusable acquisition must re-verify the download pin on the snapshotting runner"
    );

    // No agent may be named in a branching construct: the whole point is one lane for all agents.
    for forbidden in [
        "cli_manifests/codex",
        "cli_manifests/claude_code",
        "cli_manifests/opencode",
        "openai/codex",
        "storage.googleapis",
        "codex-union",
        "claude-union",
        "codex-report",
        "codex-validate",
        "codex-version-metadata",
    ] {
        assert!(
            !yml.contains(forbidden),
            "reusable acquisition workflow must stay agent-agnostic: {forbidden}"
        );
    }
}

#[test]
fn c4_spec_reusable_promotion_validates_every_promoted_target_before_advancing_pointers() {
    let yml = read_repo_file(".github/workflows/parity-promote.yml");

    for required in [
        "workflow_call:",
        "workflow_dispatch:",
        "validation_matrix",
        "fromJSON(needs.plan.outputs.validation_matrix)",
        "runs-on: ${{ matrix.runs_on }}",
        "needs: [plan, validate-target]",
        // Only targets present in the committed union may be validated and promoted.
        "jq -r '.inputs[].target_triple'",
        "VALIDATION_ARGS+=(--passed-target \"$target\")",
        "--status validated",
        "pointers/latest_validated/${REQUIRED_TARGET}.txt",
        "manifest-validate --root",
        // Validation commands and env come from the descriptor, not from this file.
        "jq -r '.validation_commands[]'",
        "{scratch_dir}",
    ] {
        assert!(
            yml.contains(required),
            "reusable promotion workflow must retain {required}"
        );
    }

    assert!(
        yml.contains("default: true") && yml.contains("if: ${{ !inputs.dry_run }}"),
        "promotion must default to dry-run and only open a PR when explicitly asked"
    );

    for forbidden in [
        "cli_manifests/codex",
        "cli_manifests/claude_code",
        "cargo test -p unified-agent-api-codex",
        "cargo test -p unified-agent-api-claude-code",
        "x86_64-unknown-linux-musl",
    ] {
        assert!(
            !yml.contains(forbidden),
            "reusable promotion workflow must stay agent-agnostic: {forbidden}"
        );
    }
}

#[test]
fn c4_spec_legacy_per_agent_parity_workflows_are_retired() {
    for legacy in [
        ".github/workflows/codex-cli-update-snapshot.yml",
        ".github/workflows/claude-code-update-snapshot.yml",
        ".github/workflows/codex-cli-promote.yml",
        ".github/workflows/claude-code-promote.yml",
    ] {
        assert!(
            !repo_root().join(legacy).exists(),
            "legacy per-agent parity workflow must be deleted: {legacy}"
        );
    }

    let workflows = fs::read_dir(repo_root().join(".github/workflows"))
        .expect("read .github/workflows")
        .map(|entry| {
            entry
                .expect("dir entry")
                .file_name()
                .to_string_lossy()
                .to_string()
        })
        .collect::<Vec<_>>();

    for name in &workflows {
        assert!(
            !(name.contains("update-snapshot") || name.contains("promote"))
                || name.starts_with("parity-"),
            "acquisition/promotion must live only in the reusable parity-* lanes, found {name}"
        );
    }
}

#[test]
fn c4_spec_ci_workflow_has_conditional_manifest_validate_gate() {
    let yml = read_repo_file(".github/workflows/ci.yml");

    // C4-spec (normative): gate runs only when committed artifacts regime is active.
    //
    // Two supported implementations:
    // - job-level hashFiles gate
    // - a first step that detects committed versions and gates subsequent steps via outputs
    let has_hashfiles_gate = yml.contains("hashFiles('cli_manifests/codex/versions/*.json') != ''");
    let has_step_gate = yml.contains("Detect Codex committed artifacts")
        && yml.contains("has_versions")
        && yml.contains("steps.codex-artifacts.outputs.has_versions");
    assert!(
        has_hashfiles_gate || has_step_gate,
        "ci.yml must gate manifest validation behind either hashFiles('cli_manifests/codex/versions/*.json') != '' or a Detect Codex committed artifacts step gate"
    );

    // Ensure the job actually runs the validator (not just mentions it). Both the neutral name and
    // the retained back-compat alias are acceptable.
    let validate_invocation = Regex::new(
        r"cargo\s+run\s+-p\s+xtask\s+--[\s\\]*\n?[\s\\]*(manifest-validate|codex-validate)",
    )
    .expect("valid regex");
    assert!(
        validate_invocation.is_match(&yml),
        "ci.yml must invoke: cargo run -p xtask -- manifest-validate"
    );
}

#[test]
fn c4_spec_ci_pins_the_latest_validated_binary_from_the_lockfile_row_not_the_descriptor() {
    let yml = read_repo_file(".github/workflows/ci.yml");

    // The descriptor answers "how would a new version be acquired"; the lockfile row answers
    // "what was actually pinned for this one". Those diverge across a distribution migration:
    // claude_code 2.1.29 is pinned as a bare binary from the old bucket while the descriptor
    // resolves an npm platform tarball. Selecting the row by the descriptor's asset name matches
    // nothing and fails the job, so the row must be selected by (version, target) alone.
    let selects_row_by_version_and_target =
        yml.contains(r#"select(.[$key]==$v and .target_triple==$t)"#);
    assert!(
        selects_row_by_version_and_target,
        "ci.yml must select the claude_code lockfile row by (version, target) only"
    );

    assert!(
        !yml.contains(r#"select(.[$key]==$v and .target_triple==$t and .asset_name==$a)"#),
        "ci.yml must not constrain the lockfile row on the descriptor's asset name: a version \
         pinned before the acquisition descriptor existed has no row under that name"
    );

    // The pinned asset names its own shape. Inferring it from the descriptor's `archive` would
    // try to untar a bare binary (or execute a tarball) depending on migration direction.
    assert!(
        yml.contains("*.tgz|*.tar.gz)"),
        "ci.yml must infer archive shape from the pinned asset name"
    );

    // Whatever the shape, the binary must never be executed before the autoupdater is disabled:
    // a self-update between the digest check and the run would make the verified pin a lie.
    let acquire_step = yml
        .split("- name: Acquire Claude Code CLI (required target)")
        .nth(1)
        .expect("ci.yml must retain the claude_code acquisition step");
    let env_export = acquire_step
        .find(".snapshot.env[]?")
        .expect("acquisition step must apply the descriptor's snapshot env");
    let version_probe = acquire_step
        .find(r#""$BIN" --version"#)
        .expect("acquisition step must smoke the acquired binary");
    assert!(
        env_export < version_probe,
        "ci.yml must export the descriptor's snapshot env before executing the binary"
    );
}

fn assert_prepare_step_precedes(
    workflow_text: &str,
    prepare_needle: &str,
    body_path_needle: &str,
    workflow: &str,
) {
    let prepare_index = workflow_text
        .find(prepare_needle)
        .unwrap_or_else(|| panic!("{workflow} must contain {prepare_needle}"));
    let body_path_index = workflow_text
        .find(body_path_needle)
        .unwrap_or_else(|| panic!("{workflow} must contain {body_path_needle}"));
    assert!(
        prepare_index < body_path_index,
        "{workflow} must render the maintenance packet before referencing {GENERATED_PR_SUMMARY_SUFFIX}"
    );
}

#[test]
fn backend_type_leak_guard_is_centralized_in_ci_and_smoke_workflows() {
    let guard_invocation = Regex::new(
        r"cargo\s+run\s+-p\s+xtask\s+--[\s\\]*\n?[\s\\]*agent-api-backend-type-leak-guard",
    )
    .expect("valid regex");
    for workflow in [
        ".github/workflows/ci.yml",
        ".github/workflows/unified-agent-api-smoke.yml",
        ".github/workflows/agent-api-codex-stream-exec-smoke.yml",
        ".github/workflows/claude-code-live-stream-json-smoke.yml",
    ] {
        let yml = read_repo_file(workflow);
        assert!(
            guard_invocation.is_match(&yml),
            "{workflow} must invoke cargo run -p xtask -- agent-api-backend-type-leak-guard"
        );
        assert!(
            !yml.contains("(?:codex|claude_code)::"),
            "{workflow} must not keep the stale inline backend regex guard"
        );
    }
}
