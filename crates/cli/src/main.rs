#![allow(unused_assignments)]

mod commands;
mod repl;
mod styles;

use clap::{Parser, Subcommand};
use miette::Result;

use commands::{
    AddCmd, BenchCmd, BuildCmd, DocsCmd, FormatCmd, LintCmd, ListCmd, NewCmd, RemoveCmd, RunCmd,
    ServerCmd, TestCmd, UpdateCmd, run_repl,
};
use styles::STYLES;

const HELP_TEMPLATE: &str = "  ʕ•ᴥ•ʔ {about}

{usage-heading} {usage}

{all-args}";

#[derive(Parser)]
#[command(name = "toy", version, about = "toy programming language")]
#[command(styles = STYLES, help_template = HELP_TEMPLATE, disable_help_subcommand = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Build and run a script or project
    #[command(display_order = 1)]
    Run(RunCmd),
    /// Build a script or project
    #[command(display_order = 2)]
    Build(BuildCmd),
    /// Benchmark execution modes
    #[command(display_order = 3)]
    Bench(BenchCmd),

    /// Run tests
    #[command(display_order = 10)]
    Test(TestCmd),
    /// Format source files
    #[command(display_order = 11)]
    Format(FormatCmd),
    /// Lint source files
    #[command(display_order = 12)]
    Lint(LintCmd),
    /// Generate documentation
    #[command(display_order = 13)]
    Docs(DocsCmd),

    /// Start language server
    #[command(display_order = 20)]
    Server(ServerCmd),

    /// Create a new project
    #[command(display_order = 30)]
    New(NewCmd),
    /// Add a package
    #[command(display_order = 31)]
    Add(AddCmd),
    /// Remove a package
    #[command(display_order = 32)]
    Remove(RemoveCmd),
    /// List packages
    #[command(display_order = 33)]
    List(ListCmd),
    /// Update packages
    #[command(display_order = 34)]
    Update(UpdateCmd),
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        None => run_repl(),
        Some(Command::Run(cmd)) => cmd.run(),
        Some(Command::Build(cmd)) => cmd.run(),
        Some(Command::Bench(cmd)) => cmd.run(),
        Some(Command::Test(cmd)) => cmd.run(),
        Some(Command::Format(cmd)) => cmd.run(),
        Some(Command::Lint(cmd)) => cmd.run(),
        Some(Command::Docs(cmd)) => cmd.run(),
        Some(Command::Server(cmd)) => cmd.run(),
        Some(Command::New(cmd)) => cmd.run(),
        Some(Command::Add(cmd)) => cmd.run(),
        Some(Command::Remove(cmd)) => cmd.run(),
        Some(Command::List(cmd)) => cmd.run(),
        Some(Command::Update(cmd)) => cmd.run(),
    }
}
