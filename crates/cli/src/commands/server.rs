use clap::Args;
use miette::Result;

#[derive(Args)]
pub struct ServerCmd;

impl ServerCmd {
    pub fn run(self) -> Result<()> {
        lsp::run()
    }
}
