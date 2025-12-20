use miette::Result;

use crate::repl::tui;

pub fn run_repl() -> Result<()> {
    tui::run().map_err(|e| miette::miette!("TUI error: {e}"))
}
