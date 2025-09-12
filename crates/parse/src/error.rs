use std::ops::Range;

use lex::TokenKind;
use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

#[derive(Error, Diagnostic, Debug, PartialEq)]
pub enum ParseError {
    #[error("unexpected token")]
    UnexpectedToken,

    #[error("e000")]
    #[diagnostic(code(parse::expected_variable_item))]
    ExpectedVariableItem {
        #[label("here")]
        at: SourceSpan,
        #[help]
        expected: Option<String>,
    },
}

#[derive(Error, Diagnostic, Debug)]
#[error("expected a variable definition, assignment, or expression starting with a variable name")]
pub struct ExpectedVariableItem {
    #[label("here")]
    pub at: SourceSpan,
}

fn one_of(tokens: Vec<String>) -> String {
    let (token_last, tokens) = match tokens.split_last() {
        Some((token_last, &[])) => return token_last.to_string(),
        Some((token_last, tokens)) => (token_last, tokens),
        None => return "nothing".to_string(),
    };

    let mut output = String::new();
    for token in tokens {
        output.push_str(token);
        output.push_str(", ");
    }
    output.push_str("or ");
    output.push_str(token_last);
    output
}
