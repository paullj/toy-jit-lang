use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

#[derive(Error, Diagnostic, Debug, Clone, PartialEq)]
pub enum Error {
    #[error("invalid token `{text}`")]
    #[diagnostic(code("lex:invalid_token"))]
    InvalidToken {
        #[label("unexpected character")]
        at: SourceSpan,
        text: String,
    },
}

impl Default for Error {
    fn default() -> Self {
        Self::InvalidToken {
            at: (0..0).into(),
            text: String::new(),
        }
    }
}
