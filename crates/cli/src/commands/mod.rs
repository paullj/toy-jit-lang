mod bench;
mod build;
mod repl;
mod run;
mod server;
mod stubs;

pub use bench::BenchCmd;
pub use build::BuildCmd;
pub use repl::run_repl;
pub use run::RunCmd;
pub use server::ServerCmd;
pub use stubs::{
    AddCmd, DocsCmd, FormatCmd, LintCmd, ListCmd, NewCmd, RemoveCmd, TestCmd, UpdateCmd,
};

use std::io::Write;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

pub fn not_implemented(feature: &str) {
    let mut stderr = StandardStream::stderr(ColorChoice::Always);
    let _ = stderr.set_color(ColorSpec::new().set_fg(Some(Color::Red)));
    let _ = writeln!(stderr, "{feature} is not implemented yet");
    let _ = stderr.reset();
}
