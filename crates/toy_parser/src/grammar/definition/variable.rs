use toy_cst::SyntaxKind;
use toy_lexer::TokenKind;

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

pub(crate) fn type_annotation(p: &mut Parser) -> CompletedMarker {
    let m = p.start();
    p.expect(TokenKind::Identifier, "type name");

    // Check for generic: list[Type]
    if p.at(TokenKind::LeftBracket) {
        p.consume(); // eat '['
        type_annotation(p); // recursive for nested: list[list[int]]
        if !p.eat(TokenKind::RightBracket) {
            let span = p.current_span();
            let found = p.current().map(|k| k.to_string());
            p.error(crate::ParseError::UnexpectedToken {
                at: span.into(),
                expected: "']'".to_string(),
                found,
            });
        }
        return m.complete(p, SyntaxKind::ListType);
    }

    m.complete(p, SyntaxKind::TypeAnnotation)
}
