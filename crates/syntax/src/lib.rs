use std::panic;

use lex::TokenKind;
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::FromPrimitive;

#[derive(Debug, Copy, Clone, PartialEq, FromPrimitive, ToPrimitive, Hash, PartialOrd, Eq, Ord)]
#[repr(u16)]
pub enum SyntaxKind {
    Error = 0,

    // Tokens
    Whitespace,
    Comment,
    NewLine,
    LeftParenthesis,
    RightParenthesis,
    LeftBrace,
    RightBrace,
    Comma,
    Colon,
    Plus,
    Minus,
    Asterisk,
    Slash,
    Equals,
    ColonEquals,
    RightArrow,
    Integer,
    Identifier,
    FunctionKeyword,

    // Nodes
    Root,
    VariableDefinition,
    VariableAssignment,
    VariableReference,
    InfixExpression,
    Literal,
    ParenthesisExpression,
    PrefixExpression,
    FunctionDeclaration,
    ParameterList,
    Parameter,
    TypeAnnotation,
    ReturnTypeAnnotation,
    Block,
}

impl SyntaxKind {
    pub fn is_trivia(self) -> bool {
        matches!(self, Self::Whitespace | Self::Comment | Self::NewLine)
    }
}

impl From<SyntaxKind> for rowan::SyntaxKind {
    fn from(kind: SyntaxKind) -> Self {
        Self(kind as u16)
    }
}

impl From<TokenKind> for SyntaxKind {
    fn from(value: TokenKind) -> Self {
        match value {
            TokenKind::Whitespace => SyntaxKind::Whitespace,
            TokenKind::Comment => SyntaxKind::Comment,
            TokenKind::NewLine => SyntaxKind::NewLine,
            TokenKind::Plus => SyntaxKind::Plus,
            TokenKind::Minus => SyntaxKind::Minus,
            TokenKind::Asterisk => SyntaxKind::Asterisk,
            TokenKind::Slash => SyntaxKind::Slash,
            TokenKind::ColonEquals => SyntaxKind::ColonEquals,
            TokenKind::RightArrow => SyntaxKind::RightArrow,
            TokenKind::Identifier => SyntaxKind::Identifier,
            TokenKind::Integer => SyntaxKind::Integer,
            TokenKind::Equals => SyntaxKind::Equals,
            TokenKind::LeftParenthesis => SyntaxKind::LeftParenthesis,
            TokenKind::RightParenthesis => SyntaxKind::RightParenthesis,
            TokenKind::LeftBrace => SyntaxKind::LeftBrace,
            TokenKind::RightBrace => SyntaxKind::RightBrace,
            TokenKind::Comma => SyntaxKind::Comma,
            TokenKind::Colon => SyntaxKind::Colon,
            TokenKind::Function => SyntaxKind::FunctionKeyword,

        }
    }
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum Language {}

pub type SyntaxNode = rowan::SyntaxNode<Language>;
pub type SyntaxToken = rowan::SyntaxToken<Language>;
pub type SyntaxElement = rowan::SyntaxElement<Language>;

impl rowan::Language for Language {
    type Kind = SyntaxKind;

    fn kind_from_raw(raw: rowan::SyntaxKind) -> Self::Kind {
        // TODO: Figure out how to handle invalid kinds, should we panic?
        return Self::Kind::from_u16(raw.0)
            .unwrap_or_else(|| panic!("Invalid syntax kind: {}", raw.0));
    }

    fn kind_to_raw(kind: Self::Kind) -> rowan::SyntaxKind {
        kind.into()
    }
}
