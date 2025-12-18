use syntax::SyntaxKind;

use crate::{Parser, marker::CompletedMarker};

use super::*;

const MAX_PARSE_ITERATIONS: usize = 10_000;

pub(crate) fn root(parser: &mut Parser) -> CompletedMarker {
    let marker = parser.start();

    let mut iterations = 0;

    while !parser.is_at_end() {
        let position_before = parser.position();
        item(parser);
        let position_after = parser.position();
        iterations += 1;

        // Check if we made progress - if position didn't change, we're stuck
        if position_before == position_after {
            // No progress made, create error and advance one token to prevent infinite loop
            if let Some(_error_marker) = parser.unexpected_token_error() {
                // Error marker created and token consumed, continue
            } else {
                // No error recovery happened, we need to break to avoid infinite loop
                break;
            }
        }

        if iterations > MAX_PARSE_ITERATIONS {
            parser.error_with_callback(|ctx| crate::ParseError::UnexpectedToken {
                at: ctx.at.clone().into(),
                expected: "end of input or valid item".to_string(),
                found: ctx.found_string(),
            });
            break;
        }
    }

    marker.complete(parser, SyntaxKind::Root)
}
