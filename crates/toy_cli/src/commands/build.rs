use std::path::PathBuf;

use clap::Args;
use miette::{Diagnostic, Result};
use thiserror::Error;
use toy_compiler::{CompileOptions, compile};

#[derive(Diagnostic, Debug, Error)]
pub enum BuildError {
    #[error("IO error: {0}")]
    IoError(String),

    #[error("compilation failed with errors")]
    CompilationFailed,

    #[error("no code generated")]
    NoCodeGenerated,
}

#[derive(Args)]
pub struct BuildCmd {
    /// Script to build
    pub script: PathBuf,

    /// Output executable path (default: .toy/bin/{debug|release}/{name})
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Build in release mode
    #[arg(long)]
    pub release: bool,
}

impl BuildCmd {
    pub fn run(self) -> Result<()> {
        let start = std::time::Instant::now();

        // 1. Compile source (includes parsing, type checking, codegen, and linking)
        let options = CompileOptions::new(self.script.clone());
        let results = compile(options)?;
        log::debug!("Total compilation took {:?}", start.elapsed());

        // Should be exactly one result
        let result = results
            .into_iter()
            .next()
            .ok_or(BuildError::NoCodeGenerated)?;

        // 2. Check for type errors
        if result.has_type_errors {
            return Err(BuildError::CompilationFailed.into());
        }

        // 3. Determine output path
        let exe_path = if let Some(output) = self.output {
            output
        } else {
            // Default: .toy/bin/{debug|release}/{script_name}
            let mode = if self.release { "release" } else { "debug" };
            let script_name = self
                .script
                .file_stem()
                .ok_or(BuildError::IoError("invalid script path".to_string()))?
                .to_string_lossy()
                .to_string();

            PathBuf::from(format!(".toy/bin/{}/{}", mode, script_name))
        };

        // 4. Create output directory if needed
        if let Some(parent) = exe_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| BuildError::IoError(e.to_string()))?;
        }

        // 5. Write executable
        std::fs::write(&exe_path, &result.executable_bytes)
            .map_err(|e| BuildError::IoError(e.to_string()))?;

        // 6. Make executable on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&exe_path)
                .map_err(|e| BuildError::IoError(e.to_string()))?
                .permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&exe_path, perms)
                .map_err(|e| BuildError::IoError(e.to_string()))?;
        }

        eprintln!("Built: {}", exe_path.display());
        Ok(())
    }
}
