use std::fmt::Display;

use crate::error::Error;
use logos::Logos;

#[derive(Logos, Debug, Clone, Copy, PartialEq)]
#[logos(error = Error)]
pub enum TokenKind {
    #[regex(r#"[ \t\f]+"#)]
    Whitespace,
    #[regex("#.*")]
    Comment,
    #[regex(r"[\r\n]+")]
    EOL,

    // Single-character tokens
    #[token("(")]
    LeftParenthesis,
    #[token(")")]
    RightParenthesis,
    // #[token("{")]
    // LeftBrace,
    // #[token("}")]
    // RightBrace,
    // #[token(",")]
    // Comma,
    // #[token(":")]
    // Colon,
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Asterisk,
    #[token("/")]
    Slash,
    #[token("=")]
    Equals,

    // Multi-character tokens
    #[token(":=")]
    ColonEquals,
    // #[token("->")]
    // RightArrow,

    // Keywords
    // #[token("fn")]
    // Fn,
    // #[token("return")]
    // Return,
    #[token("echo")]
    Echo,

    // Literals
    #[regex("[a-zA-Z_][a-zA-Z0-9_]*")]
    Identifier,
    #[regex(r#"[0-9]+(?:_[0-9]+)*"#)]
    Integer,
    // #[regex(r#"[0-9]+(?:_[0-9]+)*\.[0-9]+(?:_[0-9]+)*"#)]
    // Float,
}

impl TokenKind {
    pub fn is_trivia(self) -> bool {
        matches!(self, Self::Whitespace | Self::EOL | Self::Comment)
    }
}

impl Display for TokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenKind::Whitespace => write!(f, "whitespace"),
            TokenKind::Comment => write!(f, "#"),
            TokenKind::EOL => write!(f, "newline"),
            TokenKind::Plus => write!(f, "+"),
            TokenKind::Minus => write!(f, "-"),
            TokenKind::Asterisk => write!(f, "*"),
            TokenKind::Slash => write!(f, "/"),
            TokenKind::ColonEquals => write!(f, ":="),
            TokenKind::Echo => write!(f, "echo"),
            TokenKind::Identifier => write!(f, "identifier"),
            TokenKind::Integer => write!(f, "integer"),
            TokenKind::Equals => write!(f, "="),
            TokenKind::LeftParenthesis => write!(f, "("),
            TokenKind::RightParenthesis => write!(f, ")"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(" ", TokenKind::Whitespace)]
    #[case("\t", TokenKind::Whitespace)]
    #[case("# comment", TokenKind::Comment)]
    #[case("\n", TokenKind::EOL)]
    #[case("+", TokenKind::Plus)]
    #[case("-", TokenKind::Minus)]
    #[case("*", TokenKind::Asterisk)]
    #[case("/", TokenKind::Slash)]
    #[case(":=", TokenKind::ColonEquals)]
    #[case("identifier", TokenKind::Identifier)]
    #[case("0", TokenKind::Integer)]
    fn test_single_token(#[case] input: &str, #[case] expected_kind: TokenKind) {
        let mut lexer = TokenKind::lexer(input);
        let kind = lexer.next().unwrap().expect("token is none");

        assert_eq!(kind, expected_kind);
    }

    #[rstest]
    #[case(TokenKind::Whitespace)]
    #[case(TokenKind::Comment)]
    #[case(TokenKind::EOL)]
    fn test_is_trivia(#[case] kind: TokenKind) {
        assert_eq!(kind.is_trivia(), true);
    }

    #[rstest]
    #[case(TokenKind::Identifier)]
    #[case(TokenKind::Integer)]
    #[case(TokenKind::Echo)]
    fn test_is_not_trivia(#[case] kind: TokenKind) {
        assert_eq!(kind.is_trivia(), false);
    }

    #[rstest]
    #[case(TokenKind::Whitespace, "whitespace")]
    #[case(TokenKind::Comment, "#")]
    #[case(TokenKind::EOL, "newline")]
    #[case(TokenKind::Plus, "+")]
    #[case(TokenKind::Minus, "-")]
    #[case(TokenKind::Asterisk, "*")]
    #[case(TokenKind::Slash, "/")]
    #[case(TokenKind::ColonEquals, ":=")]
    #[case(TokenKind::Echo, "echo")]
    #[case(TokenKind::Identifier, "identifier")]
    #[case(TokenKind::Integer, "integer")]
    fn test_token_kind_display(#[case] token_kind: TokenKind, #[case] expected: &str) {
        assert_eq!(format!("{}", token_kind), expected);
    }
}
