use lex::TokenKind;
use syntax::SyntaxKind;

use crate::{Parser, marker::CompletedMarker};

/// Parses struct definition: `struct Name { field: Type, ... }`
pub(crate) fn struct_definition(p: &mut Parser) -> CompletedMarker {
    debug_assert!(p.at(TokenKind::Struct));
    let m = p.start();
    p.consume(); // eat 'struct'

    // Struct name
    if !p.eat(TokenKind::Identifier) {
        let span = p.current_span();
        let found = p.current().map(|k| k.to_string());
        p.error(crate::ParseError::UnexpectedToken {
            at: span.into(),
            expected: "struct name".to_string(),
            found,
        });
    }

    // Expect '{'
    if !p.eat(TokenKind::LeftBrace) {
        let span = p.current_span();
        let found = p.current().map(|k| k.to_string());
        p.error(crate::ParseError::UnexpectedToken {
            at: span.into(),
            expected: "'{'".to_string(),
            found,
        });
    }

    p.enter_delimiter();

    // Parse fields: name: Type, ...
    while !p.at(TokenKind::RightBrace) && !p.is_at_end() {
        // Skip newlines between fields
        while p.eat(TokenKind::NewLine) {}

        if p.at(TokenKind::RightBrace) {
            break;
        }

        struct_field_def(p);

        // Allow comma or newline as separator
        if !p.at(TokenKind::RightBrace) && !p.eat(TokenKind::Comma) && !p.eat(TokenKind::NewLine) {
            // If neither comma nor newline, might still be at end
            if !p.at(TokenKind::RightBrace) {
                let span = p.current_span();
                let found = p.current().map(|k| k.to_string());
                p.error(crate::ParseError::UnexpectedToken {
                    at: span.into(),
                    expected: "',' or newline".to_string(),
                    found,
                });
                break;
            }
        }
    }

    p.exit_delimiter();

    if !p.eat(TokenKind::RightBrace) {
        let span = p.current_span();
        let found = p.current().map(|k| k.to_string());
        p.error(crate::ParseError::UnexpectedToken {
            at: span.into(),
            expected: "'}'".to_string(),
            found,
        });
    }

    m.complete(p, SyntaxKind::StructDefinition)
}

/// Parses a single field definition: `name: Type`
fn struct_field_def(p: &mut Parser) -> CompletedMarker {
    let m = p.start();

    // Field name
    if !p.eat(TokenKind::Identifier) {
        let span = p.current_span();
        let found = p.current().map(|k| k.to_string());
        p.error(crate::ParseError::UnexpectedToken {
            at: span.into(),
            expected: "field name".to_string(),
            found,
        });
    }

    // Expect ':'
    if !p.eat(TokenKind::Colon) {
        let span = p.current_span();
        let found = p.current().map(|k| k.to_string());
        p.error(crate::ParseError::UnexpectedToken {
            at: span.into(),
            expected: "':'".to_string(),
            found,
        });
    }

    // Type annotation (identifier for now)
    if !p.eat(TokenKind::Identifier) {
        let span = p.current_span();
        let found = p.current().map(|k| k.to_string());
        p.error(crate::ParseError::UnexpectedToken {
            at: span.into(),
            expected: "type".to_string(),
            found,
        });
    }

    m.complete(p, SyntaxKind::StructFieldDef)
}
