use std::path::PathBuf;

use clap::Args;
use miette::{Diagnostic, Result};
use thiserror::Error;

use ast::AstNode;
use infer::InferDiagnostic;
use parse::ParseError;

use super::not_implemented;

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

        let (tree, errors) = parse::parse(&src);
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
