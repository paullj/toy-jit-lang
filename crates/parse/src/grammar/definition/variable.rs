use lex::TokenKind;
use syntax::SyntaxKind;

use crate::{
    Parser,
    grammar::expression,
    marker::{CompletedMarker, Marker},
};

/// Parses `x := expr` (type inference)
/// NOTE: Identifier and ':' have already been consumed
pub(crate) fn variable_definition_inferred(p: &mut Parser, m: Marker) -> CompletedMarker {
    debug_assert!(p.at(TokenKind::Equals));
    p.consume();
    expression(p);
    m.complete(p, SyntaxKind::VariableDefinition)
}

/// Parses `x: Type = expr` (explicit type)
/// NOTE: Identifier and ':' have already been consumed
pub(crate) fn variable_definition_typed(p: &mut Parser, m: Marker) -> CompletedMarker {
    type_annotation(p);
    p.expect(TokenKind::Equals, "'='");
    expression(p);
    m.complete(p, SyntaxKind::VariableDefinition)
}

fn type_annotation(p: &mut Parser) -> CompletedMarker {
    let m = p.start();
    p.expect(TokenKind::Identifier, "type name");
    m.complete(p, SyntaxKind::TypeAnnotation)
}
