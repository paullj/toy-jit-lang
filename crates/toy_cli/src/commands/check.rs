use std::path::PathBuf;

use clap::Args;
use miette::{Diagnostic, Result};
use thiserror::Error;
use toy_compiler::{CompileOptions, check};

#[derive(Diagnostic, Debug, Error)]
pub enum CheckError {
    #[error("type checking failed with errors")]
    TypeErrors,
}

#[derive(Args)]
pub struct CheckCmd {
    /// Script to check
    pub script: PathBuf,
}

impl CheckCmd {
    pub fn run(self) -> Result<()> {
        let start = std::time::Instant::now();

        // Run type checking without code generation
        let options = CompileOptions::new(self.script);
        let results = check(options)?;
        log::debug!("Type checking took {:?}", start.elapsed());

        // Should be exactly one result
        let result = results.into_iter().next().ok_or(CheckError::TypeErrors)?;

        // Check for type errors
        if result.has_type_errors {
            return Err(CheckError::TypeErrors.into());
        }

        eprintln!("Check passed");
        Ok(())
    }
}
