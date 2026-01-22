mod commands;

use clap::{Parser, Subcommand};
use miette::Result;

use commands::{BuildCmd, CheckCmd, RunCmd};

#[derive(Parser)]
#[command(name = "toy", version, about = "toy programming language")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Type check a script without generating code
    Check(CheckCmd),
    /// Build and run a script
    Run(RunCmd),
    /// Compile a script to a native object file
    Build(BuildCmd),
}

fn main() -> Result<()> {
    toy_logger::init(log::LevelFilter::Debug);
    let cli = Cli::parse();

    match cli.command {
        Command::Check(cmd) => cmd.run(),
        Command::Run(cmd) => cmd.run(),
        Command::Build(cmd) => cmd.run(),
    }
}
