use toy_cst::SyntaxKind;
use toy_lexer::TokenKind;

use crate::{Parser, marker::CompletedMarker};

/// Parses a use statement
/// Examples:
/// - `use std.io`
/// - `use pkg.http`
/// - `use .helpers`
/// - `use src.helpers`
/// - `use std.math.{sin, cos, PI}`
/// - `use std.math as m`
/// - `pub use std.math.PI`
pub(crate) fn use_statement(p: &mut Parser, is_pub: bool) -> CompletedMarker {
    let m = p.start();

    // If this is a pub use, we already consumed the pub token
    if is_pub {
        // pub token already consumed by caller
    }

    debug_assert!(p.at(TokenKind::Use));
    p.consume(); // eat 'use'

    // Parse the module path
    module_path(p);

    // Check for selective imports: {sin, cos, PI}
    if p.at(TokenKind::LeftBrace) {
        import_list(p);
    }

    // Check for alias: as m
    if p.at(TokenKind::As) {
        import_alias(p);
    }

    m.complete(p, SyntaxKind::UseStatement)
}

/// Parses a module path
/// Examples:
/// - `std.io`
/// - `pkg.http`
/// - `.helpers`
/// - `src.helpers`
fn module_path(p: &mut Parser) -> CompletedMarker {
    let m = p.start();

    // Handle different path types
    if p.at(TokenKind::Dot) {
        // Relative path: .helpers
        p.consume(); // eat '.'
    } else if !p.at(TokenKind::Identifier) {
        // Error: expected identifier or '.'
        let span = p.current_span();
        let found = p.current().map(|k| k.to_string());
        p.error(crate::ParseError::UnexpectedToken {
            at: span.into(),
            expected: "module path".to_string(),
            found,
        });
        return m.complete(p, SyntaxKind::ModulePath);
    }

    // Parse the first identifier (if not a relative path)
    if p.at(TokenKind::Identifier) {
        p.consume();
    }

    // Parse remaining path segments
    while p.at(TokenKind::Dot) {
        p.consume(); // eat '.'
        // Check if there's an identifier after the dot
        if p.at(TokenKind::Identifier) {
            p.consume(); // eat identifier
        } else if !p.at(TokenKind::LeftBrace) {
            // Error if dot is not followed by identifier or '{'
            let span = p.current_span();
            let found = p.current().map(|k| k.to_string());
            p.error(crate::ParseError::UnexpectedToken {
                at: span.into(),
                expected: "identifier or '{' after '.'".to_string(),
                found,
            });
            break;
        } else {
            // Dot followed by '{' is valid for selective imports
            break;
        }
    }

    m.complete(p, SyntaxKind::ModulePath)
}

/// Parses an import list
/// Example: {sin, cos, PI}
fn import_list(p: &mut Parser) -> CompletedMarker {
    let m = p.start();

    debug_assert!(p.at(TokenKind::LeftBrace));
    p.consume(); // eat '{'
    p.enter_delimiter();

    let mut first = true;
    while !p.at(TokenKind::RightBrace) && !p.is_at_end() {
        if !first && !p.eat(TokenKind::Comma) {
            break;
        }
        first = false;

        // Handle trailing comma
        if p.at(TokenKind::RightBrace) {
            break;
        }

        import_item(p);
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

    m.complete(p, SyntaxKind::ImportList)
}

/// Parses a single import item
fn import_item(p: &mut Parser) -> CompletedMarker {
    let m = p.start();

    if !p.eat(TokenKind::Identifier) {
        let span = p.current_span();
        let found = p.current().map(|k| k.to_string());
        p.error(crate::ParseError::UnexpectedToken {
            at: span.into(),
            expected: "identifier".to_string(),
            found,
        });
    }

    // Check for alias within import item: sin as sine
    if p.at(TokenKind::As) {
        p.consume(); // eat 'as'
        if !p.eat(TokenKind::Identifier) {
            let span = p.current_span();
            let found = p.current().map(|k| k.to_string());
            p.error(crate::ParseError::UnexpectedToken {
                at: span.into(),
                expected: "alias name".to_string(),
                found,
            });
        }
    }

    m.complete(p, SyntaxKind::ImportItem)
}

/// Parses an import alias
/// Example: as m
fn import_alias(p: &mut Parser) -> CompletedMarker {
    let m = p.start();

    debug_assert!(p.at(TokenKind::As));
    p.consume(); // eat 'as'

    if !p.eat(TokenKind::Identifier) {
        let span = p.current_span();
        let found = p.current().map(|k| k.to_string());
        p.error(crate::ParseError::UnexpectedToken {
            at: span.into(),
            expected: "alias name".to_string(),
            found,
        });
    }

    m.complete(p, SyntaxKind::ImportAlias)
}
