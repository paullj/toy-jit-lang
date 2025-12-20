#![allow(unused_assignments)] // Fields used by derive macros

use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

#[derive(Error, Diagnostic, Debug, PartialEq, Clone)]
pub enum InferDiagnostic {
    #[error("type mismatch: expected `{expected}`, found `{found}`")]
    #[diagnostic(code(infer::type_mismatch))]
    Mismatch {
        expected: String,
        found: String,
        #[label("expected `{expected}` here")]
        span: SourceSpan,
    },

    #[error("undefined variable `{name}`")]
    #[diagnostic(code(infer::undefined))]
    Undefined {
        name: String,
        #[label("not found in scope")]
        span: SourceSpan,
    },

    #[error("infinite type: `{var}` occurs in `{ty}`")]
    #[diagnostic(code(infer::infinite_type))]
    InfiniteType {
        var: String,
        ty: String,
        #[label("creates infinite type")]
        span: SourceSpan,
    },

    #[error("arity mismatch: expected {expected} argument(s), found {found}")]
    #[diagnostic(code(infer::arity_mismatch))]
    ArityMismatch {
        expected: usize,
        found: usize,
        #[label("wrong number of arguments")]
        span: SourceSpan,
    },
}
