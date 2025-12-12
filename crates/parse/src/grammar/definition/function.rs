use lex::TokenKind;
use syntax::SyntaxKind;

use crate::{ParseError, Parser, grammar::item, marker::CompletedMarker};

pub(crate) fn function_definition(parser: &mut Parser) -> CompletedMarker {
    assert!(parser.is_at(TokenKind::Function));

    let marker = parser.start();
    parser.consume();

    parser.expect(TokenKind::Identifier, |ctx| ParseError::UnexpectedToken {
        at: ctx.at.clone().into(),
        expected: "function name".to_string(),
        found: ctx.found_string(),
    });
    if parser.is_at(TokenKind::LeftParenthesis) {
        parameter_list(parser);
    }

    // Optional return type annotation
    if parser.is_at(TokenKind::RightArrow) {
        return_type_annotation(parser);
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

    const MAX_PARAMETERS: usize = 1000;
    let mut param_count = 0;
    
    while !parser.is_at(TokenKind::RightParenthesis) && param_count < MAX_PARAMETERS {
        if !parser.is_at(TokenKind::Identifier) {
            // If we're not at an identifier, we can't parse a parameter
            // Break to avoid infinite loop
            break;
        }
        
        parameter(parser);
        param_count += 1;

        if parser.is_at(TokenKind::Comma) {
            parser.consume();
        } else if !parser.is_at(TokenKind::RightParenthesis) {
            // Expected comma or closing paren, break to avoid infinite loop
            break;
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

    // Require type annotation with colon
    parser.expect(TokenKind::Colon, |ctx| ParseError::MissingParameterType {
        at: ctx.at.into(),
        help: "Add type annotation like ': int' after parameter name".to_string(),
    });
    
    type_annotation(parser);

    marker.complete(parser, SyntaxKind::Parameter)
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

fn return_type_annotation(parser: &mut Parser) -> CompletedMarker {
    let marker = parser.start();
    
    assert!(parser.is_at(TokenKind::RightArrow));
    parser.consume(); // consume the '->'
    
    type_annotation(parser);
    
    marker.complete(parser, SyntaxKind::ReturnTypeAnnotation)
}

fn block(parser: &mut Parser) -> CompletedMarker {
    let marker = parser.start();

    assert!(parser.is_at(TokenKind::LeftBrace));
    parser.consume();
    parser.consume_trivia();

    const MAX_STATEMENTS: usize = 10000;
    let mut statement_count = 0;
    
    while !parser.is_at(TokenKind::RightBrace) && statement_count < MAX_STATEMENTS {
        if parser.is_at_end() {
            // Reached end of input without closing brace
            break;
        }
        
        let parsed_item = item(parser);
        statement_count += 1;
        
        // If item parsing failed and we didn't advance, break to avoid infinite loop
        if parsed_item.is_none() && !parser.is_at(TokenKind::RightBrace) {
            // Try to recover by consuming one token
            if !parser.is_at_end() {
                parser.consume();
            } else {
                break;
            }
        }
    }

    parser.consume();

    marker.complete(parser, SyntaxKind::Block)
}
