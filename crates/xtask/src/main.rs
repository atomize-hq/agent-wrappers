#![forbid(unsafe_code)]

mod agent_api_backend_type_leak_guard;
mod capability_matrix_audit;
mod claude_snapshot;
mod claude_wrapper_coverage;
mod close_proving_run;
mod codex_snapshot;
mod codex_wrapper_coverage;
mod historical_lifecycle_backfill;
mod manifest_report;
mod manifest_retain;
mod manifest_snapshot_schema;
mod manifest_union;
mod manifest_validate;
mod manifest_version_metadata;
mod opencode_snapshot;
mod version_bump;
mod wrapper_coverage_shared;

/// Historical manifest root defaults preserved by the per-agent back-compat command aliases.
///
/// The neutral `manifest-*` commands require an explicit `--root`; these constants exist only so
/// pre-existing callers of `codex-union` / `claude-union` keep their original behavior.
const CODEX_MANIFEST_ROOT: &str = "cli_manifests/codex";
const CLAUDE_CODE_MANIFEST_ROOT: &str = "cli_manifests/claude_code";

use xtask::agent_maintenance::{
    closeout as agent_maintenance_closeout, drift as agent_maintenance_drift,
    execute as agent_maintenance_execute, prepare as agent_maintenance_prepare,
    refresh as agent_maintenance_refresh, watch as agent_maintenance_watch,
};
use xtask::capability_matrix;
pub use xtask::manifest_acquisition;
pub use xtask::onboard_agent;
pub use xtask::prepare_proving_run_closeout;
pub use xtask::prepare_publication;
pub use xtask::publication_refresh;
pub use xtask::recommend_next_agent_research;
pub use xtask::repair_runtime_evidence;
pub use xtask::runtime_follow_on;
pub use xtask::support_matrix;
pub use xtask::wrapper_scaffold;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "xtask")]
#[command(about = "Project automation tasks")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
#[allow(clippy::enum_variant_names)]
enum Command {
    /// Guard against backend crate types leaking into the public agent_api surface.
    AgentApiBackendTypeLeakGuard(agent_api_backend_type_leak_guard::Args),
    /// Generate a Codex CLI snapshot manifest under `cli_manifests/codex/`.
    CodexSnapshot(codex_snapshot::Args),
    /// Generate a Claude Code CLI snapshot manifest under `cli_manifests/claude_code/`.
    ClaudeSnapshot(claude_snapshot::Args),
    /// Generate an OpenCode CLI snapshot manifest under `cli_manifests/opencode/`.
    OpencodeSnapshot(opencode_snapshot::Args),
    /// Validate a proving-run closeout artifact and refresh the onboarding packet docs.
    CloseProvingRun(close_proving_run::Args),
    /// Resolve the multi-target acquisition plan for one agent at one upstream version.
    ManifestAcquisitionPlan(manifest_acquisition::Args),
    /// Merge per-target snapshots into a union snapshot under any `--root` manifest directory.
    ManifestUnion(manifest_union::Args),
    /// Back-compat alias for `manifest-union --root cli_manifests/codex`.
    CodexUnion(manifest_union::Args),
    /// Back-compat alias for `manifest-union --root cli_manifests/claude_code`.
    ClaudeUnion(manifest_union::Args),
    /// Generate deterministic coverage reports under `<root>/reports/<version>/`.
    ManifestReport(manifest_report::Args),
    /// Back-compat alias for `manifest-report --root cli_manifests/codex`.
    CodexReport(manifest_report::Args),
    /// Materialize `<root>/versions/<version>.json` deterministically.
    ManifestVersionMetadata(manifest_version_metadata::Args),
    /// Back-compat alias for `manifest-version-metadata --root cli_manifests/codex`.
    CodexVersionMetadata(manifest_version_metadata::Args),
    /// Deterministically prune out-of-window snapshots/reports directories (dry-run by default).
    ManifestRetain(manifest_retain::Args),
    /// Back-compat alias for `manifest-retain --root cli_manifests/codex`.
    CodexRetain(manifest_retain::Args),
    /// Generate `cli_manifests/codex/wrapper_coverage.json` from wrapper source of truth.
    CodexWrapperCoverage(codex_wrapper_coverage::CliArgs),
    /// Generate `cli_manifests/claude_code/wrapper_coverage.json` from wrapper source of truth.
    ClaudeWrapperCoverage(claude_wrapper_coverage::CliArgs),
    /// Validate committed parity artifacts under any `--root` manifest directory.
    ManifestValidate(manifest_validate::Args),
    /// Back-compat alias for `manifest-validate --root cli_manifests/codex`.
    CodexValidate(manifest_validate::Args),
    /// Preview the next control-plane onboarding packet without writing files.
    OnboardAgent(Box<onboard_agent::Args>),
    /// Create a publishable wrapper crate shell for an onboarded agent.
    ScaffoldWrapperCrate(wrapper_scaffold::Args),
    /// Prepare or validate the bounded runtime follow-on lane for an onboarded agent.
    RuntimeFollowOn(runtime_follow_on::Args),
    /// Prepare the committed publication handoff from runtime-integrated evidence.
    PreparePublication(prepare_publication::Args),
    /// Refresh publication-owned outputs from a committed publication-ready packet.
    RefreshPublication(publication_refresh::Args),
    /// Prepare or validate the bounded recommendation research lane.
    RecommendNextAgentResearch(recommend_next_agent_research::Args),
    /// Prepare the canonical proving-run closeout draft from published lifecycle truth.
    PrepareProvingRunCloseout(prepare_proving_run_closeout::Args),
    /// Repair a stale runtime evidence bundle without advancing lifecycle stage.
    RepairRuntimeEvidence(repair_runtime_evidence::Args),
    /// Generate or verify the universal agent capability matrix markdown.
    CapabilityMatrix(capability_matrix::Args),
    /// Audit the capability matrix for orthogonality invariants.
    CapabilityMatrixAudit(capability_matrix_audit::Args),
    /// Detect maintenance-relevant drift for an already-onboarded agent.
    CheckAgentDrift(agent_maintenance_drift::Args),
    /// Detect stale enrolled agents from registry truth and emit the maintenance queue.
    MaintenanceWatch(agent_maintenance_watch::Args),
    /// Prepare an automated maintenance request and packet docs from release-watch inputs.
    PrepareAgentMaintenance(agent_maintenance_prepare::Args),
    /// Execute the bounded contributor relay for an automated maintenance request.
    ExecuteAgentMaintenance(agent_maintenance_execute::Args),
    /// Refresh maintenance packet docs and generated publication surfaces from a maintenance request.
    RefreshAgent(agent_maintenance_refresh::Args),
    /// Validate and close an agent maintenance run.
    CloseAgentMaintenance(agent_maintenance_closeout::Args),
    /// Backfill truthful historical lifecycle maintenance artifacts for known malformed baselines.
    HistoricalLifecycleBackfill(historical_lifecycle_backfill::Args),
    /// Generate support publication JSON and Markdown outputs from committed manifest evidence.
    SupportMatrix(support_matrix::Args),
    /// Bump the workspace release version and exact inter-crate publish pins.
    VersionBump(version_bump::Args),
}

fn main() {
    let cli = Cli::parse();

    let exit_code = match cli.command {
        Command::AgentApiBackendTypeLeakGuard(args) => {
            match agent_api_backend_type_leak_guard::run(args) {
                Ok(()) => 0,
                Err(err) => {
                    eprintln!("{err}");
                    1
                }
            }
        }
        Command::CodexSnapshot(args) => match codex_snapshot::run(args) {
            Ok(()) => 0,
            Err(err) => {
                eprintln!("{err}");
                1
            }
        },
        Command::ClaudeSnapshot(args) => match claude_snapshot::run(args) {
            Ok(()) => 0,
            Err(err) => {
                eprintln!("{err}");
                1
            }
        },
        Command::OpencodeSnapshot(args) => match opencode_snapshot::run(args) {
            Ok(()) => 0,
            Err(err) => {
                eprintln!("{err}");
                1
            }
        },
        Command::CloseProvingRun(args) => match close_proving_run::run(args) {
            Ok(()) => 0,
            Err(err) => {
                eprintln!("{err}");
                err.exit_code()
            }
        },
        Command::ManifestAcquisitionPlan(args) => match manifest_acquisition::run(args) {
            Ok(()) => 0,
            Err(err) => {
                eprintln!("{err}");
                1
            }
        },
        Command::ManifestUnion(args) => match manifest_union::run(args) {
            Ok(()) => 0,
            Err(err) => {
                eprintln!("{err}");
                1
            }
        },
        Command::CodexUnion(args) => {
            match manifest_union::run_with_default_root(args, Some(CODEX_MANIFEST_ROOT)) {
                Ok(()) => 0,
                Err(err) => {
                    eprintln!("{err}");
                    1
                }
            }
        }
        Command::ClaudeUnion(args) => {
            match manifest_union::run_with_default_root(args, Some(CLAUDE_CODE_MANIFEST_ROOT)) {
                Ok(()) => 0,
                Err(err) => {
                    eprintln!("{err}");
                    1
                }
            }
        }
        Command::ManifestReport(args) => match manifest_report::run(args) {
            Ok(()) => 0,
            Err(err) => {
                eprintln!("{err}");
                1
            }
        },
        Command::CodexReport(args) => {
            match manifest_report::run_with_default_root(args, Some(CODEX_MANIFEST_ROOT)) {
                Ok(()) => 0,
                Err(err) => {
                    eprintln!("{err}");
                    1
                }
            }
        }
        Command::ManifestVersionMetadata(args) => match manifest_version_metadata::run(args) {
            Ok(()) => 0,
            Err(err) => {
                eprintln!("{err}");
                1
            }
        },
        Command::CodexVersionMetadata(args) => {
            match manifest_version_metadata::run_with_default_root(args, Some(CODEX_MANIFEST_ROOT))
            {
                Ok(()) => 0,
                Err(err) => {
                    eprintln!("{err}");
                    1
                }
            }
        }
        Command::ManifestRetain(args) => match manifest_retain::run(args) {
            Ok(()) => 0,
            Err(err) => {
                eprintln!("{err}");
                1
            }
        },
        Command::CodexRetain(args) => {
            match manifest_retain::run_with_default_root(args, Some(CODEX_MANIFEST_ROOT)) {
                Ok(()) => 0,
                Err(err) => {
                    eprintln!("{err}");
                    1
                }
            }
        }
        Command::CodexWrapperCoverage(args) => match codex_wrapper_coverage::run(args) {
            Ok(()) => 0,
            Err(err) => {
                eprintln!("{err}");
                1
            }
        },
        Command::ClaudeWrapperCoverage(args) => match claude_wrapper_coverage::run(args) {
            Ok(()) => 0,
            Err(err) => {
                eprintln!("{err}");
                1
            }
        },
        Command::ManifestValidate(args) => manifest_validate::run(args),
        Command::CodexValidate(args) => {
            manifest_validate::run_with_default_root(args, Some(CODEX_MANIFEST_ROOT))
        }
        Command::OnboardAgent(args) => match onboard_agent::run(*args) {
            Ok(()) => 0,
            Err(err) => {
                eprintln!("{err}");
                err.exit_code()
            }
        },
        Command::ScaffoldWrapperCrate(args) => match wrapper_scaffold::run(args) {
            Ok(()) => 0,
            Err(err) => {
                eprintln!("{err}");
                err.exit_code()
            }
        },
        Command::RuntimeFollowOn(args) => match runtime_follow_on::run(args) {
            Ok(()) => 0,
            Err(err) => {
                eprintln!("{err}");
                err.exit_code()
            }
        },
        Command::PreparePublication(args) => match prepare_publication::run(args) {
            Ok(()) => 0,
            Err(err) => {
                eprintln!("{err}");
                err.exit_code()
            }
        },
        Command::RefreshPublication(args) => match publication_refresh::run(args) {
            Ok(()) => 0,
            Err(err) => {
                eprintln!("{err}");
                err.exit_code()
            }
        },
        Command::RecommendNextAgentResearch(args) => {
            match recommend_next_agent_research::run(args) {
                Ok(()) => 0,
                Err(err) => {
                    eprintln!("{err}");
                    err.exit_code()
                }
            }
        }
        Command::PrepareProvingRunCloseout(args) => match prepare_proving_run_closeout::run(args) {
            Ok(()) => 0,
            Err(err) => {
                eprintln!("{err}");
                err.exit_code()
            }
        },
        Command::RepairRuntimeEvidence(args) => match repair_runtime_evidence::run(args) {
            Ok(()) => 0,
            Err(err) => {
                eprintln!("{err}");
                err.exit_code()
            }
        },
        Command::CapabilityMatrix(args) => match capability_matrix::run(args) {
            Ok(()) => 0,
            Err(err) => {
                eprintln!("{err}");
                1
            }
        },
        Command::CapabilityMatrixAudit(args) => match capability_matrix_audit::run(args) {
            Ok(()) => 0,
            Err(err) => {
                eprintln!("{err}");
                1
            }
        },
        Command::CheckAgentDrift(args) => match agent_maintenance_drift::run(args) {
            Ok(agent_maintenance_drift::DriftCheckOutcome::Clean(_)) => 0,
            Ok(agent_maintenance_drift::DriftCheckOutcome::DriftDetected(_)) => 2,
            Err(err) => {
                eprintln!("{err}");
                err.exit_code()
            }
        },
        Command::MaintenanceWatch(args) => match agent_maintenance_watch::run(args) {
            Ok(()) => 0,
            Err(err) => {
                eprintln!("{err}");
                err.exit_code()
            }
        },
        Command::PrepareAgentMaintenance(args) => match agent_maintenance_prepare::run(args) {
            Ok(()) => 0,
            Err(err) => {
                eprintln!("{err}");
                err.exit_code()
            }
        },
        Command::ExecuteAgentMaintenance(args) => match agent_maintenance_execute::run(args) {
            Ok(()) => 0,
            Err(err) => {
                eprintln!("{err}");
                err.exit_code()
            }
        },
        Command::RefreshAgent(args) => match agent_maintenance_refresh::run(args) {
            Ok(()) => 0,
            Err(err) => {
                eprintln!("{err}");
                err.exit_code()
            }
        },
        Command::CloseAgentMaintenance(args) => match agent_maintenance_closeout::run(args) {
            Ok(()) => 0,
            Err(err) => {
                eprintln!("{err}");
                err.exit_code()
            }
        },
        Command::HistoricalLifecycleBackfill(args) => {
            match historical_lifecycle_backfill::run(args) {
                Ok(()) => 0,
                Err(err) => {
                    eprintln!("{err}");
                    err.exit_code()
                }
            }
        }
        Command::SupportMatrix(args) => match support_matrix::run(args) {
            Ok(()) => 0,
            Err(err) => {
                eprintln!("{err}");
                1
            }
        },
        Command::VersionBump(args) => match version_bump::run(args) {
            Ok(()) => 0,
            Err(err) => {
                eprintln!("{err}");
                1
            }
        },
    };

    std::process::exit(exit_code);
}
