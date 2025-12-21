use std::path::PathBuf;

use clap::{Args, ValueEnum};
use miette::{Diagnostic, Result};
use thiserror::Error;

use ast::AstNode;
use infer::InferDiagnostic;
use parse::ParseError;
use runtime::{ExecutionMode, Runtime};

#[derive(Diagnostic, Debug, Error)]
#[error("parse errors")]
struct ParseErrors {
    #[source_code]
    src: String,
    #[related]
    errors: Vec<ParseError>,
}

#[derive(Diagnostic, Debug, Error)]
#[error("type errors")]
struct TypeErrors {
    #[source_code]
    src: String,
    #[related]
    errors: Vec<InferDiagnostic>,
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
    let (tree, errors) = parse::parse(src);
    if !errors.is_empty() {
        return Err(ParseErrors {
            src: src.to_string(),
            errors,
        }
        .into());
    }

    let root = ast::Root::cast(tree).ok_or_else(|| miette::miette!("invalid syntax tree"))?;
    let lower = hir::lower(root);
    let inferred = infer::infer(&lower);

    if inferred.has_errors() {
        return Err(TypeErrors {
            src: src.to_string(),
            errors: inferred.diagnostics,
        }
        .into());
    }

    let mir_module = mir::lower(&lower, &inferred);

    let mut runtime = Runtime::new(mode);
    let result = runtime
        .execute(&mir_module, &lower, &inferred)
        .map_err(|e| miette::miette!("{e}"))?;

    println!("{}", result);

    Ok(())
}
