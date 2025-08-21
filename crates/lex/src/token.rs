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

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(TokenKind::Identifier, 0..5, "hello", "identifier  0..5  \"hello\"")]
    #[case(TokenKind::Integer, 10..13, "123", "integer    10..13 \"123\"")]
    #[case(TokenKind::Plus, 5..6, "+", "+           5..6  \"+\"")]
    #[case(TokenKind::Whitespace, 0..3, "   ", "whitespace  0..3  \"   \"")]
    #[case(TokenKind::Comment, 7..16, "# comment", "#           7..16 \"# comment\"")]
    #[case(TokenKind::ColonEquals, 20..22, ":=", ":=         20..22 \":=\"")]
    fn test_token_display(
        #[case] kind: TokenKind,
        #[case] span: Range<usize>,
        #[case] text: &str,
        #[case] expected: &str,
    ) {
        let token = Token { kind, span, text };
        assert_eq!(format!("{}", token), expected);
    }
}
