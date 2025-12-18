use lex::TokenKind;
use syntax::SyntaxKind;

use super::*;
use crate::{Parser, TokenSet, marker::CompletedMarker};

/// Recovery set for item-level parsing
const ITEM_RECOVERY: TokenSet = EXPR_FIRST
    .union(TokenSet::single(TokenKind::NewLine))
    .union(TokenSet::single(TokenKind::Identifier));

pub(crate) fn item(p: &mut Parser) -> Option<CompletedMarker> {
    match p.current() {
        Some(TokenKind::Identifier) => {
            let m = p.start();
            p.consume();

            match p.current() {
                Some(TokenKind::Colon) => {
                    p.consume(); // eat ':'
                    if p.at(TokenKind::Equals) {
                        // x := expr (type inference)
                        Some(variable_definition_inferred(p, m))
                    } else {
                        // x: Type = expr (explicit type)
                        Some(variable_definition_typed(p, m))
                    }
                }
                Some(TokenKind::Equals) => Some(variable_assignment(p, m)),
                Some(k) if EXPR_FIRST.contains(k) => {
                    // Identifier followed by expr token -> treat as expression
                    let lhs = m.complete(p, SyntaxKind::VariableReference);
                    inner_expression_with_binding_power(p, lhs, 0)
                }
                _ => {
                    // Just an identifier on its own -> variable reference
                    Some(m.complete(p, SyntaxKind::VariableReference))
                }
            }
        }
        Some(k) if EXPR_FIRST.contains(k) => statement(p),
        _ => {
            p.recover("expected item", ITEM_RECOVERY);
            None
        }
    }
}
