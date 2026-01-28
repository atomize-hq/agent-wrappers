mod codex_snapshot;
mod codex_validate;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "xtask")]
#[command(about = "Project automation tasks")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Generate a Codex CLI snapshot manifest under `cli_manifests/codex/`.
    CodexSnapshot(codex_snapshot::Args),
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
        Command::CodexValidate(args) => codex_validate::run(args),
    };

    std::process::exit(exit_code);
}
