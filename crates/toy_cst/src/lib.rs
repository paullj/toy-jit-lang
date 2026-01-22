use cstree::Syntax;
use toy_lexer::TokenKind;

// Re-exports for parser
pub use cstree::green::GreenNode;
pub use cstree::interning::TokenInterner;
pub use cstree::text::TextRange;

/// Builder for constructing syntax trees
pub type Builder = cstree::build::GreenNodeBuilder<'static, 'static, SyntaxKind>;

pub type SyntaxNode = cstree::syntax::SyntaxNode<SyntaxKind>;
pub type SyntaxToken = cstree::syntax::SyntaxToken<SyntaxKind>;
pub type SyntaxElement = cstree::syntax::SyntaxElement<SyntaxKind>;

/// Resolved node with access to string interner (for parser output)
pub type ResolvedNode = cstree::syntax::ResolvedNode<SyntaxKind>;

/// Resolved token with access to string interner
pub type ResolvedToken = cstree::syntax::ResolvedToken<SyntaxKind>;

/// Resolved element (node or token) with access to string interner
pub type ResolvedElement = cstree::syntax::ResolvedElement<SyntaxKind>;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Syntax)]
#[repr(u32)]
pub enum SyntaxKind {
    Error,

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
    Dot,

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
    ForKeyword,
    InKeyword,
    PubKeyword,
    UseKeyword,
    AsKeyword,

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
    ForExpression,
    RangeExpression,
    BreakStatement,
    ContinueStatement,

    // List nodes
    ListExpression,
    IndexExpression,
    SliceExpression,
    IndexAssignment,
    ListType,

    // Tuple nodes
    TupleExpression,
    TupleAccessExpression,

    // Module nodes
    UseStatement,
    ModulePath,
    ImportList,
    ImportItem,
    ImportAlias,
}

impl SyntaxKind {
    pub fn is_trivia(self) -> bool {
        matches!(self, Self::Whitespace | Self::Comment | Self::NewLine)
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
            TokenKind::Dot => SyntaxKind::Dot,

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
            TokenKind::For => SyntaxKind::ForKeyword,
            TokenKind::In => SyntaxKind::InKeyword,
            TokenKind::Pub => SyntaxKind::PubKeyword,
            TokenKind::Use => SyntaxKind::UseKeyword,
            TokenKind::As => SyntaxKind::AsKeyword,

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
