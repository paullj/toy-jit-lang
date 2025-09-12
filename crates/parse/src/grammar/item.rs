use lex::TokenKind;
use syntax::SyntaxKind;

use super::*;
use crate::{ParseError, Parser, concat_kinds, marker::CompletedMarker};

pub(crate) const ITEM_KINDS: &[TokenKind] = concat_kinds!(DEFINITION_KINDS, EXPRESSION_LHS_KINDS);
pub(crate) const ITEM_VARIABLE_KINDS: &[TokenKind] = concat_kinds!(
    &[TokenKind::ColonEquals, TokenKind::Equals],
    EXPRESSION_KINDS
);

pub(crate) fn item(parser: &mut Parser) -> Option<CompletedMarker> {
    match parser.is_at_one_of(&ITEM_KINDS) {
        Some(TokenKind::Function) => Some(function_definition(parser)),
        // NOTE: Special case for handling identifiers since we do not have a dedicated keyword for defining variables
        Some(TokenKind::Identifier) => {
            let marker = parser.start();
            parser.consume();

            match parser.is_at_one_of(&ITEM_VARIABLE_KINDS) {
                Some(TokenKind::ColonEquals) => Some(variable_definition(parser, marker)),
                Some(TokenKind::Equals) => Some(variable_assignment(parser, marker)),
                Some(_) => {
                    let lhs = marker.complete(parser, SyntaxKind::VariableReference);
                    inner_expression_with_binding_power(parser, lhs, 0)
                }
                None => {
                    marker.discard(parser);
                    parser.error_with_callback(|_| ParseError::UnexpectedToken)
                }
            }
        }
        Some(_) => statement(parser),
        None => parser.error_with_callback(|_| ParseError::UnexpectedToken),
    }
}
