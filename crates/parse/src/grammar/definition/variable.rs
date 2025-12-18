use lex::TokenKind;
use syntax::SyntaxKind;

use crate::{
    ParseError, Parser,
    grammar::expression,
    marker::{CompletedMarker, Marker},
};

/// Parses `x := expr` (type inference)
/// Note: Identifier and ':' have already been consumed
pub(crate) fn variable_definition_inferred(parser: &mut Parser, marker: Marker) -> CompletedMarker {
    // The ':' was consumed in item.rs, now consume '='
    assert!(parser.is_at(TokenKind::Equals));
    parser.consume();

    expression(parser);
    marker.complete(parser, SyntaxKind::VariableDefinition)
}

/// Parses `x: Type = expr` (explicit type)
/// Note: Identifier and ':' have already been consumed
pub(crate) fn variable_definition_typed(parser: &mut Parser, marker: Marker) -> CompletedMarker {
    // The ':' was consumed in item.rs
    // Now expect the type identifier
    type_annotation(parser);

    // Expect '=' and the expression
    parser.expect(TokenKind::Equals, |ctx| ParseError::UnexpectedToken {
        at: ctx.at.clone().into(),
        expected: "'=' after type annotation".to_string(),
        found: ctx.found_string(),
    });

    expression(parser);
    marker.complete(parser, SyntaxKind::VariableDefinition)
}

fn type_annotation(parser: &mut Parser) -> CompletedMarker {
    let marker = parser.start();

    parser.expect(TokenKind::Identifier, |ctx| ParseError::UnexpectedToken {
        at: ctx.at.clone().into(),
        expected: "type name".to_string(),
        found: ctx.found_string(),
    });

    marker.complete(parser, SyntaxKind::TypeAnnotation)
}
