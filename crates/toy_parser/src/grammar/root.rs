use toy_cst::SyntaxKind;

use crate::{Parser, marker::CompletedMarker};

use super::*;

pub(crate) fn root(p: &mut Parser) -> CompletedMarker {
    let m = p.start();

    while !p.is_at_end() {
        let pos_before = p.position();
        item(p);

        // NOTE: prevent infinite loop if item() makes no progress.
        // Currently unreachable - item() either handles tokens or recover() skips them.
        if p.position() == pos_before && !p.is_at_end() {
            eprintln!(
                "[parse] root(): no progress made, skipping token {:?}",
                p.current()
            );
            p.consume();
        }
    }

    // Capture any trailing trivia (newlines, comments, whitespace) at EOF
    p.consume_trailing_trivia();

    m.complete(p, SyntaxKind::Root)
}
