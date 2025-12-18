use syntax::SyntaxKind;

use crate::{Parser, TokenSet, marker::CompletedMarker};
use lex::TokenKind;

use super::*;

const MAX_PARSE_ITERATIONS: usize = 10_000;

/// Recovery for root level - skip to newline or identifier
const ROOT_RECOVERY: TokenSet = TokenSet::new(&[TokenKind::NewLine, TokenKind::Identifier]);

pub(crate) fn root(p: &mut Parser) -> CompletedMarker {
    let m = p.start();
    let mut iterations = 0;

    while !p.is_at_end() {
        let pos_before = p.position();
        item(p);
        let pos_after = p.position();
        iterations += 1;

        // No progress - skip token to prevent infinite loop
        if pos_before == pos_after
            && p.recover("unexpected token", ROOT_RECOVERY).is_none()
            && !p.is_at_end()
        {
            // Still stuck, force consume
            p.consume();
        }

        if iterations > MAX_PARSE_ITERATIONS {
            let span = p.current_span();
            let found = p.current().map(|k| k.to_string());
            p.error(crate::ParseError::UnexpectedToken {
                at: span.into(),
                expected: "end of input".to_string(),
                found,
            });
            break;
        }
    }

    m.complete(p, SyntaxKind::Root)
}
