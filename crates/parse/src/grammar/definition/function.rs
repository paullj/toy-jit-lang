use lex::TokenKind;
use syntax::SyntaxKind;

use crate::{
    ParseError, Parser,
    grammar::{item, statement},
    marker::CompletedMarker,
};

pub(crate) fn function_definition(parser: &mut Parser) -> CompletedMarker {
    assert!(parser.is_at(TokenKind::Function));

    let marker = parser.start();
    parser.consume();

    parser.expect(TokenKind::Identifier, |_| ParseError::UnexpectedToken);
    if parser.is_at(TokenKind::LeftParenthesis) {
        parameter_list(parser);
    }

    if parser.is_at(TokenKind::LeftBrace) {
        block(parser);
    }

    marker.complete(parser, SyntaxKind::FunctionDeclaration)
}

fn parameter_list(parser: &mut Parser) -> CompletedMarker {
    assert!(parser.is_at(TokenKind::LeftParenthesis));
    parser.consume();

    let marker = parser.start();

    while !parser.is_at(TokenKind::RightParenthesis) {
        parameter(parser);

        if parser.is_at(TokenKind::Comma) {
            parser.consume();
        }
    }

    let completed_marker = marker.complete(parser, SyntaxKind::ParameterList);
    parser.consume();
    completed_marker
}

fn parameter(parser: &mut Parser) -> CompletedMarker {
    let marker = parser.start();

    assert!(parser.is_at(TokenKind::Identifier));
    parser.consume();

    marker.complete(parser, SyntaxKind::Parameter)
}

fn block(parser: &mut Parser) -> CompletedMarker {
    let marker = parser.start();

    assert!(parser.is_at(TokenKind::LeftBrace));
    parser.consume();
    parser.consume_trivia();

    while !parser.is_at(TokenKind::RightBrace) {
        item(parser);
    }

    parser.consume();

    marker.complete(parser, SyntaxKind::Block)
}
