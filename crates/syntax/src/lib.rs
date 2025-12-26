use std::panic;

use lex::TokenKind;
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::FromPrimitive;

#[derive(Debug, Copy, Clone, PartialEq, FromPrimitive, ToPrimitive, Hash, PartialOrd, Eq, Ord)]
#[repr(u16)]
pub enum SyntaxKind {
    Error = 0,

    // Tokens - Trivia
    Whitespace,
    Comment,
    DocComment,
    NewLine,

    // Tokens - Brackets
    LeftParenthesis,
    RightParenthesis,
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,

    // Tokens - Punctuation
    Comma,
    Colon,
    DotDot,

    // Tokens - Arithmetic operators
    Plus,
    Minus,
    Asterisk,
    Slash,
    Percent,

    // Tokens - Float arithmetic operators
    PlusDot,
    MinusDot,
    AsteriskDot,
    SlashDot,

    // Tokens - Comparison operators
    Equals,
    EqualsEquals,
    NotEquals,
    Bang,
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,

    // Tokens - Float comparison operators
    GreaterThanDot,
    LessThanDot,
    GreaterThanOrEqualDot,
    LessThanOrEqualDot,

    // Tokens - Keywords
    AndKeyword,
    OrKeyword,
    TrueKeyword,
    FalseKeyword,
    IfKeyword,
    ElseKeyword,
    FnKeyword,
    ReturnKeyword,
    EchoKeyword,
    LoopKeyword,
    WhileKeyword,
    BreakKeyword,
    ContinueKeyword,

    // Tokens - Literals
    Identifier,
    BinaryInteger,
    OctalInteger,
    HexInteger,
    Integer,
    Float,
    FloatExponent,
    String,
    MultiLineString,

    // Nodes
    Root,
    VariableDefinition,
    VariableAssignment,
    VariableReference,
    InfixExpression,
    Literal,
    ParenthesisExpression,
    PrefixExpression,
    TypeAnnotation,
    BlockExpression,
    IfExpression,
    FunctionDefinition,
    FunctionExpression,
    ParameterList,
    Parameter,
    CallExpression,
    ReturnStatement,
    EchoStatement,
    LoopExpression,
    WhileExpression,
    BreakStatement,
    ContinueStatement,

    // List nodes
    ListExpression,
    IndexExpression,
    SliceExpression,
    IndexAssignment,
    ListType,
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
            // Trivia
            TokenKind::Whitespace => SyntaxKind::Whitespace,
            TokenKind::DocComment => SyntaxKind::DocComment,
            TokenKind::Comment => SyntaxKind::Comment,
            TokenKind::NewLine => SyntaxKind::NewLine,

            // Brackets
            TokenKind::LeftParenthesis => SyntaxKind::LeftParenthesis,
            TokenKind::RightParenthesis => SyntaxKind::RightParenthesis,
            TokenKind::LeftBrace => SyntaxKind::LeftBrace,
            TokenKind::RightBrace => SyntaxKind::RightBrace,
            TokenKind::LeftBracket => SyntaxKind::LeftBracket,
            TokenKind::RightBracket => SyntaxKind::RightBracket,

            // Punctuation
            TokenKind::Comma => SyntaxKind::Comma,
            TokenKind::Colon => SyntaxKind::Colon,
            TokenKind::DotDot => SyntaxKind::DotDot,

            // Arithmetic operators
            TokenKind::Plus => SyntaxKind::Plus,
            TokenKind::Minus => SyntaxKind::Minus,
            TokenKind::Asterisk => SyntaxKind::Asterisk,
            TokenKind::Slash => SyntaxKind::Slash,
            TokenKind::Percent => SyntaxKind::Percent,

            // Float arithmetic operators
            TokenKind::PlusDot => SyntaxKind::PlusDot,
            TokenKind::MinusDot => SyntaxKind::MinusDot,
            TokenKind::AsteriskDot => SyntaxKind::AsteriskDot,
            TokenKind::SlashDot => SyntaxKind::SlashDot,

            // Comparison operators
            TokenKind::Equals => SyntaxKind::Equals,
            TokenKind::EqualsEquals => SyntaxKind::EqualsEquals,
            TokenKind::NotEquals => SyntaxKind::NotEquals,
            TokenKind::Bang => SyntaxKind::Bang,
            TokenKind::GreaterThan => SyntaxKind::GreaterThan,
            TokenKind::LessThan => SyntaxKind::LessThan,
            TokenKind::GreaterThanOrEqual => SyntaxKind::GreaterThanOrEqual,
            TokenKind::LessThanOrEqual => SyntaxKind::LessThanOrEqual,

            // Float comparison operators
            TokenKind::GreaterThanDot => SyntaxKind::GreaterThanDot,
            TokenKind::LessThanDot => SyntaxKind::LessThanDot,
            TokenKind::GreaterThanOrEqualDot => SyntaxKind::GreaterThanOrEqualDot,
            TokenKind::LessThanOrEqualDot => SyntaxKind::LessThanOrEqualDot,

            // Keywords
            TokenKind::And => SyntaxKind::AndKeyword,
            TokenKind::Or => SyntaxKind::OrKeyword,
            TokenKind::True => SyntaxKind::TrueKeyword,
            TokenKind::False => SyntaxKind::FalseKeyword,
            TokenKind::If => SyntaxKind::IfKeyword,
            TokenKind::Else => SyntaxKind::ElseKeyword,
            TokenKind::Fn => SyntaxKind::FnKeyword,
            TokenKind::Return => SyntaxKind::ReturnKeyword,
            TokenKind::Echo => SyntaxKind::EchoKeyword,
            TokenKind::Loop => SyntaxKind::LoopKeyword,
            TokenKind::While => SyntaxKind::WhileKeyword,
            TokenKind::Break => SyntaxKind::BreakKeyword,
            TokenKind::Continue => SyntaxKind::ContinueKeyword,

            // Literals
            TokenKind::Identifier => SyntaxKind::Identifier,
            TokenKind::BinaryInteger => SyntaxKind::BinaryInteger,
            TokenKind::OctalInteger => SyntaxKind::OctalInteger,
            TokenKind::HexInteger => SyntaxKind::HexInteger,
            TokenKind::Integer => SyntaxKind::Integer,
            TokenKind::Float => SyntaxKind::Float,
            TokenKind::FloatExponent => SyntaxKind::FloatExponent,
            TokenKind::String => SyntaxKind::String,
            TokenKind::MultiLineString => SyntaxKind::MultiLineString,
        }
    }
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum Language {}

pub type SyntaxNode = rowan::SyntaxNode<Language>;
pub type SyntaxToken = rowan::SyntaxToken<Language>;
pub type SyntaxElement = rowan::SyntaxElement<Language>;
pub type TextRange = rowan::TextRange;

impl rowan::Language for Language {
    type Kind = SyntaxKind;

    fn kind_from_raw(raw: rowan::SyntaxKind) -> Self::Kind {
        Self::Kind::from_u16(raw.0).unwrap_or_else(|| panic!("Invalid syntax kind: {}", raw.0))
    }

    fn kind_to_raw(kind: Self::Kind) -> rowan::SyntaxKind {
        kind.into()
    }
}
