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
        #[help]
        suggestion: Option<String>,
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

    #[error("operator `{op}` is for `{expected_type}`, but operands are `{actual_type}`")]
    #[diagnostic(code(infer::wrong_operator))]
    WrongOperator {
        op: String,
        expected_type: String,
        actual_type: String,
        #[label("wrong operator for this type")]
        span: SourceSpan,
        #[help]
        suggest_op: String,
    },

    #[error("division by zero")]
    #[diagnostic(code(infer::division_by_zero))]
    DivisionByZero {
        #[label("divisor is zero")]
        span: SourceSpan,
    },

    #[error("literal `{value}` overflows type `{ty}`")]
    #[diagnostic(code(infer::overflow_literal))]
    OverflowLiteral {
        value: String,
        ty: String,
        #[label("value too large for type")]
        span: SourceSpan,
    },

    #[error("assigning empty block to variable")]
    #[diagnostic(code(infer::empty_block_assignment))]
    #[diagnostic(help(
        "empty blocks have type `()` (unit). Did you mean to have an expression in the block?"
    ))]
    EmptyBlockAssignment {
        #[label("empty block")]
        span: SourceSpan,
    },
}
