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
    NewLine,

    // Single-character tokens
    #[token("(")]
    LeftParenthesis,
    #[token(")")]
    RightParenthesis,
    #[token("{")]
    LeftBrace,
    #[token("}")]
    RightBrace,
    #[token("[")]
    LeftBracket,
    #[token("]")]
    RightBracket,
    #[token(",")]
    Comma,
    #[token(":")]
    Colon,
    #[token(".")]
    Dot,
    #[token("|")]
    Pipe,
    #[token("_")]
    Underscore,

    // Arithmetic operators
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Asterisk,
    #[token("/")]
    Slash,
    #[token("%")]
    Percent,

    // Float arithmetic operators
    #[token("+.")]
    PlusDot,
    #[token("-.")]
    MinusDot,
    #[token("*.")]
    AsteriskDot,
    #[token("/.")]
    SlashDot,

    // Comparison operators
    #[token("=")]
    Equals,
    #[token("==")]
    EqualsEquals,
    #[token("!=")]
    NotEquals,
    #[token("!")]
    Bang,
    #[token(">")]
    GreaterThan,
    #[token("<")]
    LessThan,
    #[token(">=")]
    GreaterThanOrEqual,
    #[token("<=")]
    LessThanOrEqual,

    // Float comparison operators
    #[token(">.")]
    GreaterThanDot,
    #[token("<.")]
    LessThanDot,
    #[token(">=.")]
    GreaterThanOrEqualDot,
    #[token("<=.")]
    LessThanOrEqualDot,

    // Multi-character tokens
    #[token("->")]
    RightArrow,
    #[token("..")]
    DotDot,

    // Keywords
    #[token("fn")]
    Function,
    #[token("if")]
    If,
    #[token("else")]
    Else,
    #[token("for")]
    For,
    #[token("while")]
    While,
    #[token("loop")]
    Loop,
    #[token("break")]
    Break,
    #[token("match")]
    Match,
    #[token("in")]
    In,
    #[token("and")]
    And,
    #[token("or")]
    Or,
    #[token("type")]
    Type,
    #[token("true")]
    True,
    #[token("false")]
    False,

    // Literals
    #[regex("[a-zA-Z][a-zA-Z0-9_]*")]
    Identifier,
    #[regex(r#"0b[01]+(?:_[01]+)*"#)]
    BinaryInteger,
    #[regex(r#"0o[0-7]+(?:_[0-7]+)*"#)]
    OctalInteger,
    #[regex(r#"0x[0-9a-fA-F]+(?:_[0-9a-fA-F]+)*"#)]
    HexInteger,
    #[regex(r#"[0-9]+(?:_[0-9]+)*"#)]
    Integer,
    #[regex(r#"[0-9]+(?:_[0-9]+)*\.[0-9]+(?:_[0-9]+)*(?:[eE][+-]?[0-9]+)?"#)]
    Float,
    #[regex(r#"[0-9]+(?:_[0-9]+)*[eE][+-]?[0-9]+"#)]
    FloatExponent,
    #[regex(r#""(?:[^"\\]|\\.)*""#)]
    String,
    #[regex(r#""""[^"]*""""#)]
    MultiLineString,
}

impl TokenKind {
    pub fn is_trivia(self) -> bool {
        matches!(self, Self::Whitespace | Self::NewLine | Self::Comment)
    }

    pub fn is_newline(self) -> bool {
        matches!(self, Self::NewLine)
    }
}

impl Display for TokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenKind::Whitespace => write!(f, "whitespace"),
            TokenKind::Comment => write!(f, "#"),
            TokenKind::NewLine => write!(f, "newline"),
            TokenKind::LeftParenthesis => write!(f, "("),
            TokenKind::RightParenthesis => write!(f, ")"),
            TokenKind::LeftBrace => write!(f, "{{"),
            TokenKind::RightBrace => write!(f, "}}"),
            TokenKind::LeftBracket => write!(f, "["),
            TokenKind::RightBracket => write!(f, "]"),
            TokenKind::Comma => write!(f, ","),
            TokenKind::Colon => write!(f, ":"),
            TokenKind::Dot => write!(f, "."),
            TokenKind::Pipe => write!(f, "|"),
            TokenKind::Underscore => write!(f, "_"),
            TokenKind::Plus => write!(f, "+"),
            TokenKind::Minus => write!(f, "-"),
            TokenKind::Asterisk => write!(f, "*"),
            TokenKind::Slash => write!(f, "/"),
            TokenKind::Percent => write!(f, "%"),
            TokenKind::PlusDot => write!(f, "+."),
            TokenKind::MinusDot => write!(f, "-."),
            TokenKind::AsteriskDot => write!(f, "*."),
            TokenKind::SlashDot => write!(f, "/."),
            TokenKind::Equals => write!(f, "="),
            TokenKind::EqualsEquals => write!(f, "=="),
            TokenKind::NotEquals => write!(f, "!="),
            TokenKind::Bang => write!(f, "!"),
            TokenKind::GreaterThan => write!(f, ">"),
            TokenKind::LessThan => write!(f, "<"),
            TokenKind::GreaterThanOrEqual => write!(f, ">="),
            TokenKind::LessThanOrEqual => write!(f, "<="),
            TokenKind::GreaterThanDot => write!(f, ">."),
            TokenKind::LessThanDot => write!(f, "<."),
            TokenKind::GreaterThanOrEqualDot => write!(f, ">=."),
            TokenKind::LessThanOrEqualDot => write!(f, "<=."),
            TokenKind::RightArrow => write!(f, "->"),
            TokenKind::DotDot => write!(f, ".."),
            TokenKind::Function => write!(f, "fn"),
            TokenKind::If => write!(f, "if"),
            TokenKind::Else => write!(f, "else"),
            TokenKind::For => write!(f, "for"),
            TokenKind::While => write!(f, "while"),
            TokenKind::Loop => write!(f, "loop"),
            TokenKind::Break => write!(f, "break"),
            TokenKind::Match => write!(f, "match"),
            TokenKind::In => write!(f, "in"),
            TokenKind::And => write!(f, "and"),
            TokenKind::Or => write!(f, "or"),
            TokenKind::Type => write!(f, "type"),
            TokenKind::True => write!(f, "true"),
            TokenKind::False => write!(f, "false"),
            TokenKind::Identifier => write!(f, "identifier"),
            TokenKind::BinaryInteger => write!(f, "binary integer"),
            TokenKind::OctalInteger => write!(f, "octal integer"),
            TokenKind::HexInteger => write!(f, "hex integer"),
            TokenKind::Integer => write!(f, "integer"),
            TokenKind::Float => write!(f, "float"),
            TokenKind::FloatExponent => write!(f, "float"),
            TokenKind::String => write!(f, "string"),
            TokenKind::MultiLineString => write!(f, "multi-line string"),
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
    #[case("\n", TokenKind::NewLine)]
    // Operators
    #[case("+", TokenKind::Plus)]
    #[case("-", TokenKind::Minus)]
    #[case("*", TokenKind::Asterisk)]
    #[case("/", TokenKind::Slash)]
    #[case("%", TokenKind::Percent)]
    #[case("=", TokenKind::Equals)]
    #[case("==", TokenKind::EqualsEquals)]
    #[case("!=", TokenKind::NotEquals)]
    #[case("!", TokenKind::Bang)]
    #[case(">", TokenKind::GreaterThan)]
    #[case("<", TokenKind::LessThan)]
    #[case(">=", TokenKind::GreaterThanOrEqual)]
    #[case("<=", TokenKind::LessThanOrEqual)]
    #[case("->", TokenKind::RightArrow)]
    #[case("..", TokenKind::DotDot)]
    #[case(":", TokenKind::Colon)]
    #[case(".", TokenKind::Dot)]
    #[case("|", TokenKind::Pipe)]
    #[case("_", TokenKind::Underscore)]
    // Float operators
    #[case("+.", TokenKind::PlusDot)]
    #[case("-.", TokenKind::MinusDot)]
    #[case("*.", TokenKind::AsteriskDot)]
    #[case("/.", TokenKind::SlashDot)]
    #[case(">.", TokenKind::GreaterThanDot)]
    #[case("<.", TokenKind::LessThanDot)]
    #[case(">=.", TokenKind::GreaterThanOrEqualDot)]
    #[case("<=.", TokenKind::LessThanOrEqualDot)]
    // Brackets
    #[case("(", TokenKind::LeftParenthesis)]
    #[case(")", TokenKind::RightParenthesis)]
    #[case("{", TokenKind::LeftBrace)]
    #[case("}", TokenKind::RightBrace)]
    #[case("[", TokenKind::LeftBracket)]
    #[case("]", TokenKind::RightBracket)]
    #[case(",", TokenKind::Comma)]
    // Keywords
    #[case("fn", TokenKind::Function)]
    #[case("if", TokenKind::If)]
    #[case("else", TokenKind::Else)]
    #[case("for", TokenKind::For)]
    #[case("while", TokenKind::While)]
    #[case("loop", TokenKind::Loop)]
    #[case("break", TokenKind::Break)]
    #[case("match", TokenKind::Match)]
    #[case("in", TokenKind::In)]
    #[case("and", TokenKind::And)]
    #[case("or", TokenKind::Or)]
    #[case("type", TokenKind::Type)]
    #[case("true", TokenKind::True)]
    #[case("false", TokenKind::False)]
    // Identifiers
    #[case("identifier", TokenKind::Identifier)]
    #[case("int", TokenKind::Identifier)]
    #[case("float", TokenKind::Identifier)]
    #[case("bool", TokenKind::Identifier)]
    #[case("string", TokenKind::Identifier)]
    // Integers
    #[case("0", TokenKind::Integer)]
    #[case("123", TokenKind::Integer)]
    #[case("1_000", TokenKind::Integer)]
    #[case("0b1010", TokenKind::BinaryInteger)]
    #[case("0o12", TokenKind::OctalInteger)]
    #[case("0xA", TokenKind::HexInteger)]
    #[case("0xFF", TokenKind::HexInteger)]
    // Floats
    #[case("3.14", TokenKind::Float)]
    #[case("1_000.50", TokenKind::Float)]
    #[case("0.000_123", TokenKind::Float)]
    #[case("2.5e10", TokenKind::Float)]
    #[case("1e5", TokenKind::FloatExponent)]
    #[case("1E-5", TokenKind::FloatExponent)]
    // Strings
    #[case(r#""hello""#, TokenKind::String)]
    #[case(r#""Hello, World!""#, TokenKind::String)]
    #[case(r#""Line 1\nLine 2""#, TokenKind::String)]
    #[case(r#""\"quoted\"""#, TokenKind::String)]
    fn test_single_token(#[case] input: &str, #[case] expected_kind: TokenKind) {
        let mut lexer = TokenKind::lexer(input);
        let kind = lexer.next().unwrap().expect("token is none");

        assert_eq!(kind, expected_kind);
    }

    #[rstest]
    #[case(TokenKind::Whitespace)]
    #[case(TokenKind::Comment)]
    #[case(TokenKind::NewLine)]
    fn test_is_trivia(#[case] kind: TokenKind) {
        assert_eq!(kind.is_trivia(), true);
    }

    #[rstest]
    #[case(TokenKind::Identifier)]
    #[case(TokenKind::Integer)]
    #[case(TokenKind::Float)]
    #[case(TokenKind::String)]
    fn test_is_not_trivia(#[case] kind: TokenKind) {
        assert_eq!(kind.is_trivia(), false);
    }

    #[test]
    fn test_is_newline() {
        assert!(TokenKind::NewLine.is_newline());
        assert!(!TokenKind::Whitespace.is_newline());
    }
}
