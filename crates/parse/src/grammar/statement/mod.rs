mod expression;
mod variable_assignment;

pub(crate) use expression::{
    EXPRESSION_KINDS, EXPRESSION_LHS_KINDS, expression, inner_expression_with_binding_power,
};
pub(crate) use variable_assignment::variable_assignment;

use crate::{Parser, concat_kinds, marker::CompletedMarker};

use lex::TokenKind;

const STATEMENT_KINDS: &[TokenKind] = concat_kinds!(&[TokenKind::Identifier], EXPRESSION_LHS_KINDS);

pub(crate) fn statement(parser: &mut Parser) -> Option<CompletedMarker> {
    match parser.is_at_one_of(&STATEMENT_KINDS) {
        Some(TokenKind::Identifier) => {
            let marker = parser.start();
            Some(variable_assignment(parser, marker))
        }
        Some(_) => expression(parser),
        None => todo!("Handle end of token stream"),
    }
}
