use std::panic;

use lex::TokenKind;
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::FromPrimitive;

#[derive(Debug, Copy, Clone, PartialEq, FromPrimitive, ToPrimitive, Hash, PartialOrd, Eq, Ord)]
#[repr(u16)]
pub enum SyntaxKind {
    Root = 0,

    // Trivia tokens
    Whitespace,
    Comment,
    EOL,

    // Tokens
    LeftParenthesis,
    RightParenthesis,

    Plus,
    Minus,
    Asterisk,
    Slash,

    ColonEquals,

    Echo,

    Integer,
    Identifier,

    // Nodes
    VariableDefinition,
}

impl SyntaxKind {
    pub fn is_trivia(self) -> bool {
        matches!(self, Self::Whitespace | Self::Comment | Self::EOL)
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
            TokenKind::EOL => SyntaxKind::EOL,
            TokenKind::Plus => SyntaxKind::Plus,
            TokenKind::Minus => SyntaxKind::Minus,
            TokenKind::Asterisk => SyntaxKind::Asterisk,
            TokenKind::Slash => SyntaxKind::Slash,
            TokenKind::ColonEquals => SyntaxKind::ColonEquals,
            TokenKind::Echo => SyntaxKind::Echo,
            TokenKind::Identifier => SyntaxKind::Identifier,
            TokenKind::Integer => SyntaxKind::Integer,
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
