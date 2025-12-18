use lex::TokenKind;
use syntax::SyntaxKind;

use super::*;
use crate::{Parser, concat_kinds, marker::CompletedMarker};

pub(crate) const ITEM_KINDS: &[TokenKind] = concat_kinds!(DEFINITION_KINDS, EXPRESSION_LHS_KINDS);
pub(crate) const ITEM_VARIABLE_KINDS: &[TokenKind] =
    concat_kinds!(&[TokenKind::Colon, TokenKind::Equals], EXPRESSION_KINDS);

pub(crate) fn item(parser: &mut Parser) -> Option<CompletedMarker> {
    match parser.is_at_one_of(ITEM_KINDS) {
        // NOTE: Special case for handling identifiers since we do not have a dedicated keyword for defining variables
        Some(TokenKind::Identifier) => {
            let marker = parser.start();
            parser.consume();

            match parser.is_at_one_of(ITEM_VARIABLE_KINDS) {
                Some(TokenKind::Colon) => {
                    // Consume the ':'
                    parser.consume();

                    if parser.is_at(TokenKind::Equals) {
                        // x := expr (type inference)
                        Some(variable_definition_inferred(parser, marker))
                    } else {
                        // x: Type = expr (explicit type)
                        Some(variable_definition_typed(parser, marker))
                    }
                }
                Some(TokenKind::Equals) => Some(variable_assignment(parser, marker)),
                Some(_) => {
                    let lhs = marker.complete(parser, SyntaxKind::VariableReference);
                    inner_expression_with_binding_power(parser, lhs, 0)
                }
                None => {
                    marker.discard(parser);
                    parser.unexpected_token_error()
                }
            }
        }
        Some(_) => statement(parser),
        None => parser.unexpected_token_error(),
    }
}
