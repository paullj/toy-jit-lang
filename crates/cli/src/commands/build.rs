use std::path::PathBuf;

use analyse::{AnalyseDiagnostic, Document};
use clap::Args;
use miette::{Diagnostic, Result};
use thiserror::Error;

use super::not_implemented;

#[derive(Diagnostic, Debug, Error)]
#[error("compilation errors")]
struct CompileErrors {
    #[source_code]
    src: String,
    #[related]
    errors: Vec<AnalyseDiagnostic>,
}

#[derive(Args)]
pub struct BuildCmd {
    /// Script to build
    pub script: PathBuf,

    /// Output MIR
    #[arg(long)]
    pub mir: bool,

    /// Output bytecode
    #[arg(long)]
    pub bytecode: bool,
}

impl BuildCmd {
    pub fn run(self) -> Result<()> {
        let src = std::fs::read_to_string(&self.script).map_err(|e| miette::miette!("{e}"))?;

        let doc = Document::new(src.clone());

        if doc.has_errors() {
            return Err(CompileErrors {
                src,
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

        if self.mir {
            println!("{}", mir_module);
            return Ok(());
        }

        if self.bytecode {
            let compiled = compile::compile(&mir_module);
            println!("{}", compiled);
            return Ok(());
        }

        not_implemented("build (native compilation)");
        Ok(())
    }
}
