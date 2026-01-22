use miette::Diagnostic;
use thiserror::Error;

#[derive(Error, Diagnostic, Debug, Clone, PartialEq)]
pub enum CompilerError {
    #[error(transparent)]
    #[diagnostic(transparent)]
    Lex(#[from] toy_lexer::LexError),

    #[error(transparent)]
    #[diagnostic(transparent)]
    Parse(#[from] toy_parser::ParseError),

    #[error(transparent)]
    #[diagnostic(transparent)]
    Hir(#[from] toy_hir::HirError),

    #[error(transparent)]
    #[diagnostic(transparent)]
    Typecheck(#[from] toy_typecheck::InferDiagnostic),
}
