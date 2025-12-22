use std::path::PathBuf;

use analyse::{AnalyseDiagnostic, Document};
use clap::{Args, ValueEnum};
use miette::{Diagnostic, Result};
use runtime::{ExecutionMode, Runtime};
use thiserror::Error;

#[derive(Diagnostic, Debug, Error)]
#[error("compilation errors")]
struct CompileErrors {
    #[source_code]
    src: String,
    #[related]
    errors: Vec<AnalyseDiagnostic>,
}

/// Execution mode for the runtime
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum Mode {
    /// Bytecode VM interpreter
    Vm,
    /// Native JIT compilation
    Jit,
    /// VM with hot-path JIT (default)
    #[default]
    Tiered,
}

impl From<Mode> for ExecutionMode {
    fn from(mode: Mode) -> Self {
        match mode {
            Mode::Vm => ExecutionMode::Vm,
            Mode::Jit => ExecutionMode::Jit,
            Mode::Tiered => ExecutionMode::Tiered,
        }
    }
}

#[derive(Args)]
pub struct RunCmd {
    /// Script to run
    pub script: PathBuf,

    /// Execution mode
    #[arg(long, value_enum, default_value = "tiered")]
    pub mode: Mode,
}

impl RunCmd {
    pub fn run(self) -> Result<()> {
        let src = std::fs::read_to_string(&self.script).map_err(|e| miette::miette!("{e}"))?;
        process(&src, self.mode.into())
    }
}

pub fn process(src: &str, mode: ExecutionMode) -> Result<()> {
    let doc = Document::new(src.to_string());

    if doc.has_errors() {
        return Err(CompileErrors {
            src: src.to_string(),
            errors: doc.diagnostics(),
        }
        .into());
    }

    let lower = doc
        .lower_result
        .as_ref()
        .ok_or_else(|| miette::miette!("failed to lower"))?;
    let inferred = doc
        .infer_result
        .as_ref()
        .ok_or_else(|| miette::miette!("failed to infer"))?;

    let mir_module = mir::lower(lower, inferred);

    let mut runtime = Runtime::new(mode);
    let result = runtime
        .execute(&mir_module, lower, inferred)
        .map_err(|e| miette::miette!("{e}"))?;

    println!("{}", result);

    Ok(())
}
