use clap::Args;
use miette::Result;

use super::not_implemented;

#[derive(Args)]
pub struct TestCmd;

impl TestCmd {
    pub fn run(self) -> Result<()> {
        not_implemented("test");
        Ok(())
    }
}

#[derive(Args)]
pub struct FormatCmd;

impl FormatCmd {
    pub fn run(self) -> Result<()> {
        not_implemented("format");
        Ok(())
    }
}

#[derive(Args)]
pub struct LintCmd;

impl LintCmd {
    pub fn run(self) -> Result<()> {
        not_implemented("lint");
        Ok(())
    }
}

#[derive(Args)]
pub struct DocsCmd;

impl DocsCmd {
    pub fn run(self) -> Result<()> {
        not_implemented("docs");
        Ok(())
    }
}

#[derive(Args)]
pub struct NewCmd {
    /// Project name
    pub project: String,
}

impl NewCmd {
    pub fn run(self) -> Result<()> {
        not_implemented("new");
        Ok(())
    }
}

#[derive(Args)]
pub struct AddCmd {
    /// Package to add
    pub package: String,
}

impl AddCmd {
    pub fn run(self) -> Result<()> {
        not_implemented("add");
        Ok(())
    }
}

#[derive(Args)]
pub struct RemoveCmd {
    /// Package to remove
    pub package: String,
}

impl RemoveCmd {
    pub fn run(self) -> Result<()> {
        not_implemented("remove");
        Ok(())
    }
}

#[derive(Args)]
pub struct ListCmd;

impl ListCmd {
    pub fn run(self) -> Result<()> {
        not_implemented("list");
        Ok(())
    }
}

#[derive(Args)]
pub struct UpdateCmd;

impl UpdateCmd {
    pub fn run(self) -> Result<()> {
        not_implemented("update");
        Ok(())
    }
}
