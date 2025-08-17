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
    // #[token("(")]
    // LeftParenthesis,
    // #[token(")")]
    // RightParenthesis,
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
        }
    }
}
