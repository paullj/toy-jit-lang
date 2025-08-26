use lex::TokenKind;
use syntax::SyntaxKind;

use crate::{Parser, marker::CompletedMarker};

pub(crate) fn expression(parser: &mut Parser) -> Option<CompletedMarker> {
    expression_with_binding_power(parser, 0)
}

fn expression_with_binding_power(
    parser: &mut Parser,
    minimum_binding_power: u8,
) -> Option<CompletedMarker> {
    let mut lhs = lhs(parser)?;

    loop {
        let op = if parser.is_at(TokenKind::Plus) {
            Operator::Plus
        } else if parser.is_at(TokenKind::Minus) {
            Operator::Minus
        } else if parser.is_at(TokenKind::Asterisk) {
            Operator::Asterisk
        } else if parser.is_at(TokenKind::Slash) {
            Operator::Slash
        } else {
            // We’re not at an operator; we don’t know what to do next, so we return and let the
            // caller decide.
            break;
        };

        if let Some((left_binding_power, right_binding_power)) = op.infix_binding_power() {
            if left_binding_power < minimum_binding_power {
                break;
            }

            // Eat the operator’s token.
            parser.consume();

            let marker = lhs.precede(parser);
            let parsed_rhs = expression_with_binding_power(parser, right_binding_power).is_some();
            lhs = marker.complete(parser, SyntaxKind::InfixExpr);

            if !parsed_rhs {
                break;
            }
        }
    }

    Some(lhs)
}

fn lhs(parser: &mut Parser) -> Option<CompletedMarker> {
    if parser.is_at(TokenKind::Integer) {
        Some(literal(parser))
    } else if parser.is_at(TokenKind::Identifier) {
        Some(variable_reference(parser))
    } else if parser.is_at(TokenKind::Minus) {
        prefix_expression(parser)
    } else if parser.is_at(TokenKind::LeftParenthesis) {
        Some(parenthesis_expression(parser))
    } else {
        parser.error(crate::ParseError::UnexpectedToken);
        None
    }
}

fn literal(parser: &mut Parser) -> CompletedMarker {
    assert!(parser.is_at(TokenKind::Integer));

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
    assert!(parser.is_at(TokenKind::Minus));

    let op = Operator::Minus;
    if let Some(((), right_binding_power)) = op.prefix_binding_power() {
        let marker = parser.start();
        // Eat the operator’s token.
        parser.consume();

        expression_with_binding_power(parser, right_binding_power);
        Some(marker.complete(parser, SyntaxKind::PrefixExpr))
    } else {
        None
    }
}

fn parenthesis_expression(parser: &mut Parser) -> CompletedMarker {
    assert!(parser.is_at(TokenKind::LeftParenthesis));

    let marker = parser.start();
    parser.consume();
    expression_with_binding_power(parser, 0);
    parser.expect(
        TokenKind::RightParenthesis,
        crate::ParseError::UnexpectedToken,
    );

    marker.complete(parser, SyntaxKind::ParenExpr)
}

// FIXME: Sort out this and syntax
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Operator {
    Plus,
    Minus,
    Asterisk,
    Slash,
    Bang,
    Less,
    Greater,
    Equal,
    BangEqual,
    LessEqual,
    GreaterEqual,
    EqualEqual,
    And,
    Or,
    Call,
    For,
    Fn,
    Return,
    Echo,
    Field,
    Let,
    Loop,
    While,
    Block,
    Root,
    If,
}

impl Operator {
    pub fn prefix_binding_power(&self) -> Option<((), u8)> {
        let result = match &self {
            Self::Return | Self::Echo => ((), 1),
            Self::Bang | Self::Minus => ((), 11),
            _ => return None,
        };
        Some(result)
    }

    pub fn postfix_binding_power(&self) -> Option<(u8, ())> {
        let result = match &self {
            Self::Call => (13, ()),
            _ => return None,
        };
        Some(result)
    }

    pub fn infix_binding_power(&self) -> Option<(u8, u8)> {
        let result = match &self {
            Self::Equal => (2, 1),
            Self::And | Self::Or => (3, 4),
            Self::BangEqual
            | Self::EqualEqual
            | Self::Less
            | Self::LessEqual
            | Self::Greater
            | Self::GreaterEqual => (5, 6),
            Self::Plus | Self::Minus => (7, 8),
            Self::Asterisk | Self::Slash => (9, 10),
            Self::Field => (16, 15),
            _ => return None,
        };
        Some(result)
    }
}
