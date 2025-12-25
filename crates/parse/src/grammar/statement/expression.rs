use lex::TokenKind;
use syntax::SyntaxKind;

use crate::{Parser, TokenSet, marker::CompletedMarker};

pub(crate) fn expression(parser: &mut Parser) -> Option<CompletedMarker> {
    expression_with_binding_power(parser, 0)
}

/// All infix operators (used for documentation)
#[allow(dead_code)]
const OPERATOR_SET: TokenSet = TokenSet::new(&[
    // Int arithmetic
    TokenKind::Plus,
    TokenKind::Minus,
    TokenKind::Asterisk,
    TokenKind::Slash,
    TokenKind::Percent,
    // Float arithmetic
    TokenKind::PlusDot,
    TokenKind::MinusDot,
    TokenKind::AsteriskDot,
    TokenKind::SlashDot,
    // Comparison
    TokenKind::EqualsEquals,
    TokenKind::NotEquals,
    TokenKind::GreaterThan,
    TokenKind::LessThan,
    TokenKind::GreaterThanOrEqual,
    TokenKind::LessThanOrEqual,
    // Float comparison
    TokenKind::GreaterThanDot,
    TokenKind::LessThanDot,
    TokenKind::GreaterThanOrEqualDot,
    TokenKind::LessThanOrEqualDot,
    // Boolean
    TokenKind::And,
    TokenKind::Or,
]);

/// Literal tokens
const LITERAL_SET: TokenSet = TokenSet::new(&[
    TokenKind::Integer,
    TokenKind::BinaryInteger,
    TokenKind::OctalInteger,
    TokenKind::HexInteger,
    TokenKind::Float,
    TokenKind::FloatExponent,
    TokenKind::True,
    TokenKind::False,
    TokenKind::String,
    TokenKind::MultiLineString,
]);

/// Prefix operators
const PREFIX_SET: TokenSet = TokenSet::new(&[TokenKind::Minus, TokenKind::Bang]);

/// All tokens that can start an expression
pub(crate) const EXPR_FIRST: TokenSet = LITERAL_SET
    .union(TokenSet::single(TokenKind::Identifier))
    .union(PREFIX_SET)
    .union(TokenSet::single(TokenKind::LeftParenthesis))
    .union(TokenSet::single(TokenKind::LeftBrace))
    .union(TokenSet::single(TokenKind::If))
    .union(TokenSet::single(TokenKind::Fn))
    .union(TokenSet::single(TokenKind::Loop))
    .union(TokenSet::single(TokenKind::While));

/// Recovery set for expression parsing (skip to newline or expr start)
const EXPR_RECOVERY: TokenSet = EXPR_FIRST.union(TokenSet::single(TokenKind::NewLine));

fn expression_with_binding_power(p: &mut Parser, min_bp: u8) -> Option<CompletedMarker> {
    let mut lhs = lhs(p)?;

    // Handle postfix operations (call expressions)
    loop {
        if p.at_newline_terminator() {
            break;
        }

        if p.at(TokenKind::LeftParenthesis) {
            lhs = call_expression(p, lhs);
        } else {
            break;
        }
    }

    inner_expression_with_binding_power(p, lhs, min_bp)
}

pub(crate) fn call_expression(p: &mut Parser, callee: CompletedMarker) -> CompletedMarker {
    debug_assert!(p.at(TokenKind::LeftParenthesis));

    let m = callee.precede(p);
    p.consume(); // eat '('
    p.enter_delimiter();

    let mut first = true;
    while !p.at(TokenKind::RightParenthesis) && !p.is_at_end() {
        if !first && !p.eat(TokenKind::Comma) {
            break;
        }
        first = false;

        // Handle trailing comma
        if p.at(TokenKind::RightParenthesis) {
            break;
        }

        expression(p);
    }

    p.exit_delimiter();
    if !p.eat(TokenKind::RightParenthesis) {
        let span = p.current_span();
        let found = p.current().map(|k| k.to_string());
        p.error(crate::ParseError::UnexpectedToken {
            at: span.into(),
            expected: "')'".to_string(),
            found,
        });
    }

    m.complete(p, SyntaxKind::CallExpression)
}

pub(crate) fn inner_expression_with_binding_power(
    p: &mut Parser,
    mut lhs: CompletedMarker,
    min_bp: u8,
) -> Option<CompletedMarker> {
    while let Some(kind) = p.current() {
        // Newline terminates expression (unless inside delimiters)
        if p.at_newline_terminator() {
            break;
        }

        // Check for operator
        let Ok(op) = Operator::try_from(kind) else {
            break;
        };

        let Some((l_bp, r_bp)) = op.infix_binding_power() else {
            break;
        };
        if l_bp < min_bp {
            break;
        }

        let op_text = op.symbol();
        p.consume(); // eat operator

        // After operator, check if newline terminates or EOF (incomplete expression)
        if p.at_newline_terminator() || p.is_at_end() {
            let span = p.current_span();
            p.error(crate::ParseError::IncompleteExpression {
                at: span.into(),
                operator: op_text.to_string(),
            });
            // Complete partial infix expression
            lhs = lhs.precede(p).complete(p, SyntaxKind::InfixExpression);
            break;
        }

        let marker = lhs.precede(p);
        let parsed_rhs = expression_with_binding_power(p, r_bp).is_some();
        lhs = marker.complete(p, SyntaxKind::InfixExpression);

        if !parsed_rhs {
            // Incomplete expression - no RHS found
            let span = p.current_span();
            p.error(crate::ParseError::IncompleteExpression {
                at: span.into(),
                operator: op_text.to_string(),
            });
            break;
        }
    }

    Some(lhs)
}

fn lhs(p: &mut Parser) -> Option<CompletedMarker> {
    match p.current() {
        Some(k) if LITERAL_SET.contains(k) => Some(literal(p)),
        Some(TokenKind::Identifier) => Some(variable_reference(p)),
        Some(TokenKind::Minus) | Some(TokenKind::Bang) => prefix_expression(p),
        Some(TokenKind::LeftParenthesis) => Some(parenthesis_expression(p)),
        Some(TokenKind::LeftBrace) => Some(block_expression(p)),
        Some(TokenKind::If) => Some(if_expression(p)),
        Some(TokenKind::Fn) => Some(crate::grammar::function_definition_or_expression(p)),
        Some(TokenKind::Loop) => Some(loop_expression(p)),
        Some(TokenKind::While) => Some(while_expression(p)),
        _ => {
            p.recover("expected expression", EXPR_RECOVERY);
            None
        }
    }
}

fn literal(p: &mut Parser) -> CompletedMarker {
    let m = p.start();
    p.consume();
    m.complete(p, SyntaxKind::Literal)
}

fn variable_reference(p: &mut Parser) -> CompletedMarker {
    debug_assert!(p.at(TokenKind::Identifier));
    let m = p.start();
    p.consume();
    m.complete(p, SyntaxKind::VariableReference)
}

fn prefix_expression(p: &mut Parser) -> Option<CompletedMarker> {
    let op = match p.current() {
        Some(TokenKind::Minus) => Operator::Minus,
        Some(TokenKind::Bang) => Operator::Bang,
        _ => return None,
    };

    let ((), r_bp) = op.prefix_binding_power()?;

    let m = p.start();
    p.consume(); // eat operator
    expression_with_binding_power(p, r_bp);
    Some(m.complete(p, SyntaxKind::PrefixExpression))
}

fn parenthesis_expression(p: &mut Parser) -> CompletedMarker {
    debug_assert!(p.at(TokenKind::LeftParenthesis));

    let m = p.start();
    p.consume(); // eat '('
    p.enter_delimiter(); // newlines allowed inside parens

    expression_with_binding_power(p, 0);

    p.exit_delimiter();
    if !p.eat(TokenKind::RightParenthesis) {
        // IDE-friendly: emit error but still complete the node
        let span = p.current_span();
        let found = p.current().map(|k| k.to_string());
        p.error(crate::ParseError::UnexpectedToken {
            at: span.into(),
            expected: "')'".to_string(),
            found,
        });
    }

    m.complete(p, SyntaxKind::ParenthesisExpression)
}

pub(crate) fn block_expression(p: &mut Parser) -> CompletedMarker {
    debug_assert!(p.at(TokenKind::LeftBrace));

    let m = p.start();
    p.consume(); // eat '{'
    p.enter_delimiter(); // newlines allowed inside braces

    // Parse statements until '}'
    while !p.at(TokenKind::RightBrace) && !p.is_at_end() {
        // Skip newlines between statements
        while p.eat(TokenKind::NewLine) {}

        if p.at(TokenKind::RightBrace) {
            break;
        }

        // Parse an item (definition, assignment, or expression)
        if crate::grammar::item(p).is_none() {
            break;
        }
    }

    p.exit_delimiter();
    if !p.eat(TokenKind::RightBrace) {
        let span = p.current_span();
        let found = p.current().map(|k| k.to_string());
        p.error(crate::ParseError::UnexpectedToken {
            at: span.into(),
            expected: "'}'".to_string(),
            found,
        });
    }

    m.complete(p, SyntaxKind::BlockExpression)
}

fn if_expression(p: &mut Parser) -> CompletedMarker {
    debug_assert!(p.at(TokenKind::If));

    let m = p.start();
    p.consume(); // eat 'if'

    // Parse condition
    expression(p);

    // Expect '{' for then branch
    if !p.at(TokenKind::LeftBrace) {
        let span = p.current_span();
        let found = p.current().map(|k| k.to_string());
        p.error(crate::ParseError::UnexpectedToken {
            at: span.into(),
            expected: "'{'".to_string(),
            found,
        });
    }
    block_expression(p);

    // Check for else (allows newline before else)
    if p.eat(TokenKind::Else) {
        if p.at(TokenKind::If) {
            // else if
            if_expression(p);
        } else {
            // else block
            if !p.at(TokenKind::LeftBrace) {
                let span = p.current_span();
                let found = p.current().map(|k| k.to_string());
                p.error(crate::ParseError::UnexpectedToken {
                    at: span.into(),
                    expected: "'{'".to_string(),
                    found,
                });
            }
            block_expression(p);
        }
    }

    m.complete(p, SyntaxKind::IfExpression)
}

fn loop_expression(p: &mut Parser) -> CompletedMarker {
    debug_assert!(p.at(TokenKind::Loop));
    let m = p.start();
    p.consume();

    if p.at(TokenKind::Colon) {
        p.consume();
        if !p.eat(TokenKind::Identifier) {
            let span = p.current_span();
            let found = p.current().map(|k| k.to_string());
            p.error(crate::ParseError::UnexpectedToken {
                at: span.into(),
                expected: "label identifier".to_string(),
                found,
            });
        }
    }

    if !p.at(TokenKind::LeftBrace) {
        let span = p.current_span();
        let found = p.current().map(|k| k.to_string());
        p.error(crate::ParseError::UnexpectedToken {
            at: span.into(),
            expected: "'{'".to_string(),
            found,
        });
    }
    block_expression(p);

    m.complete(p, SyntaxKind::LoopExpression)
}

fn while_expression(p: &mut Parser) -> CompletedMarker {
    debug_assert!(p.at(TokenKind::While));
    let m = p.start();
    p.consume();

    expression(p);

    if p.at(TokenKind::Colon) {
        p.consume();
        if !p.eat(TokenKind::Identifier) {
            let span = p.current_span();
            let found = p.current().map(|k| k.to_string());
            p.error(crate::ParseError::UnexpectedToken {
                at: span.into(),
                expected: "label identifier".to_string(),
                found,
            });
        }
    }

    if !p.at(TokenKind::LeftBrace) {
        let span = p.current_span();
        let found = p.current().map(|k| k.to_string());
        p.error(crate::ParseError::UnexpectedToken {
            at: span.into(),
            expected: "'{'".to_string(),
            found,
        });
    }
    block_expression(p);

    m.complete(p, SyntaxKind::WhileExpression)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Operator {
    // Int arithmetic
    Plus,
    Minus,
    Asterisk,
    Slash,
    Percent,
    // Float arithmetic
    PlusDot,
    MinusDot,
    AsteriskDot,
    SlashDot,
    // Comparison
    EqualsEquals,
    NotEquals,
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
    // Float comparison
    GreaterThanDot,
    LessThanDot,
    GreaterThanOrEqualDot,
    LessThanOrEqualDot,
    // Boolean
    And,
    Or,
    Bang,
}

impl TryFrom<TokenKind> for Operator {
    type Error = TokenKind;

    fn try_from(value: TokenKind) -> Result<Self, TokenKind> {
        match value {
            // Int arithmetic
            TokenKind::Plus => Ok(Self::Plus),
            TokenKind::Minus => Ok(Self::Minus),
            TokenKind::Asterisk => Ok(Self::Asterisk),
            TokenKind::Slash => Ok(Self::Slash),
            TokenKind::Percent => Ok(Self::Percent),
            // Float arithmetic
            TokenKind::PlusDot => Ok(Self::PlusDot),
            TokenKind::MinusDot => Ok(Self::MinusDot),
            TokenKind::AsteriskDot => Ok(Self::AsteriskDot),
            TokenKind::SlashDot => Ok(Self::SlashDot),
            // Comparison
            TokenKind::EqualsEquals => Ok(Self::EqualsEquals),
            TokenKind::NotEquals => Ok(Self::NotEquals),
            TokenKind::GreaterThan => Ok(Self::GreaterThan),
            TokenKind::LessThan => Ok(Self::LessThan),
            TokenKind::GreaterThanOrEqual => Ok(Self::GreaterThanOrEqual),
            TokenKind::LessThanOrEqual => Ok(Self::LessThanOrEqual),
            // Float comparison
            TokenKind::GreaterThanDot => Ok(Self::GreaterThanDot),
            TokenKind::LessThanDot => Ok(Self::LessThanDot),
            TokenKind::GreaterThanOrEqualDot => Ok(Self::GreaterThanOrEqualDot),
            TokenKind::LessThanOrEqualDot => Ok(Self::LessThanOrEqualDot),
            // Boolean
            TokenKind::And => Ok(Self::And),
            TokenKind::Or => Ok(Self::Or),
            TokenKind::Bang => Ok(Self::Bang),
            _ => Err(value),
        }
    }
}

impl Operator {
    pub fn symbol(&self) -> &'static str {
        match self {
            Self::Plus => "+",
            Self::Minus => "-",
            Self::Asterisk => "*",
            Self::Slash => "/",
            Self::Percent => "%",
            Self::PlusDot => "+.",
            Self::MinusDot => "-.",
            Self::AsteriskDot => "*.",
            Self::SlashDot => "/.",
            Self::EqualsEquals => "==",
            Self::NotEquals => "!=",
            Self::GreaterThan => ">",
            Self::LessThan => "<",
            Self::GreaterThanOrEqual => ">=",
            Self::LessThanOrEqual => "<=",
            Self::GreaterThanDot => ">.",
            Self::LessThanDot => "<.",
            Self::GreaterThanOrEqualDot => ">=.",
            Self::LessThanOrEqualDot => "<=.",
            Self::And => "and",
            Self::Or => "or",
            Self::Bang => "!",
        }
    }

    pub fn prefix_binding_power(&self) -> Option<((), u8)> {
        let result = match &self {
            Self::Bang | Self::Minus => ((), 15),
            _ => return None,
        };
        Some(result)
    }

    pub fn infix_binding_power(&self) -> Option<(u8, u8)> {
        // Binding powers (higher = tighter binding):
        // 1-2: or
        // 3-4: and
        // 5-6: equality (==, !=)
        // 7-8: comparison (>, <, >=, <=, and float variants)
        // 9-10: additive (+, -, +., -.)
        // 11-12: multiplicative (*, /, %, *., /.)
        let result = match &self {
            // Boolean - lowest precedence
            Self::Or => (1, 2),
            Self::And => (3, 4),
            // Equality
            Self::EqualsEquals | Self::NotEquals => (5, 6),
            // Comparison (int and float)
            Self::GreaterThan
            | Self::LessThan
            | Self::GreaterThanOrEqual
            | Self::LessThanOrEqual
            | Self::GreaterThanDot
            | Self::LessThanDot
            | Self::GreaterThanOrEqualDot
            | Self::LessThanOrEqualDot => (7, 8),
            // Additive (int and float)
            Self::Plus | Self::Minus | Self::PlusDot | Self::MinusDot => (9, 10),
            // Multiplicative (int and float)
            Self::Asterisk | Self::Slash | Self::Percent | Self::AsteriskDot | Self::SlashDot => {
                (11, 12)
            }
            // Bang is prefix-only
            Self::Bang => return None,
        };
        Some(result)
    }
}
