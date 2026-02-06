#![forbid(unsafe_code)]

mod codex_report;
mod codex_retain;
mod codex_snapshot;
mod codex_union;
mod codex_validate;
mod codex_version_metadata;
mod codex_wrapper_coverage;

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
    /// Generate a Codex CLI snapshot manifest under `cli_manifests/codex/`.
    CodexSnapshot(codex_snapshot::Args),
    /// Merge per-target snapshots into a union snapshot under `cli_manifests/codex/`.
    CodexUnion(codex_union::Args),
    /// Generate deterministic coverage reports under `cli_manifests/codex/reports/<version>/`.
    CodexReport(codex_report::Args),
    /// Materialize `cli_manifests/codex/versions/<version>.json` deterministically.
    CodexVersionMetadata(codex_version_metadata::Args),
    /// Deterministically prune out-of-window snapshots/reports directories (dry-run by default).
    CodexRetain(codex_retain::Args),
    /// Generate `cli_manifests/codex/wrapper_coverage.json` from wrapper source of truth.
    CodexWrapperCoverage(codex_wrapper_coverage::CliArgs),
    /// Validate committed Codex parity artifacts under `cli_manifests/codex/`.
    CodexValidate(codex_validate::Args),
}

fn main() {
    let cli = Cli::parse();

    let exit_code = match cli.command {
        Command::CodexSnapshot(args) => match codex_snapshot::run(args) {
            Ok(()) => 0,
            Err(err) => {
                eprintln!("{err}");
                1
            }
        },
        Command::CodexUnion(args) => match codex_union::run(args) {
            Ok(()) => 0,
            Err(err) => {
                eprintln!("{err}");
                1
            }
        },
        Command::CodexReport(args) => match codex_report::run(args) {
            Ok(()) => 0,
            Err(err) => {
                eprintln!("{err}");
                1
            }
        },
        Command::CodexVersionMetadata(args) => match codex_version_metadata::run(args) {
            Ok(()) => 0,
            Err(err) => {
                eprintln!("{err}");
                1
            }
        },
        Command::CodexRetain(args) => match codex_retain::run(args) {
            Ok(()) => 0,
            Err(err) => {
                eprintln!("{err}");
                1
            }
        },
        Command::CodexWrapperCoverage(args) => match codex_wrapper_coverage::run(args) {
            Ok(()) => 0,
            Err(err) => {
                eprintln!("{err}");
                1
            }
        },
        Command::CodexValidate(args) => codex_validate::run(args),
    };

    std::process::exit(exit_code);
}
