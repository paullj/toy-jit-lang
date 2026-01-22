use std::path::PathBuf;
use std::process::Command;

use clap::Args;
use miette::{Diagnostic, Result};
use thiserror::Error;

use super::BuildCmd;

#[derive(Diagnostic, Debug, Error)]
pub enum RunError {
    #[error("build failed")]
    BuildFailed,

    #[error("failed to execute binary: {0}")]
    ExecutionFailed(String),

    #[error("binary exited with code {0}")]
    NonZeroExit(i32),
}

#[derive(Args)]
pub struct RunCmd {
    /// Script to run
    pub script: PathBuf,

    /// Run in release mode
    #[arg(long)]
    pub release: bool,
}

impl RunCmd {
    pub fn run(self) -> Result<()> {
        // 1. Build the script
        let build_cmd = BuildCmd {
            script: self.script.clone(),
            output: None,
            release: self.release,
        };
        build_cmd.run()?;

        // 2. Determine the executable path
        let mode = if self.release { "release" } else { "debug" };
        let script_name = self
            .script
            .file_stem()
            .ok_or(RunError::BuildFailed)?
            .to_string_lossy()
            .to_string();
        let exe_path = PathBuf::from(format!(".toy/bin/{}/{}", mode, script_name));

        // 3. Execute the binary
        let mut child = Command::new(&exe_path)
            .spawn()
            .map_err(|e| RunError::ExecutionFailed(e.to_string()))?;

        let status = child
            .wait()
            .map_err(|e| RunError::ExecutionFailed(e.to_string()))?;

        // 4. Pass through exit code
        if !status.success()
            && let Some(code) = status.code()
        {
            return Err(RunError::NonZeroExit(code).into());
        }

        Ok(())
    }
}
