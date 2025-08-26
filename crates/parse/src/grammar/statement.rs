use lex::TokenKind;
use syntax::SyntaxKind;

use crate::{
    ParseError, Parser,
    marker::{CompletedMarker, Marker},
};

use super::*;

pub(crate) fn statement(parser: &mut Parser) -> Option<CompletedMarker> {
    if parser.is_at(TokenKind::Identifier) {
        variable_statement(parser)
    } else {
        None
    }
}

fn variable_statement(parser: &mut Parser) -> Option<CompletedMarker> {
    // TODO: Should this be an assertion because we already check for this in the calling fn?
    if !parser.is_at(TokenKind::Identifier) {
        todo!("Handle unexpectedly not at identifier");
    }

    let marker = parser.start();
    parser.consume();

    if parser.is_at(TokenKind::ColonEquals) {
        Some(variable_definition(marker, parser))
    } else if parser.is_at(TokenKind::Equals) {
        Some(variable_assignment(marker, parser))
    } else {
        marker.discard(parser);
        parser.error(ParseError::UnexpectedToken);
        None
    }
}

fn variable_definition(marker: Marker, parser: &mut Parser) -> CompletedMarker {
    // Assert and consume token since we already know from the layer above that we are at `:=`
    // TODO: Can this be a debug_assert?
    assert!(parser.is_at(TokenKind::ColonEquals));

    parser.consume();
    expression(parser);
    marker.complete(parser, SyntaxKind::VariableDefinition)
}

fn variable_assignment(marker: Marker, parser: &mut Parser) -> CompletedMarker {
    // Assert and consume token since we already know from the layer above that we are at `:=`
    // TODO: Can this be a debug_assert?
    assert!(parser.is_at(TokenKind::Equals));

    parser.consume();
    expression(parser);
    marker.complete(parser, SyntaxKind::VariableAssignment)
}
