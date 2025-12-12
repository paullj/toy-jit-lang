use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

#[derive(Error, Diagnostic, Debug, Clone, PartialEq)]
pub enum Error {
    #[error("invalid token")]
    #[diagnostic(code("lex:invalid_token"))]
    InvalidToken {
        #[label("unexpected character")]
        at: SourceSpan,
    },
}

impl Default for Error {
    fn default() -> Self {
        Self::InvalidToken { at: (0..0).into() }
    }
}
