use std::ops::Range;

use codespan_reporting::diagnostic::{Diagnostic, Label};
use lex::TokenKind;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ParseError {
    #[error("unexpected token")]
    UnexpectedToken,
    #[error("unexpected token in variable statement")]
    UnexpectedTokenInVariableStatement,
    #[error("")]
    ExpectedEqual,
    #[error("")]
    ExpectedColonEqual,
}

#[derive(Debug, PartialEq)]
pub struct Issue {
    pub expected: Vec<TokenKind>,
    pub found: Option<TokenKind>,
    pub span: Range<usize>,
    pub kind: ParseError,
}

pub trait AsDiagnostic {
    fn as_diagnostic(&self) -> Diagnostic<()>;
}

impl AsDiagnostic for ParseError {
    fn as_diagnostic(&self) -> Diagnostic<()> {
        match self {
            ParseError::UnexpectedToken => todo!(),
            ParseError::UnexpectedTokenInVariableStatement => todo!(),
            ParseError::ExpectedEqual => todo!(),
            ParseError::ExpectedColonEqual => todo!(),
        }
    }
}

impl AsDiagnostic for Issue {
    fn as_diagnostic(&self) -> Diagnostic<()> {
        let diagnostic = Diagnostic::error()
            .with_message(self.kind.to_string())
            .with_note(format!(
                "Expected {}",
                one_of(self.expected.iter().map(|t| t.to_string()).collect())
            ))
            .with_label(Label::primary((), self.span.clone()));

        diagnostic
    }
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

// use rowan::TextRange;

// #[derive(Debug, Clone)]
// pub struct ParseError {
//     pub message: String,
//     pub range: TextRange,
//     pub severity: ErrorSeverity,
//     pub suggestion: Option<String>,
// }

// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
// pub enum ErrorSeverity {
//     Error,
//     Warning,
//     Info,
// }

// impl ParseError {
//     pub fn new(message: String, range: TextRange) -> Self {
//         Self {
//             message,
//             range,
//             severity: ErrorSeverity::Error,
//             suggestion: None,
//         }
//     }

//     pub fn with_suggestion(mut self, suggestion: String) -> Self {
//         self.suggestion = Some(suggestion);
//         self
//     }

//     pub fn with_severity(mut self, severity: ErrorSeverity) -> Self {
//         self.severity = severity;
//         self
//     }
// }

// impl std::fmt::Display for ParseError {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(f, "{}", self.message)?;
//         if let Some(suggestion) = &self.suggestion {
//             write!(f, " {}", suggestion)?;
//         }
//         Ok(())
//     }
// }

// impl std::error::Error for ParseError {}
