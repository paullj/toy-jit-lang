use lex::TokenKind;
use syntax::SyntaxKind;

use super::*;
use crate::{Parser, TokenSet, marker::CompletedMarker};

/// Recovery set for item-level parsing
const ITEM_RECOVERY: TokenSet = EXPR_FIRST
    .union(TokenSet::single(TokenKind::NewLine))
    .union(TokenSet::single(TokenKind::Identifier))
    .union(TokenSet::single(TokenKind::Fn))
    .union(TokenSet::single(TokenKind::Return))
    .union(TokenSet::single(TokenKind::Echo));

pub(crate) fn item(p: &mut Parser) -> Option<CompletedMarker> {
    match p.current() {
        Some(TokenKind::Fn) => Some(function_definition_or_expression(p)),
        Some(TokenKind::Return) => Some(return_statement(p)),
        Some(TokenKind::Echo) => Some(echo_statement(p)),
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
                _ => {
                    // Identifier possibly followed by call or operators -> treat as expression
                    let mut lhs = m.complete(p, SyntaxKind::VariableReference);

                    // Handle postfix operations (call expressions) - same logic as expression_with_binding_power
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

                    // Then handle infix operators
                    inner_expression_with_binding_power(p, lhs, 0)
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

fn return_statement(p: &mut Parser) -> CompletedMarker {
    debug_assert!(p.at(TokenKind::Return));
    let m = p.start();
    p.consume(); // eat 'return'

    // Optional return value
    if p.current().is_some_and(|k| EXPR_FIRST.contains(k)) && !p.at_newline_terminator() {
        expression(p);
    }

    m.complete(p, SyntaxKind::ReturnStatement)
}

fn echo_statement(p: &mut Parser) -> CompletedMarker {
    debug_assert!(p.at(TokenKind::Echo));
    let m = p.start();
    p.consume(); // eat 'echo'

    // Required expression to print
    if p.current().is_some_and(|k| EXPR_FIRST.contains(k)) && !p.at_newline_terminator() {
        expression(p);
    }

    m.complete(p, SyntaxKind::EchoStatement)
}
