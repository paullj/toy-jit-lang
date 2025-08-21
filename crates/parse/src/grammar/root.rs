use syntax::SyntaxKind;

use crate::{Parser, marker::CompletedMarker};

use super::*;

pub(crate) fn root(parser: &mut Parser) -> CompletedMarker {
    let marker = parser.start();

    while !parser.is_at_end() {
        statement(parser);
    }

    marker.complete(parser, SyntaxKind::Root)
}
