use toy_cst::SyntaxKind;
use toy_lexer::TokenKind;

use super::*;
use crate::{Parser, TokenSet, marker::CompletedMarker};

/// Recovery set for item-level parsing
const ITEM_RECOVERY: TokenSet = EXPR_FIRST
    .union(TokenSet::single(TokenKind::NewLine))
    .union(TokenSet::single(TokenKind::Identifier))
    .union(TokenSet::single(TokenKind::Fn))
    .union(TokenSet::single(TokenKind::Return))
    .union(TokenSet::single(TokenKind::Echo))
    .union(TokenSet::single(TokenKind::Break))
    .union(TokenSet::single(TokenKind::Continue))
    .union(TokenSet::single(TokenKind::Use))
    .union(TokenSet::single(TokenKind::Pub));

pub(crate) fn item(p: &mut Parser) -> Option<CompletedMarker> {
    match p.current() {
        Some(TokenKind::Use) => Some(use_statement(p, false)),
        Some(TokenKind::Pub) => {
            p.consume(); // eat 'pub'
            // Check what follows pub
            match p.current() {
                Some(TokenKind::Use) => Some(use_statement(p, true)),
                Some(TokenKind::Fn) => Some(function_definition_or_expression_pub(p, true)),
                _ => {
                    // Invalid pub usage
                    let span = p.current_span();
                    let found = p.current().map(|k| k.to_string());
                    p.error(crate::ParseError::UnexpectedToken {
                        at: span.into(),
                        expected: "'use' or 'fn' after 'pub'".to_string(),
                        found,
                    });
                    None
                }
            }
        }
        Some(TokenKind::Fn) => Some(function_definition_or_expression_pub(p, false)),
        Some(TokenKind::Return) => Some(return_statement(p)),
        Some(TokenKind::Echo) => Some(echo_statement(p)),
        Some(TokenKind::Break) => Some(break_statement(p)),
        Some(TokenKind::Continue) => Some(continue_statement(p)),
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
                Some(TokenKind::LeftBracket) => {
                    // x[...] - could be index expression or index assignment
                    let var_ref = m.complete(p, SyntaxKind::VariableReference);
                    let index_m = var_ref.precede(p);
                    p.consume(); // eat '['
                    p.enter_delimiter();

                    // Check for slice vs index
                    let is_slice = if p.at(TokenKind::DotDot) {
                        p.consume();
                        if !p.at(TokenKind::RightBracket) {
                            expression(p);
                        }
                        true
                    } else if !p.at(TokenKind::RightBracket) {
                        expression(p);
                        if p.at(TokenKind::DotDot) {
                            p.consume();
                            if !p.at(TokenKind::RightBracket) {
                                expression(p);
                            }
                            true
                        } else {
                            false
                        }
                    } else {
                        false
                    };

                    p.exit_delimiter();
                    if !p.eat(TokenKind::RightBracket) {
                        let span = p.current_span();
                        let found = p.current().map(|k| k.to_string());
                        p.error(crate::ParseError::UnexpectedToken {
                            at: span.into(),
                            expected: "']'".to_string(),
                            found,
                        });
                    }

                    let kind = if is_slice {
                        SyntaxKind::SliceExpression
                    } else {
                        SyntaxKind::IndexExpression
                    };
                    let mut lhs = index_m.complete(p, kind);

                    // Handle chained postfix operations (more indexes, calls)
                    loop {
                        // Check for newline terminator, but allow `[` to continue if index
                        if p.at_newline_terminator() {
                            if p.at(TokenKind::LeftBracket) && p.is_bracket_index_not_list() {
                                // Continue - this is an index expression
                            } else {
                                break;
                            }
                        }

                        if p.at(TokenKind::LeftParenthesis) {
                            lhs = call_expression(p, lhs);
                        } else if p.at(TokenKind::LeftBracket) {
                            lhs = index_or_slice_expression(p, lhs);
                        } else {
                            break;
                        }
                    }

                    // Now check if this is an assignment
                    if p.at(TokenKind::Equals) {
                        let assign_m = lhs.precede(p);
                        p.consume(); // eat '='
                        expression(p);
                        Some(assign_m.complete(p, SyntaxKind::IndexAssignment))
                    } else {
                        // Just an expression, continue with infix operators
                        inner_expression_with_binding_power(p, lhs, 0)
                    }
                }
                _ => {
                    // Identifier possibly followed by call/index or operators -> treat as expression
                    let mut lhs = m.complete(p, SyntaxKind::VariableReference);

                    // Handle postfix operations (call expressions, indexes)
                    loop {
                        // Check for newline terminator, but allow `[` to continue if index
                        if p.at_newline_terminator() {
                            if p.at(TokenKind::LeftBracket) && p.is_bracket_index_not_list() {
                                // Continue - this is an index expression
                            } else {
                                break;
                            }
                        }

                        if p.at(TokenKind::LeftParenthesis) {
                            lhs = call_expression(p, lhs);
                        } else if p.at(TokenKind::LeftBracket) {
                            lhs = index_or_slice_expression(p, lhs);
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

fn break_statement(p: &mut Parser) -> CompletedMarker {
    debug_assert!(p.at(TokenKind::Break));
    let m = p.start();
    p.consume();

    if p.at(TokenKind::Identifier) && !p.at_newline_terminator() {
        p.consume();
    }

    m.complete(p, SyntaxKind::BreakStatement)
}

fn continue_statement(p: &mut Parser) -> CompletedMarker {
    debug_assert!(p.at(TokenKind::Continue));
    let m = p.start();
    p.consume();

    if p.at(TokenKind::Identifier) && !p.at_newline_terminator() {
        p.consume();
    }

    m.complete(p, SyntaxKind::ContinueStatement)
}
