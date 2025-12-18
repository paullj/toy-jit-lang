use lex::TokenKind;
use syntax::SyntaxKind;

use crate::{
    Parser,
    grammar::statement::expression,
    marker::{CompletedMarker, Marker},
};

pub(crate) fn variable_assignment(p: &mut Parser, m: Marker) -> CompletedMarker {
    debug_assert!(p.at(TokenKind::Equals));
    p.consume();
    expression(p);
    m.complete(p, SyntaxKind::VariableAssignment)
}
