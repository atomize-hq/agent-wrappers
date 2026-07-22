#![allow(dead_code, unused_imports)]

use std::{fs, path::Path};

use sha2::{Digest, Sha256};

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
#[path = "../src/agent_maintenance/docs.rs"]
mod docs;
#[path = "../src/agent_maintenance/prepare.rs"]
mod prepare;
#[path = "../src/agent_maintenance/request.rs"]
mod request;
#[path = "../src/agent_maintenance/support_audit.rs"]
mod support_audit;
#[path = "../src/workspace_mutation.rs"]
mod workspace_mutation;

use harness::{fixture_root, write_text};
use prepare::{apply_prepare_plan, build_prepare_plan, Args};
use request::load_request_envelope;

const SEEDED_REGISTRY: &str = include_str!("../data/agent_registry.toml");

#[test]
fn prepare_agent_maintenance_builds_packet_first_plan() {
    let fixture = fixture_root("prepare-agent-maintenance-plan");
    seed_registry(&fixture);
    seed_support_files(&fixture);

    let plan = build_prepare_plan(&fixture, &args()).expect("build plan");
    assert_eq!(
        plan.request.relative_path,
        "docs/agents/lifecycle/codex-maintenance/governance/maintenance-request.toml"
    );
    let request_text = String::from_utf8(plan.files[0].contents.clone()).expect("utf8");
    assert!(request_text.contains("artifact_version = \"2\""));
    assert!(request_text.contains("trigger_kind = \"upstream_release_detected\""));
    assert!(request_text.contains("branch_name = \"automation/codex-maintenance-0.98.0\""));
    assert!(request_text.contains("[execution_contract]"));
    assert!(request_text.contains("executor = \"execute-agent-maintenance\""));
    assert!(request_text.contains("dispatch_kind = \"packet_pr\""));
    assert!(request_text.contains("dispatch_workflow = \"agent-maintenance-open-pr.yml\""));
    assert!(request_text.contains(
        "prompt_template_path = \"docs/agents/lifecycle/codex-maintenance/governance/execute-agent-maintenance-prompt.md\""
    ));
    assert!(request_text.contains("\"crates/agent_api/src/runtime_support_data.rs\""));
    assert!(request_text.contains("[support_surface_audit]"));
    assert!(request_text.contains(
        "pr_summary_path = \"docs/agents/lifecycle/codex-maintenance/governance/pr-summary.md\""
    ));
    assert!(request_text.contains(
        "recreate_packet_command = \"cargo run -p xtask -- refresh-agent --request docs/agents/lifecycle/codex-maintenance/governance/maintenance-request.toml --write\""
    ));
    assert!(plan
        .planned_paths()
        .contains(&"docs/agents/lifecycle/codex-maintenance/HANDOFF.md"));
    assert!(plan
        .planned_paths()
        .contains(&"docs/agents/lifecycle/codex-maintenance/governance/pr-summary.md"));
}

#[test]
fn prepare_agent_maintenance_write_creates_packet_root() {
    let fixture = fixture_root("prepare-agent-maintenance-write");
    seed_registry(&fixture);
    seed_support_files(&fixture);

    let plan = build_prepare_plan(&fixture, &args()).expect("build plan");
    let summary = apply_prepare_plan(&fixture, &plan).expect("apply plan");
    assert_eq!(summary.total, plan.files.len());

    let handoff =
        fs::read_to_string(fixture.join("docs/agents/lifecycle/codex-maintenance/HANDOFF.md"))
            .expect("read handoff");
    assert!(handoff.contains(
        "<!-- generated-by: xtask agent-maintenance renderer; source-of-truth: governance/maintenance-request.toml -->"
    ));
    assert!(handoff.contains("detected_by"));
    assert!(handoff.contains("automation/codex-maintenance-0.98.0"));
    assert!(handoff.contains("This file is the canonical contributor execution contract"));
    assert!(handoff.contains("## Relay contract"));
    assert!(handoff.contains("maintained agent packet: `codex`"));
    assert!(handoff
        .contains("local execution host: `local Codex CLI host via execute-agent-maintenance`"));
    assert!(handoff.contains("## Writable surfaces"));
    assert!(handoff.contains("## Read-only inputs"));
    assert!(handoff.contains("## Ordered repo commands"));
    assert!(handoff.contains("## Exact green gates"));
    assert!(handoff.contains("## Recovery"));
    assert!(handoff.contains("## Exact closeout command"));
    assert!(handoff.contains("## Exact maintained-agent prompt"));
    assert!(
        handoff.contains("Execute the automated maintenance packet for `codex` target `0.98.0`.")
    );
    assert!(handoff.contains("docs/agents/lifecycle/codex-maintenance/**"));
    assert!(handoff.contains("crates/agent_api/**"));
    assert!(handoff.contains("cli_manifests/support_matrix/current.json"));
    assert!(handoff.contains("docs/specs/unified-agent-api/support-matrix.md"));
    assert!(handoff.contains("crates/agent_api/src/runtime_support_data.rs"));
    assert!(handoff.contains("docs/specs/codex-wrapper-coverage-scenarios-v1.md"));
    assert!(handoff.contains("docs/agents/lifecycle/codex-maintenance/OPS_PLAYBOOK.md"));
    assert!(handoff.contains("docs/agents/lifecycle/codex-maintenance/CI_WORKFLOWS_PLAN.md"));
    assert!(handoff.contains(
        "docs/agents/lifecycle/codex-maintenance/governance/execute-agent-maintenance-prompt.md"
    ));
    assert!(handoff.contains(".github/workflows/agent-maintenance-open-pr.yml"));
    assert!(handoff.contains("## Support-surface audit"));
    assert!(handoff.contains("pre-run debt count: `0`"));
    assert!(handoff.contains("cargo run -p xtask -- capability-matrix-audit"));
    assert!(handoff.contains(
        "cargo run -p xtask -- refresh-agent --request docs/agents/lifecycle/codex-maintenance/governance/maintenance-request.toml --write"
    ));
    assert!(handoff.contains(
        "If the local execution-host preflight (local Codex CLI host via execute-agent-maintenance) fails, fix the Codex binary/auth state and rerun `execute-agent-maintenance --dry-run` before write mode."
    ));
    assert!(!handoff.contains("## Explicit exclusions"));

    let pr_summary = fs::read_to_string(
        fixture.join("docs/agents/lifecycle/codex-maintenance/governance/pr-summary.md"),
    )
    .expect("read pr summary");
    assert!(pr_summary.contains("Automated maintenance packet for `codex` target `0.98.0`."));
    assert!(pr_summary
        .contains("Execute the automated maintenance packet for `codex` target `0.98.0`."));
    assert!(pr_summary.contains("docs/agents/lifecycle/codex-maintenance/HANDOFF.md"));
    assert!(pr_summary.contains("prompt sha256"));
    assert!(pr_summary.contains("## Exact maintained-agent prompt"));
    assert!(pr_summary.contains("## Support-surface audit"));
    assert!(!pr_summary.contains("## Work Queue Summary (autogenerated)"));
}

#[test]
fn prepare_agent_maintenance_defaults_to_generic_open_pr_workflow_for_enrolled_agents() {
    let fixture = fixture_root("prepare-agent-maintenance-packet-pr");
    seed_registry(&fixture);
    seed_support_files(&fixture);

    let plan = build_prepare_plan(&fixture, &args()).expect("build plan");
    let request_text = String::from_utf8(plan.files[0].contents.clone()).expect("utf8");
    assert!(request_text.contains("dispatch_kind = \"packet_pr\""));
    assert!(request_text.contains("dispatch_workflow = \"agent-maintenance-open-pr.yml\""));
    assert!(request_text.contains(
        "prompt_template_path = \"docs/agents/lifecycle/codex-maintenance/governance/execute-agent-maintenance-prompt.md\""
    ));
    assert!(request_text.contains("\"docs/agents/lifecycle/codex-maintenance/OPS_PLAYBOOK.md\""));
    assert!(
        request_text.contains("\"docs/agents/lifecycle/codex-maintenance/CI_WORKFLOWS_PLAN.md\"")
    );
    assert!(plan
        .planned_paths()
        .contains(&"docs/agents/lifecycle/codex-maintenance/OPS_PLAYBOOK.md"));
    assert!(plan
        .planned_paths()
        .contains(&"docs/agents/lifecycle/codex-maintenance/CI_WORKFLOWS_PLAN.md"));
    assert!(plan.planned_paths().contains(
        &"docs/agents/lifecycle/codex-maintenance/governance/execute-agent-maintenance-prompt.md"
    ));
}

#[test]
fn shared_renderer_keeps_handoff_pr_summary_and_prompt_in_lockstep() {
    let fixture = fixture_root("prepare-agent-maintenance-shared-renderer");
    seed_registry(&fixture);
    seed_support_files(&fixture);

    let request_path =
        "docs/agents/lifecycle/codex-maintenance/governance/maintenance-request.toml";
    write_text(
        &fixture.join(request_path),
        &automated_request_with_execution_contract_toml(),
    );

    let envelope =
        load_request_envelope(&fixture, Path::new(request_path)).expect("load request envelope");
    let contract = envelope
        .require_execution_contract_for_relay()
        .expect("execution contract")
        .clone();
    let rendered_packet = docs::render_execution_packet(&fixture, &envelope.request, &contract)
        .expect("render execution packet");
    let planned_docs =
        docs::build_packet_docs_from_envelope(&fixture, &envelope).expect("build packet docs");

    assert_eq!(
        rendered_packet.pr_summary_relative_path,
        contract.pr_summary_path
    );
    assert_eq!(
        rendered_packet.prompt_sha256, contract.prompt_sha256,
        "renderer must preserve the exact request-truth prompt digest"
    );

    let prompt_hash = hex::encode(Sha256::digest(rendered_packet.prompt_contents.as_bytes()));
    assert_eq!(prompt_hash, contract.prompt_sha256);
    assert!(rendered_packet.handoff_contents.contains("## Recovery"));
    assert!(rendered_packet
        .handoff_contents
        .contains("docs/agents/lifecycle/codex-maintenance/**"));
    assert!(rendered_packet
        .handoff_contents
        .contains("cargo run -p xtask -- codex-validate --root cli_manifests/codex"));
    assert!(rendered_packet
        .handoff_contents
        .contains("automation/codex-maintenance-0.98.0"));
    assert!(rendered_packet
        .handoff_contents
        .contains("## Dry-run to write relay"));
    assert!(rendered_packet.handoff_contents.contains(
        "cargo run -p xtask -- execute-agent-maintenance --dry-run --request docs/agents/lifecycle/codex-maintenance/governance/maintenance-request.toml"
    ));
    assert!(rendered_packet.handoff_contents.contains(
        "cargo run -p xtask -- execute-agent-maintenance --write --request docs/agents/lifecycle/codex-maintenance/governance/maintenance-request.toml --run-id <run-id-from-dry-run>"
    ));
    assert!(rendered_packet
        .handoff_contents
        .contains(&rendered_packet.prompt_contents));
    assert!(rendered_packet
        .pr_summary_contents
        .contains(&rendered_packet.handoff_relative_path));
    assert!(rendered_packet
        .pr_summary_contents
        .contains(&rendered_packet.prompt_contents));

    let handoff_doc = planned_docs
        .iter()
        .find(|doc| doc.relative_path == rendered_packet.handoff_relative_path)
        .expect("handoff doc");
    let pr_summary_doc = planned_docs
        .iter()
        .find(|doc| doc.relative_path == rendered_packet.pr_summary_relative_path)
        .expect("pr summary doc");
    assert_eq!(handoff_doc.contents, rendered_packet.handoff_contents);
    assert_eq!(pr_summary_doc.contents, rendered_packet.pr_summary_contents);
}

#[test]
fn shared_renderer_fails_closed_on_digest_and_root_mismatch() {
    let fixture = fixture_root("prepare-agent-maintenance-renderer-fail-closed");
    seed_registry(&fixture);
    seed_support_files(&fixture);

    let request_path =
        "docs/agents/lifecycle/codex-maintenance/governance/maintenance-request.toml";
    write_text(
        &fixture.join(request_path),
        &automated_request_with_execution_contract_toml(),
    );

    let envelope =
        load_request_envelope(&fixture, Path::new(request_path)).expect("load request envelope");
    let contract = envelope
        .require_execution_contract_for_relay()
        .expect("execution contract")
        .clone();

    let mut bad_digest = contract.clone();
    bad_digest.prompt_sha256 = "0".repeat(64);
    let digest_err = docs::render_execution_packet(&fixture, &envelope.request, &bad_digest)
        .expect_err("prompt digest mismatch should fail closed");
    assert!(digest_err.contains("prompt digest mismatch"));

    let mut wrong_root = contract;
    wrong_root.pr_summary_path =
        "docs/agents/lifecycle/other-maintenance/governance/pr-summary.md".to_string();
    let root_err = docs::render_execution_packet(&fixture, &envelope.request, &wrong_root)
        .expect_err("maintenance root mismatch should fail closed");
    assert!(root_err.contains("pr-summary path mismatch"));
}

fn args() -> Args {
    Args {
        agent: "codex".to_string(),
        current_version: "0.97.0".to_string(),
        latest_stable: "0.99.0".to_string(),
        target_version: "0.98.0".to_string(),
        opened_from: Path::new(".github/workflows/agent-maintenance-open-pr.yml").to_path_buf(),
        detected_by: ".github/workflows/agent-maintenance-release-watch.yml".to_string(),
        dispatch_kind: "packet_pr".to_string(),
        dispatch_workflow: None,
        branch_name: "automation/codex-maintenance-0.98.0".to_string(),
        request_recorded_at: "2026-05-05T15:00:00Z".to_string(),
        request_commit: "abcdef1".to_string(),
        dry_run: true,
        write: false,
    }
}

fn seed_registry(root: &Path) {
    seed_registry_with(root, SEEDED_REGISTRY);
}

fn seed_registry_with(root: &Path, registry: &str) {
    write_text(
        &root.join("crates/xtask/data/agent_registry.toml"),
        registry,
    );
}

fn seed_support_files(root: &Path) {
    let registry = agent_registry::AgentRegistry::parse(SEEDED_REGISTRY).expect("parse registry");
    let entry = registry.find("codex").expect("codex entry");
    write_text(
        &root.join(".github/workflows/agent-maintenance-open-pr.yml"),
        "name: Packet PR worker\n",
    );
    write_text(
        &root.join("cli_manifests/codex/PR_BODY_TEMPLATE.md"),
        "@codex\n\n## Goal\n\nFollow the maintained PR template for {{VERSION}}.\n",
    );
    write_text(
        &root.join("cli_manifests/codex/OPS_PLAYBOOK.md"),
        "# Codex ops\n",
    );
    write_text(
        &root.join("cli_manifests/codex/CI_WORKFLOWS_PLAN.md"),
        "# Codex CI workflows\n",
    );
    write_text(
        &root.join("docs/agents/lifecycle/codex-maintenance/governance/execute-agent-maintenance-prompt.md"),
        &contract_policy::packet_pr_prompt_template(entry, "docs/agents/lifecycle/codex-maintenance"),
    );
    write_text(
        &root.join("docs/agents/lifecycle/codex-maintenance/OPS_PLAYBOOK.md"),
        "# Packet ops\n",
    );
    write_text(
        &root.join("docs/agents/lifecycle/codex-maintenance/CI_WORKFLOWS_PLAN.md"),
        "# Packet workflow plan\n",
    );
    write_text(
        &root.join("cli_manifests/codex/latest_validated.txt"),
        "0.97.0\n",
    );
    write_text(
        &root.join("cli_manifests/codex/reports/0.98.0/coverage.any.json"),
        "{\n  \"deltas\": {\n    \"missing_commands\": [],\n    \"missing_flags\": [],\n    \"missing_args\": [],\n    \"intentionally_unsupported\": []\n  }\n}\n",
    );
    write_text(
        &root.join("docs/specs/unified-agent-api/non-tui-support-debt.md"),
        "# Non-TUI Support Debt Inventory\n\n## Inventory\n",
    );
}

fn automated_request_with_execution_contract_toml() -> String {
    let registry = agent_registry::AgentRegistry::parse(SEEDED_REGISTRY).expect("parse registry");
    let entry = registry.find("codex").expect("codex registry entry");
    let prompt = contract_policy::packet_pr_prompt_template(
        entry,
        "docs/agents/lifecycle/codex-maintenance",
    )
    .replace("{{VERSION}}", "0.98.0");
    let prompt_sha256 = hex::encode(Sha256::digest(prompt.as_bytes()));

    format!(
        concat!(
            "artifact_version = \"2\"\n",
            "agent_id = \"codex\"\n",
            "trigger_kind = \"upstream_release_detected\"\n",
            "basis_ref = \"cli_manifests/codex/latest_validated.txt\"\n",
            "opened_from = \".github/workflows/agent-maintenance-open-pr.yml\"\n",
            "requested_control_plane_actions = [\n",
            "  \"packet_doc_refresh\",\n",
            "]\n",
            "request_recorded_at = \"2026-05-05T15:00:00Z\"\n",
            "request_commit = \"abcdef1\"\n",
            "\n",
            "[runtime_followup_required]\n",
            "required = false\n",
            "items = []\n",
            "\n",
            "[detected_release]\n",
            "detected_by = \".github/workflows/agent-maintenance-release-watch.yml\"\n",
            "current_validated = \"0.97.0\"\n",
            "target_version = \"0.98.0\"\n",
            "latest_stable = \"0.99.0\"\n",
            "version_policy = \"latest_stable_minus_one\"\n",
            "source_kind = \"github_releases\"\n",
            "source_ref = \"openai/codex\"\n",
            "dispatch_kind = \"packet_pr\"\n",
            "dispatch_workflow = \"agent-maintenance-open-pr.yml\"\n",
            "branch_name = \"automation/codex-maintenance-0.98.0\"\n",
            "\n",
            "[support_surface_audit]\n",
            "required = true\n",
            "surface_kinds = [\n",
            "  \"commands\",\n",
            "  \"subcommands\",\n",
            "  \"flags\",\n",
            "  \"global_flags\",\n",
            "  \"positional_args\",\n",
            "]\n",
            "excluded_surface_kinds = [\n",
            "  \"tui_only\",\n",
            "]\n",
            "allowed_deferrals = [\n",
            "  \"upstream_not_machine_exposed\",\n",
            "  \"platform_evidence_missing\",\n",
            "  \"requires_new_infra\",\n",
            "  \"requires_new_architectural_seam\",\n",
            "  \"outside_registry_maintenance_write_envelope\",\n",
            "]\n",
            "pre_run_debt_count = 0\n",
            "expected_post_run_debt_count = 0\n",
            "\n",
            "[execution_contract]\n",
            "executor = \"execute-agent-maintenance\"\n",
            "prompt_template_path = \"docs/agents/lifecycle/codex-maintenance/governance/execute-agent-maintenance-prompt.md\"\n",
            "prompt_sha256 = \"{prompt_sha256}\"\n",
            "pr_summary_path = \"docs/agents/lifecycle/codex-maintenance/governance/pr-summary.md\"\n",
            "closeout_path = \"docs/agents/lifecycle/codex-maintenance/governance/maintenance-closeout.json\"\n",
            "requires_manual_closeout = true\n",
            "writable_surfaces = [\n",
            "  \"docs/agents/lifecycle/codex-maintenance/**\",\n",
            "  \"crates/codex/**\",\n",
            "  \"crates/agent_api/**\",\n",
            "  \"cli_manifests/codex/artifacts.lock.json\",\n",
            "  \"cli_manifests/codex/snapshots/0.98.0/**\",\n",
            "  \"cli_manifests/codex/reports/0.98.0/**\",\n",
            "  \"cli_manifests/codex/versions/0.98.0.json\",\n",
            "  \"cli_manifests/codex/wrapper_coverage.json\",\n",
            "  \"cli_manifests/support_matrix/current.json\",\n",
            "  \"docs/specs/unified-agent-api/support-matrix.md\",\n",
            "  \"crates/agent_api/src/runtime_support_data.rs\",\n",
            "  \"docs/specs/unified-agent-api/non-tui-support-debt.md\",\n",
            "  \"docs/specs/codex-wrapper-coverage-scenarios-v1.md\",\n",
            "]\n",
            "read_only_inputs = [\n",
            "  \"docs/agents/lifecycle/codex-maintenance/OPS_PLAYBOOK.md\",\n",
            "  \"docs/agents/lifecycle/codex-maintenance/CI_WORKFLOWS_PLAN.md\",\n",
            "  \"docs/agents/lifecycle/codex-maintenance/governance/execute-agent-maintenance-prompt.md\",\n",
            "  \".github/workflows/agent-maintenance-open-pr.yml\",\n",
            "  \"docs/specs/unified-agent-api/non-tui-support-debt.md\",\n",
            "]\n",
            "ordered_commands = [\n",
            "  \"cargo fmt --all\",\n",
            "  \"cargo run -p xtask -- codex-validate --root cli_manifests/codex\",\n",
            "  \"cargo run -p xtask -- support-matrix --check\",\n",
            "  \"cargo run -p xtask -- capability-matrix --check\",\n",
            "  \"cargo run -p xtask -- capability-matrix-audit\",\n",
            "  \"make preflight\",\n",
            "]\n",
            "green_gates = [\n",
            "  \"cargo fmt --all\",\n",
            "  \"cargo run -p xtask -- codex-validate --root cli_manifests/codex\",\n",
            "  \"cargo run -p xtask -- support-matrix --check\",\n",
            "  \"cargo run -p xtask -- capability-matrix --check\",\n",
            "  \"cargo run -p xtask -- capability-matrix-audit\",\n",
            "  \"make preflight\",\n",
            "]\n",
            "\n",
            "[execution_contract.recovery]\n",
            "recreate_packet_command = \"cargo run -p xtask -- refresh-agent --request docs/agents/lifecycle/codex-maintenance/governance/maintenance-request.toml --write\"\n",
            "reopen_pr_body_path = \"docs/agents/lifecycle/codex-maintenance/governance/pr-summary.md\"\n",
            "reopen_pr_branch = \"automation/codex-maintenance-0.98.0\"\n",
            "notes = [\n",
            "  \"If PR creation fails after packet generation, rerun packet regeneration from the frozen request and reopen the PR from the generated pr-summary path.\",\n",
            "  \"If the local execution-host preflight (local Codex CLI host via execute-agent-maintenance) fails, fix the Codex binary/auth state and rerun `execute-agent-maintenance --dry-run` before write mode.\",\n",
            "]\n"
        ),
        prompt_sha256 = prompt_sha256
    )
}
