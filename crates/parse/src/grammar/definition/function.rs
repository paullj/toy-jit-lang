use lex::TokenKind;
use syntax::SyntaxKind;

use super::type_annotation;
use crate::{Parser, marker::CompletedMarker};

/// Parses a function definition or lambda expression.
/// Called when we see `fn`.
pub(crate) fn function_definition_or_expression(p: &mut Parser) -> CompletedMarker {
    debug_assert!(p.at(TokenKind::Fn));
    let m = p.start();
    p.consume(); // eat 'fn'

    if p.at(TokenKind::Identifier) {
        // Named function: fn name(...)
        p.consume(); // eat name
        parameter_list(p);
        optional_return_type(p);
        block_expression(p);
        m.complete(p, SyntaxKind::FunctionDefinition)
    } else {
        // Lambda: fn(...)
        parameter_list(p);
        optional_return_type(p);
        block_expression(p);
        m.complete(p, SyntaxKind::FunctionExpression)
    }
}

fn parameter_list(p: &mut Parser) -> CompletedMarker {
    let m = p.start();
    p.expect(TokenKind::LeftParenthesis, "'('");
    p.enter_delimiter();

    let mut first = true;
    while !p.at(TokenKind::RightParenthesis) && !p.is_at_end() {
        if !first && !p.eat(TokenKind::Comma) {
            break;
        }
        first = false;

        // Handle trailing comma
        if p.at(TokenKind::RightParenthesis) {
            break;
        }

        parameter(p);
    }

    p.exit_delimiter();
    p.expect(TokenKind::RightParenthesis, "')'");
    m.complete(p, SyntaxKind::ParameterList)
}

fn parameter(p: &mut Parser) -> CompletedMarker {
    let m = p.start();
    p.expect(TokenKind::Identifier, "parameter name");

    // Optional type: `: int`
    if p.eat(TokenKind::Colon) {
        type_annotation(p);
    }

    // Optional default: `= expr`
    if p.eat(TokenKind::Equals) {
        crate::grammar::expression(p);
    }

    m.complete(p, SyntaxKind::Parameter)
}

fn optional_return_type(p: &mut Parser) {
    // Return type syntax: `: Type` after params
    if p.eat(TokenKind::Colon) {
        type_annotation(p);
    }
}

fn block_expression(p: &mut Parser) {
    if !p.at(TokenKind::LeftBrace) {
        let span = p.current_span();
        let found = p.current().map(|k| k.to_string());
        p.error(crate::ParseError::UnexpectedToken {
            at: span.into(),
            expected: "'{'".to_string(),
            found,
        });
        return;
    }
    crate::grammar::statement::expression::block_expression(p);
}
