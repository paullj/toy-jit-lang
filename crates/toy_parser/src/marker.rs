use drop_bomb::DropBomb;
use toy_cst::SyntaxKind;

use crate::{Parser, event::Event};

/// Marks the start of a syntax node, must be completed to avoid panic
pub(crate) struct Marker {
    at: usize,
    bomb: DropBomb,
}

impl Marker {
    pub(crate) fn new(at: usize) -> Self {
        Self {
            at,
            bomb: DropBomb::new("Markers need to be completed"),
        }
    }

    /// Completes the marker by setting the node kind at the marked position
    pub(crate) fn complete(mut self, parser: &mut Parser, kind: SyntaxKind) -> CompletedMarker {
        self.bomb.defuse();

        let event_at_pos = &mut parser.events[self.at];
        assert_eq!(*event_at_pos, Event::Placeholder);

        *event_at_pos = Event::StartNode { kind, at: None };

        parser.events.push(Event::FinishNode);

        CompletedMarker { at: self.at }
    }
}

/// A marker that has been completed and can create preceding nodes
pub(crate) struct CompletedMarker {
    at: usize,
}

impl CompletedMarker {
    /// Creates a new marker that wraps this completed node
    pub(crate) fn precede(self, parser: &mut Parser) -> Marker {
        let new_marker = parser.start();

        // NOTE: Links the new marker to this completed one for precedence
        if let Event::StartNode { ref mut at, .. } = parser.events[self.at] {
            *at = Some(new_marker.at - self.at);
        } else {
            unreachable!();
        }

        new_marker
    }
}
