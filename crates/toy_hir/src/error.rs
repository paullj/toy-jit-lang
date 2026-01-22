#![allow(unused_assignments)] // Fields used by derive macros

use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

#[derive(Error, Diagnostic, Debug, Clone, PartialEq)]
pub enum HirError {
    #[error("division by zero")]
    #[diagnostic(code(toy_hir::division_by_zero))]
    DivisionByZero {
        #[label("divisor is zero")]
        span: SourceSpan,
    },

    #[error("empty block expression")]
    #[diagnostic(code(toy_hir::empty_block), help("empty blocks have type `()` (unit)"))]
    EmptyBlock {
        #[label("empty block")]
        span: SourceSpan,
    },

    #[error("`break` outside of loop")]
    #[diagnostic(code(toy_hir::break_outside_loop))]
    BreakOutsideLoop {
        #[label("cannot `break` outside of a loop")]
        span: SourceSpan,
    },

    #[error("`continue` outside of loop")]
    #[diagnostic(code(toy_hir::continue_outside_loop))]
    ContinueOutsideLoop {
        #[label("cannot `continue` outside of a loop")]
        span: SourceSpan,
    },

    #[error("unknown loop label `{label}`")]
    #[diagnostic(code(toy_hir::unknown_loop_label))]
    UnknownLoopLabel {
        label: String,
        #[label("no loop with this label in scope")]
        span: SourceSpan,
    },
}
