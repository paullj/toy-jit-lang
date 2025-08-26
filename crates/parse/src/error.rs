use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ParseError {
    #[error("unknown data store error")]
    UnexpectedToken,
    #[error("")]
    ExpectedEqual,
    #[error("")]
    ExpectedColonEqual,
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
