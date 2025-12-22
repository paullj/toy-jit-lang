#![allow(unused_assignments)] // Fields used by derive macros

use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

#[derive(Error, Diagnostic, Debug, Clone, PartialEq)]
pub enum HirDiagnostic {
    #[error("division by zero")]
    #[diagnostic(code(hir::division_by_zero))]
    DivisionByZero {
        #[label("divisor is zero")]
        span: SourceSpan,
    },

    #[error("empty block expression")]
    #[diagnostic(code(hir::empty_block), help("empty blocks have type `()` (unit)"))]
    EmptyBlock {
        #[label("empty block")]
        span: SourceSpan,
    },
}
