use lex::TokenKind;
use syntax::SyntaxKind;

use crate::{
    Parser,
    grammar::expression,
    marker::{CompletedMarker, Marker},
};

pub(crate) fn variable_definition(parser: &mut Parser, marker: Marker) -> CompletedMarker {
    // Assert and consume token since we already know from the layer above that we are at `:=`
    // TODO: Can this be a debug_assert?
    assert!(parser.is_at(TokenKind::ColonEquals));

    parser.consume();
    expression(parser);
    marker.complete(parser, SyntaxKind::VariableDefinition)
}
