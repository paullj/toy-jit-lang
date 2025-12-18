use lex::TokenKind;
use syntax::SyntaxKind;

use crate::{ParseError, Parser, concat_kinds, marker::CompletedMarker};

pub(crate) fn expression(parser: &mut Parser) -> Option<CompletedMarker> {
    expression_with_binding_power(parser, 0)
}

// All infix operators
pub(crate) const EXPRESSION_OPERATOR_KINDS: &[TokenKind] = &[
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
];

// All tokens that can start an expression (literals, identifiers, prefix ops, parens)
pub(crate) const EXPRESSION_LHS_KINDS: &[TokenKind] = &[
    // Literals - integers
    TokenKind::Integer,
    TokenKind::BinaryInteger,
    TokenKind::OctalInteger,
    TokenKind::HexInteger,
    // Literals - floats
    TokenKind::Float,
    TokenKind::FloatExponent,
    // Literals - booleans
    TokenKind::True,
    TokenKind::False,
    // Literals - strings
    TokenKind::String,
    TokenKind::MultiLineString,
    // Identifiers
    TokenKind::Identifier,
    // Prefix operators
    TokenKind::Minus,
    TokenKind::Bang,
    // Grouping
    TokenKind::LeftParenthesis,
];

pub(crate) const EXPRESSION_KINDS: &[TokenKind] =
    concat_kinds!(EXPRESSION_LHS_KINDS, EXPRESSION_OPERATOR_KINDS);

fn expression_with_binding_power(
    parser: &mut Parser,
    minimum_binding_power: u8,
) -> Option<CompletedMarker> {
    let lhs = lhs(parser)?;

    inner_expression_with_binding_power(parser, lhs, minimum_binding_power)
}

pub(crate) fn inner_expression_with_binding_power(
    parser: &mut Parser,
    mut lhs: CompletedMarker,
    minimum_binding_power: u8,
) -> Option<CompletedMarker> {
    loop {
        let Some(op): Option<Operator> = parser
            .is_at_one_of(EXPRESSION_OPERATOR_KINDS)
            .and_then(|kind| kind.try_into().ok())
        else {
            // We're not at an operator; we don't know what to do next, so we return and let the
            // caller decide.
            break;
        };

        if let Some((left_binding_power, right_binding_power)) = op.infix_binding_power() {
            if left_binding_power < minimum_binding_power {
                break;
            }

            parser.consume();

            let marker = lhs.precede(parser);
            // Eat the operator's token.

            let parsed_rhs = expression_with_binding_power(parser, right_binding_power).is_some();
            lhs = marker.complete(parser, SyntaxKind::InfixExpression);

            if !parsed_rhs {
                break;
            }
        }
    }

    Some(lhs)
}

fn lhs(parser: &mut Parser) -> Option<CompletedMarker> {
    match parser.is_at_one_of(EXPRESSION_LHS_KINDS) {
        // Integer literals
        Some(TokenKind::Integer)
        | Some(TokenKind::BinaryInteger)
        | Some(TokenKind::OctalInteger)
        | Some(TokenKind::HexInteger) => Some(literal(parser)),
        // Float literals
        Some(TokenKind::Float) | Some(TokenKind::FloatExponent) => Some(literal(parser)),
        // Boolean literals
        Some(TokenKind::True) | Some(TokenKind::False) => Some(literal(parser)),
        // String literals
        Some(TokenKind::String) | Some(TokenKind::MultiLineString) => Some(literal(parser)),
        // Identifiers
        Some(TokenKind::Identifier) => Some(variable_reference(parser)),
        // Prefix operators
        Some(TokenKind::Minus) | Some(TokenKind::Bang) => prefix_expression(parser),
        // Parenthesized expressions
        Some(TokenKind::LeftParenthesis) => Some(parenthesis_expression(parser)),
        Some(_) => unreachable!("Parser should only match on EXPRESSION_LHS_KINDS"),
        None => parser.unexpected_token_error(),
    }
}

fn literal(parser: &mut Parser) -> CompletedMarker {
    let marker = parser.start();
    parser.consume();
    marker.complete(parser, SyntaxKind::Literal)
}

fn variable_reference(parser: &mut Parser) -> CompletedMarker {
    assert!(parser.is_at(TokenKind::Identifier));

    let marker = parser.start();
    parser.consume();
    marker.complete(parser, SyntaxKind::VariableReference)
}

fn prefix_expression(parser: &mut Parser) -> Option<CompletedMarker> {
    let op = match parser.is_at_one_of(&[TokenKind::Minus, TokenKind::Bang]) {
        Some(TokenKind::Minus) => Operator::Minus,
        Some(TokenKind::Bang) => Operator::Bang,
        _ => return None,
    };

    if let Some(((), right_binding_power)) = op.prefix_binding_power() {
        let marker = parser.start();
        // Eat the operator's token.
        parser.consume();

        expression_with_binding_power(parser, right_binding_power);
        Some(marker.complete(parser, SyntaxKind::PrefixExpression))
    } else {
        None
    }
}

fn parenthesis_expression(parser: &mut Parser) -> CompletedMarker {
    assert!(parser.is_at(TokenKind::LeftParenthesis));

    let marker = parser.start();
    parser.consume();
    expression_with_binding_power(parser, 0);
    parser.expect(TokenKind::RightParenthesis, |ctx| {
        ParseError::UnexpectedToken {
            at: ctx.at.clone().into(),
            expected: "closing parenthesis ')'".to_string(),
            found: ctx.found_string(),
        }
    });

    marker.complete(parser, SyntaxKind::ParenthesisExpression)
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
