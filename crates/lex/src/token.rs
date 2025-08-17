use std::{
    fmt::{Debug, Display},
    ops::Range,
};

use crate::TokenKind;

#[derive(Debug, PartialEq, Clone)]
pub struct Token<'a> {
    pub kind: TokenKind,
    pub span: Range<usize>,
    pub text: &'a str,
}

impl<'a> Display for Token<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:<10} {:>2}..{:<2} {:?}",
            format!("{}", self.kind),
            self.span.start,
            self.span.end,
            self.text
        )
    }
}
