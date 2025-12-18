#![allow(unused_assignments)] // Fields used by derive macros

use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

#[derive(Error, Diagnostic, Debug, PartialEq)]
pub enum ParseError {
    #[error("unexpected token, expected {expected}")]
    #[diagnostic(code("parse:unexpected_token"))]
    UnexpectedToken {
        #[label("unexpected token")]
        at: SourceSpan,
        expected: String,
        found: Option<String>,
    },

    #[error(
        "expected a variable definition, assignment, or expression starting with a variable name"
    )]
    #[diagnostic(code("parse:expected_variable_item"))]
    ExpectedVariableItem {
        #[label("here")]
        at: SourceSpan,
        #[help]
        expected: Option<String>,
    },
}

impl From<lex::Error> for ParseError {
    fn from(lex_error: lex::Error) -> Self {
        match lex_error {
            lex::Error::InvalidToken { at } => ParseError::UnexpectedToken {
                at,
                expected: "valid token".to_string(),
                found: Some("invalid character".to_string()),
            },
        }
    }
}
