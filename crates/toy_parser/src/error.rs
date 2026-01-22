#![allow(unused_assignments)] // Fields used by derive macros

use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

#[derive(Error, Diagnostic, Debug, PartialEq, Clone, Hash)]
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

    #[error("incomplete expression after `{operator}`")]
    #[diagnostic(code("parse:incomplete_expression"))]
    #[diagnostic(help("did you forget another operand?"))]
    IncompleteExpression {
        #[label("expected expression after this")]
        at: SourceSpan,
        operator: String,
    },

    #[error("cannot assign to undefined variable `{name}`")]
    #[diagnostic(code("parse:suggest_definition"))]
    SuggestDefinition {
        #[label("not defined")]
        at: SourceSpan,
        name: String,
        #[help]
        hint: String,
    },

    #[error("unexpected semicolon")]
    #[diagnostic(code("parse:unnecessary_semicolon"))]
    #[diagnostic(help("semicolons are not needed - statements are separated by newlines"))]
    UnnecessarySemicolon {
        #[label("not needed")]
        at: SourceSpan,
    },
}

impl From<toy_lexer::LexError> for ParseError {
    fn from(lex_error: toy_lexer::LexError) -> Self {
        match lex_error {
            toy_lexer::LexError::InvalidToken { at, text } if text == ";" => {
                ParseError::UnnecessarySemicolon { at }
            }
            toy_lexer::LexError::InvalidToken { at, text } => ParseError::UnexpectedToken {
                at,
                expected: "valid token".to_string(),
                found: Some(format!("invalid character `{text}`")),
            },
        }
    }
}
