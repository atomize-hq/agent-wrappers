use std::{fs, path::Path};

use clap::{CommandFactory, Parser};
use serde_json::Value;

mod maintenance_docs {
    pub use xtask::agent_maintenance::docs::*;
}
mod maintenance_request {
    pub use xtask::agent_maintenance::request::*;
}
mod release_doc {
    pub use xtask::release_doc::*;
}
mod support_matrix {
    pub use xtask::support_matrix::*;
}

#[path = "support/onboard_agent_harness.rs"]
mod harness;
#[path = "support/agent_maintenance_harness.rs"]
mod maintenance_harness;

use maintenance_harness::{
    execute_args, fake_execute_codex_binary, prepare_execute_fixture, read_json, run_execute_cli,
    snapshot_without_execute_runs, write_fake_execute_codex_preflight_scenario,
    write_fake_execute_codex_scenario, Cli, EXECUTE_RUNS_ROOT, EXECUTE_WRITE_RUN_ID,
    FAKE_EXECUTE_CODEX_LOG_FILE, GATE_ORDER_LOG_FILE,
};

#[test]
fn execute_agent_maintenance_help_text_includes_required_surface() {
    let top_help = Cli::command().render_help().to_string();
    assert!(top_help.contains("execute-agent-maintenance"));

    let err = Cli::try_parse_from(["xtask", "execute-agent-maintenance", "--help"])
        .expect_err("subcommand help should short-circuit parsing");
    assert_eq!(err.exit_code(), 0);
    let help_text = err.to_string();
    assert!(help_text.contains("--request"));
    assert!(help_text.contains("--dry-run"));
    assert!(help_text.contains("--write"));
    assert!(help_text.contains("--run-id"));
    assert!(help_text.contains("--codex-binary"));
}

#[test]
fn execute_agent_maintenance_dry_run_writes_frozen_packet_only_under_run_root() {
    let fixture = prepare_execute_fixture("agent-maintenance-execute-dry-run");
    let codex_binary = fake_execute_codex_binary(&fixture);
    let before = snapshot_without_execute_runs(&fixture);
    let output = run_execute_cli(execute_args("--dry-run", Some(&codex_binary)), &fixture);
    let after = snapshot_without_execute_runs(&fixture);

    assert_eq!(output.exit_code, 0, "stderr:\n{}", output.stderr);
    assert_eq!(before, after, "dry-run must only write the temp run packet");

    let run_dir = fixture.join(EXECUTE_RUNS_ROOT).join(EXECUTE_WRITE_RUN_ID);
    for name in [
        "input-contract.json",
        "codex-prompt.md",
        "run-status.json",
        "run-summary.md",
        "validation-report.json",
        "written-paths.json",
    ] {
        assert!(run_dir.join(name).is_file(), "missing {name}");
    }
    let prompt = fs::read_to_string(run_dir.join("codex-prompt.md")).expect("read prompt");
    assert!(
        prompt.contains("Execute the automated maintenance packet for `codex` target `0.98.0`.")
    );
    assert!(output.stdout.contains("closeout remains manual"));
}

#[test]
fn execute_agent_maintenance_dry_run_locks_relay_wording_and_distinction() {
    let fixture = prepare_execute_fixture("agent-maintenance-execute-relay-contract");
    let codex_binary = fake_execute_codex_binary(&fixture);
    let output = run_execute_cli(execute_args("--dry-run", Some(&codex_binary)), &fixture);

    assert_eq!(output.exit_code, 0, "stderr:\n{}", output.stderr);

    let run_dir = fixture.join(EXECUTE_RUNS_ROOT).join(EXECUTE_WRITE_RUN_ID);
    let input_contract = read_json(&run_dir.join("input-contract.json"));
    let recovery_notes = input_contract
        .get("recovery")
        .and_then(|recovery| recovery.get("notes"))
        .and_then(Value::as_array)
        .expect("recovery notes array")
        .iter()
        .map(|note| note.as_str().expect("note string"))
        .collect::<Vec<_>>();
    assert_eq!(
        recovery_notes,
        vec![
            "If PR creation fails after packet generation, rerun packet regeneration from the frozen request and reopen the PR from the generated pr-summary path.",
            "If the local execution-host preflight (local Codex CLI host via execute-agent-maintenance) fails, fix the Codex binary/auth state and rerun `execute-agent-maintenance --dry-run` before write mode.",
        ]
    );
    assert!(output.stdout.contains(
        "recreate_packet_command: cargo run -p xtask -- refresh-agent --request docs/agents/lifecycle/codex-maintenance/governance/maintenance-request.toml --write"
    ));

    let request_path =
        Path::new("docs/agents/lifecycle/codex-maintenance/governance/maintenance-request.toml");
    let envelope =
        maintenance_request::load_request_envelope(&fixture, request_path).expect("load request");
    let packet = maintenance_docs::build_packet_docs_from_envelope(&fixture, &envelope)
        .expect("render execution packet");
    let handoff = packet
        .iter()
        .find(|doc| doc.relative_path.ends_with("/HANDOFF.md"))
        .map(|doc| doc.contents.as_str())
        .expect("handoff contents");
    assert!(handoff.contains("maintained agent packet: `codex`"));
    assert!(handoff
        .contains("local execution host: `local Codex CLI host via execute-agent-maintenance`"));
    assert!(handoff.contains(
        "If PR creation fails after packet generation, rerun packet regeneration from the frozen request and reopen the PR from the generated pr-summary path."
    ));
    assert!(handoff.contains(
        "If the local execution-host preflight (local Codex CLI host via execute-agent-maintenance) fails, fix the Codex binary/auth state and rerun `execute-agent-maintenance --dry-run` before write mode."
    ));
    assert!(handoff.contains("## Dry-run to write relay"));
    assert!(handoff.contains(
        "cargo run -p xtask -- execute-agent-maintenance --dry-run --request docs/agents/lifecycle/codex-maintenance/governance/maintenance-request.toml"
    ));
    assert!(handoff.contains(
        "cargo run -p xtask -- execute-agent-maintenance --write --request docs/agents/lifecycle/codex-maintenance/governance/maintenance-request.toml --run-id RUN_ID_FROM_DRY_RUN"
    ));
}

#[test]
fn execute_agent_maintenance_write_requires_run_id() {
    let fixture = prepare_execute_fixture("agent-maintenance-execute-run-id");
    let codex_binary = fake_execute_codex_binary(&fixture);
    let output = run_execute_cli(
        [
            "xtask",
            "execute-agent-maintenance",
            "--write",
            "--request",
            "docs/agents/lifecycle/codex-maintenance/governance/maintenance-request.toml",
            "--codex-binary",
            codex_binary.to_string_lossy().as_ref(),
        ],
        &fixture,
    );

    assert_eq!(output.exit_code, 2);
    assert!(output.stderr.contains("--run-id is required"));
}

#[test]
fn execute_agent_maintenance_write_ignores_operator_governance_edits_but_still_fails_runtime_out_of_bounds_writes(
) {
    let ignored_fixture = prepare_execute_fixture("agent-maintenance-execute-operator-governance");
    let codex_binary = fake_execute_codex_binary(&ignored_fixture);
    let dry_run = run_execute_cli(
        execute_args("--dry-run", Some(&codex_binary)),
        &ignored_fixture,
    );
    assert_eq!(dry_run.exit_code, 0, "stderr:\n{}", dry_run.stderr);
    harness::write_text(
        &ignored_fixture.join(
            "docs/agents/lifecycle/codex-maintenance/governance/orchestration-friction-log.md",
        ),
        "# Friction log\n\n- operator note between dry-run and write\n",
    );
    write_fake_execute_codex_scenario(&ignored_fixture, "success");

    let ignored_output = run_execute_cli(
        execute_args("--write", Some(&codex_binary)),
        &ignored_fixture,
    );

    assert_eq!(
        ignored_output.exit_code, 0,
        "stderr:\n{}",
        ignored_output.stderr
    );
    let ignored_run_dir = ignored_fixture
        .join(EXECUTE_RUNS_ROOT)
        .join(EXECUTE_WRITE_RUN_ID);
    let ignored_written_paths: Vec<String> = serde_json::from_slice(
        &fs::read(ignored_run_dir.join("written-paths.json")).expect("read written paths"),
    )
    .expect("parse written paths");
    assert!(!ignored_written_paths
        .iter()
        .any(|path| path.ends_with("governance/orchestration-friction-log.md")));
    assert!(ignored_written_paths
        .iter()
        .any(|path| path == "docs/agents/lifecycle/codex-maintenance/runtime-note.md"));

    let violating_fixture =
        prepare_execute_fixture("agent-maintenance-execute-operator-governance-violation");
    let violating_codex_binary = fake_execute_codex_binary(&violating_fixture);
    let violating_dry_run = run_execute_cli(
        execute_args("--dry-run", Some(&violating_codex_binary)),
        &violating_fixture,
    );
    assert_eq!(
        violating_dry_run.exit_code, 0,
        "stderr:\n{}",
        violating_dry_run.stderr
    );
    harness::write_text(
        &violating_fixture.join(
            "docs/agents/lifecycle/codex-maintenance/governance/orchestration-friction-log.md",
        ),
        "# Friction log\n\n- operator note between dry-run and write\n",
    );
    write_fake_execute_codex_scenario(&violating_fixture, "out_of_bounds");

    let violating_output = run_execute_cli(
        execute_args("--write", Some(&violating_codex_binary)),
        &violating_fixture,
    );

    assert_eq!(violating_output.exit_code, 2);
    assert!(violating_output.stderr.contains("write boundary violation"));
    assert!(violating_output.stderr.contains("docs/unowned.md"));
    assert!(!violating_output
        .stderr
        .contains("orchestration-friction-log.md"));
}

#[test]
fn execute_agent_maintenance_write_rejects_out_of_bounds_paths() {
    let fixture = prepare_execute_fixture("agent-maintenance-execute-boundary");
    let codex_binary = fake_execute_codex_binary(&fixture);
    let dry_run = run_execute_cli(execute_args("--dry-run", Some(&codex_binary)), &fixture);
    assert_eq!(dry_run.exit_code, 0, "stderr:\n{}", dry_run.stderr);
    write_fake_execute_codex_scenario(&fixture, "out_of_bounds");

    let output = run_execute_cli(execute_args("--write", Some(&codex_binary)), &fixture);

    assert_eq!(output.exit_code, 2);
    assert!(output.stderr.contains("write boundary violation"));
    assert!(output.stderr.contains("docs/unowned.md"));
}

#[test]
fn execute_agent_maintenance_write_rejects_noop_runtime_execution() {
    let fixture = prepare_execute_fixture("agent-maintenance-execute-noop");
    let codex_binary = fake_execute_codex_binary(&fixture);
    let dry_run = run_execute_cli(execute_args("--dry-run", Some(&codex_binary)), &fixture);
    assert_eq!(dry_run.exit_code, 0, "stderr:\n{}", dry_run.stderr);
    write_fake_execute_codex_scenario(&fixture, "noop");

    let output = run_execute_cli(execute_args("--write", Some(&codex_binary)), &fixture);

    assert_eq!(output.exit_code, 2);
    assert!(output
        .stderr
        .contains("no runtime-owned output changes from the prepared baseline"));
}

#[test]
fn execute_agent_maintenance_dry_run_reports_execution_host_preflight_failures() {
    let fixture = prepare_execute_fixture("agent-maintenance-execute-preflight-fail");
    let codex_binary = fake_execute_codex_binary(&fixture);
    write_fake_execute_codex_preflight_scenario(&fixture, "preflight_fail");

    let output = run_execute_cli(execute_args("--dry-run", Some(&codex_binary)), &fixture);

    assert_eq!(output.exit_code, 2);
    assert!(output.stderr.contains(
        "local execution-host preflight failed; fix the Codex binary/auth state and rerun `execute-agent-maintenance --dry-run --request docs/agents/lifecycle/codex-maintenance/governance/maintenance-request.toml` before write mode"
    ));
    assert!(output.stderr.contains(
        "local execution-host preflight did not confirm readiness with `UAA_AGENT_MAINTENANCE_PREFLIGHT_OK`"
    ));

    let run_dir = fixture.join(EXECUTE_RUNS_ROOT).join(EXECUTE_WRITE_RUN_ID);
    let report = read_json(&run_dir.join("validation-report.json"));
    assert_eq!(report.get("status").and_then(Value::as_str), Some("fail"));
    assert_eq!(
        report
            .get("preflight")
            .and_then(|preflight| preflight.get("exit_code"))
            .and_then(Value::as_i64),
        Some(17)
    );
}

#[test]
fn execute_agent_maintenance_write_fails_closed_on_prompt_mismatch() {
    let fixture = prepare_execute_fixture("agent-maintenance-execute-prompt-mismatch");
    let codex_binary = fake_execute_codex_binary(&fixture);
    let dry_run = run_execute_cli(execute_args("--dry-run", Some(&codex_binary)), &fixture);
    assert_eq!(dry_run.exit_code, 0, "stderr:\n{}", dry_run.stderr);
    let request_path =
        fixture.join("docs/agents/lifecycle/codex-maintenance/governance/maintenance-request.toml");
    let request = fs::read_to_string(&request_path).expect("read request");
    harness::write_text(
        &request_path,
        &request.replace("prompt_sha256 = \"", "prompt_sha256 = \"0000"),
    );

    let output = run_execute_cli(execute_args("--write", Some(&codex_binary)), &fixture);

    assert_eq!(output.exit_code, 2);
    assert!(output.stderr.contains("prompt_sha256"));
}

#[test]
fn execute_agent_maintenance_write_reuses_prepared_baseline_runs_gates_and_keeps_closeout_manual() {
    let fixture = prepare_execute_fixture("agent-maintenance-execute-success");
    let codex_binary = fake_execute_codex_binary(&fixture);
    let dry_run = run_execute_cli(execute_args("--dry-run", Some(&codex_binary)), &fixture);
    assert_eq!(dry_run.exit_code, 0, "stderr:\n{}", dry_run.stderr);
    write_fake_execute_codex_scenario(&fixture, "success");

    let output = run_execute_cli(execute_args("--write", Some(&codex_binary)), &fixture);

    assert_eq!(output.exit_code, 0, "stderr:\n{}", output.stderr);
    assert!(output.stdout.contains("closeout remains manual"));
    let run_dir = fixture.join(EXECUTE_RUNS_ROOT).join(EXECUTE_WRITE_RUN_ID);
    let written_paths: Vec<String> = serde_json::from_slice(
        &fs::read(run_dir.join("written-paths.json")).expect("read written paths"),
    )
    .expect("parse written paths");
    assert!(written_paths
        .iter()
        .any(|path| { path == "docs/agents/lifecycle/codex-maintenance/runtime-note.md" }));
    assert!(written_paths
        .iter()
        .any(|path| path == "cli_manifests/codex/versions/0.98.0.json"));
    let gate_order = fs::read_to_string(run_dir.join(GATE_ORDER_LOG_FILE)).expect("read gate log");
    assert_eq!(gate_order, "gate-1\ngate-2\n");
    let invocation_log =
        fs::read_to_string(run_dir.join(FAKE_EXECUTE_CODEX_LOG_FILE)).expect("read invocation log");
    assert!(invocation_log.contains("--skip-git-repo-check"));
    assert!(invocation_log.contains("--cd"));
    assert!(!invocation_log.contains("--quiet"));
    assert!(!fixture
        .join("docs/agents/lifecycle/codex-maintenance/governance/maintenance-closeout.json")
        .exists());

    let execution = read_json(&run_dir.join("codex-execution.json"));
    assert_eq!(execution.get("exit_code").and_then(Value::as_i64), Some(0));
    let argv = execution
        .get("argv")
        .and_then(Value::as_array)
        .expect("argv array");
    assert_eq!(argv.first().and_then(Value::as_str), Some("exec"));
    assert!(!argv
        .iter()
        .filter_map(Value::as_str)
        .any(|arg| arg == "--quiet"));
    let report = read_json(&run_dir.join("validation-report.json"));
    assert_eq!(report.get("status").and_then(Value::as_str), Some("pass"));
}

#[test]
fn execute_agent_maintenance_write_ignores_generated_python_bytecode_caches() {
    let fixture = prepare_execute_fixture("agent-maintenance-execute-pyc");
    let codex_binary = fake_execute_codex_binary(&fixture);
    let dry_run = run_execute_cli(execute_args("--dry-run", Some(&codex_binary)), &fixture);
    assert_eq!(dry_run.exit_code, 0, "stderr:\n{}", dry_run.stderr);
    write_fake_execute_codex_scenario(&fixture, "success_with_pycache");

    let output = run_execute_cli(execute_args("--write", Some(&codex_binary)), &fixture);

    assert_eq!(output.exit_code, 0, "stderr:\n{}", output.stderr);
    let run_dir = fixture.join(EXECUTE_RUNS_ROOT).join(EXECUTE_WRITE_RUN_ID);
    let written_paths: Vec<String> = serde_json::from_slice(
        &fs::read(run_dir.join("written-paths.json")).expect("read written paths"),
    )
    .expect("parse written paths");
    assert!(!written_paths
        .iter()
        .any(|path| path.ends_with(".pyc") || path.contains("__pycache__")));
    let report = read_json(&run_dir.join("validation-report.json"));
    assert_eq!(report.get("status").and_then(Value::as_str), Some("pass"));
}

#[test]
fn execute_agent_maintenance_write_fails_when_support_surface_audit_goes_stale_after_gates() {
    let fixture = prepare_execute_fixture("agent-maintenance-execute-support-audit-stale");
    let request_path =
        fixture.join("docs/agents/lifecycle/codex-maintenance/governance/maintenance-request.toml");
    let request_text = fs::read_to_string(&request_path).expect("read request");
    harness::write_text(
        &request_path,
        &request_text.replace(
            "  \"cli_manifests/codex/versions/0.98.0.json\",\n",
            "  \"cli_manifests/codex/versions/0.98.0.json\",\n  \"cli_manifests/codex/reports/0.98.0/**\",\n",
        ),
    );
    harness::write_text(
        &fixture.join("gate-command.sh"),
        "#!/usr/bin/env sh\nset -eu\nlabel=\"$1\"\nlog_path=\"$2\"\nmkdir -p \"$(dirname \"$log_path\")\"\nprintf '%s\\n' \"$label\" >> \"$log_path\"\ncat > \"cli_manifests/codex/reports/0.98.0/coverage.any.json\" <<'EOF'\n{\n  \"deltas\": {\n    \"missing_commands\": [\n      {\n        \"path\": [\"status\"]\n      }\n    ],\n    \"missing_flags\": [],\n    \"missing_args\": [],\n    \"intentionally_unsupported\": []\n  }\n}\nEOF\n",
    );

    let codex_binary = fake_execute_codex_binary(&fixture);
    let dry_run = run_execute_cli(execute_args("--dry-run", Some(&codex_binary)), &fixture);
    assert_eq!(dry_run.exit_code, 0, "stderr:\n{}", dry_run.stderr);

    let output = run_execute_cli(execute_args("--write", Some(&codex_binary)), &fixture);

    assert_eq!(output.exit_code, 2);
    assert!(output
        .stderr
        .contains("support_surface_audit.discovered_upstream_surface added"));
    assert!(output
        .stderr
        .contains("surface_kind=commands command_path=codex status surface_id=status"));

    let run_dir = fixture.join(EXECUTE_RUNS_ROOT).join(EXECUTE_WRITE_RUN_ID);
    let report = read_json(&run_dir.join("validation-report.json"));
    assert_eq!(report.get("status").and_then(Value::as_str), Some("fail"));
    assert!(
        report
            .get("errors")
            .and_then(Value::as_array)
            .expect("errors array")
            .iter()
            .filter_map(Value::as_str)
            .any(|message| message
                .contains("support_surface_audit.discovered_upstream_surface added"))
    );
}

#[test]
fn execute_agent_maintenance_closeout_harness_keeps_claude_code_recovery_parity() {
    let harness_source = include_str!("support/agent_maintenance_closeout_harness.rs");

    assert!(
        harness_source.contains("branch_name = \\\"automation/{agent_id}-maintenance-1.14.47\\\"")
    );
    assert!(harness_source.contains("agent-maintenance-open-pr.yml"));
    assert!(harness_source
        .contains("prompt_template_path = \\\"docs/agents/lifecycle/{agent_id}-maintenance/governance/execute-agent-maintenance-prompt.md\\\""));
    assert!(harness_source.contains(
        "pr_summary_path = \\\"docs/agents/lifecycle/{agent_id}-maintenance/governance/pr-summary.md\\\""
    ));
    assert!(harness_source.contains(
        "If PR creation fails after packet generation, rerun packet regeneration from the frozen request and reopen the PR from the generated pr-summary path."
    ));
    assert!(harness_source.contains(
        "If the local execution-host preflight (local Codex CLI host via execute-agent-maintenance) fails, fix the Codex binary/auth state and rerun `execute-agent-maintenance --dry-run` before write mode."
    ));
}
