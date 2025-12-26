pub(crate) mod expression;
mod variable_assignment;

pub(crate) use expression::{
    EXPR_FIRST, call_expression, expression, index_or_slice_expression,
    inner_expression_with_binding_power,
};
pub(crate) use variable_assignment::variable_assignment;

use crate::{Parser, TokenSet, marker::CompletedMarker};
use lex::TokenKind;

const STMT_FIRST: TokenSet = EXPR_FIRST.union(TokenSet::single(TokenKind::Identifier));

pub(crate) fn statement(p: &mut Parser) -> Option<CompletedMarker> {
    match p.current() {
        Some(TokenKind::Identifier) => {
            let m = p.start();
            Some(variable_assignment(p, m))
        }
        Some(k) if EXPR_FIRST.contains(k) => expression(p),
        _ => {
            p.recover("expected statement", STMT_FIRST);
            None
        }
    }
}
