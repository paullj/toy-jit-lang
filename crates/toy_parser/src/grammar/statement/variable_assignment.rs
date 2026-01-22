use toy_cst::SyntaxKind;
use toy_lexer::TokenKind;

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
