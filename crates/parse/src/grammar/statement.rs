use lex::TokenKind;
use syntax::SyntaxKind;

use crate::{
    Parser,
    marker::{CompletedMarker, Marker},
};

pub(crate) fn statement(parser: &mut Parser) -> Option<CompletedMarker> {
    if parser.is_at(TokenKind::Identifier) {
        Some(variable_definition(parser))
    } else {
        None
    }
}

fn variable_definition(parser: &mut Parser) -> CompletedMarker {
    // TODO: Should this be an assertion because we already check for this in the calling fn?
    if !parser.is_at(TokenKind::Identifier) {
        todo!("Handle unexpectedly not at identifier");
    }

    let marker = parser.start();
    parser.consume();

    if parser.is_at(TokenKind::ColonEquals) {
        variable_defintion(marker, parser)
    } else {
        todo!("variable assignement is not supported yet");
        // parser.expect(TokenKind::Equals);
        // expression()
    }
}

fn variable_defintion(marker: Marker, parser: &mut Parser) -> CompletedMarker {
    // TODO: Should this be an assertion because we already check for this in the calling fn? how to handle errors?
    parser.expect(TokenKind::ColonEquals);
    // TODO: Replace this with expression parsing
    parser.expect(TokenKind::Integer);

    marker.complete(parser, SyntaxKind::VariableDefinition)
}
