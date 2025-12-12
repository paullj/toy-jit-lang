use lex::TokenKind;
use syntax::SyntaxKind;

use crate::{ParseError, Parser, concat_kinds, marker::CompletedMarker};

pub(crate) fn expression(parser: &mut Parser) -> Option<CompletedMarker> {
    expression_with_binding_power(parser, 0)
}

pub(crate) const EXPRESSION_OPERATOR_KINDS: &[TokenKind] = &[
    TokenKind::Plus,
    TokenKind::Minus,
    TokenKind::Asterisk,
    TokenKind::Slash,
];

pub(crate) const EXPRESSION_LHS_KINDS: &[TokenKind] = &[
    TokenKind::Integer,
    TokenKind::Identifier,
    TokenKind::Minus,
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
            .is_at_one_of(&EXPRESSION_OPERATOR_KINDS)
            .and_then(|kind| kind.try_into().ok())
        else {
            // We’re not at an operator; we don’t know what to do next, so we return and let the
            // caller decide.
            break;
        };

        if let Some((left_binding_power, right_binding_power)) = op.infix_binding_power() {
            if left_binding_power < minimum_binding_power {
                break;
            }

            parser.consume();

            let marker = lhs.precede(parser);
            // Eat the operator’s token.

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
    match parser.is_at_one_of(&EXPRESSION_LHS_KINDS) {
        Some(TokenKind::Integer) => Some(literal(parser)),
        Some(TokenKind::Identifier) => Some(variable_reference(parser)),
        Some(TokenKind::Minus) => prefix_expression(parser),
        Some(TokenKind::LeftParenthesis) => Some(parenthesis_expression(parser)),
        Some(_) => unreachable!("Parser should only match on {:?}", &EXPRESSION_LHS_KINDS),
        None => parser.unexpected_token_error(),
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
    parser.expect(TokenKind::RightParenthesis, |ctx| ParseError::UnexpectedToken {
        at: ctx.at.clone().into(),
        expected: "closing parenthesis ')'".to_string(),
        found: ctx.found_string(),
    });

    marker.complete(parser, SyntaxKind::ParenthesisExpression)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Operator {
    Plus,
    Minus,
    Asterisk,
    Slash,
    // Bang,
    // Less,
    // Greater,
    // Equal,
    // BangEqual,
    // LessEqual,
    // GreaterEqual,
    // EqualEqual,
    // And,
    // Or,
    // Call,
    // For,
    // Fn,
    // Return,
    // Echo,
    // Field,
    // Let,
    // Loop,
    // While,
    // Block,
    // Root,
    // If,
}

impl TryFrom<TokenKind> for Operator {
    type Error = TokenKind;

    fn try_from(value: TokenKind) -> Result<Self, TokenKind> {
        match value {
            TokenKind::Plus => Ok(Self::Plus),
            TokenKind::Minus => Ok(Self::Minus),
            TokenKind::Asterisk => Ok(Self::Asterisk),
            TokenKind::Slash => Ok(Self::Slash),
            _ => Err(value),
        }
    }
}

impl Operator {
    pub fn prefix_binding_power(&self) -> Option<((), u8)> {
        let result = match &self {
            // Self::Return | Self::Echo => ((), 1),
            Self::Minus => ((), 11), // Self::Bang | Self::Minus => ((), 11),
            _ => return None,
        };
        Some(result)
    }

    // pub fn postfix_binding_power(&self) -> Option<(u8, ())> {
    //     let result = match &self {
    //         Self::Call => (13, ()),
    //         _ => return None,
    //     };
    //     Some(result)
    // }

    pub fn infix_binding_power(&self) -> Option<(u8, u8)> {
        let result = match &self {
            // Self::Equal => (2, 1),
            // Self::And | Self::Or => (3, 4),
            // Self::BangEqual
            // | Self::EqualEqual
            // | Self::Less
            // | Self::LessEqual
            // | Self::Greater
            // | Self::GreaterEqual => (5, 6),
            Self::Plus | Self::Minus => (7, 8),
            Self::Asterisk | Self::Slash => (9, 10),
            // Self::Field => (16, 15),
            // _ => return None,
        };
        Some(result)
    }
}
