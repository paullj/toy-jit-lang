use lex::TokenKind;
use syntax::SyntaxKind;

use crate::{
    Parser,
    grammar::statement::expression,
    marker::{CompletedMarker, Marker},
};

pub(crate) fn variable_assignment(parser: &mut Parser, marker: Marker) -> CompletedMarker {
    assert!(parser.is_at(TokenKind::Equals));

    parser.consume();
    expression(parser);
    marker.complete(parser, SyntaxKind::VariableAssignment)
}
