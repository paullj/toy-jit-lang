use drop_bomb::DropBomb;
use syntax::SyntaxKind;

use crate::{Parser, event::Event};

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

    pub(crate) fn complete(mut self, p: &mut Parser, kind: SyntaxKind) -> CompletedMarker {
        self.bomb.defuse();

        let event_at_pos = &mut p.events[self.at];
        assert_eq!(*event_at_pos, Event::Placeholder);

        *event_at_pos = Event::StartNode { kind, at: None };

        p.events.push(Event::FinishNode);

        CompletedMarker { at: self.at }
    }
}

pub(crate) struct CompletedMarker {
    at: usize,
}

impl CompletedMarker {
    pub(crate) fn precede(self, p: &mut Parser) -> Marker {
        let new_m = p.start();

        if let Event::StartNode { ref mut at, .. } = p.events[self.at] {
            *at = Some(new_m.at - self.at);
        } else {
            unreachable!();
        }

        new_m
    }
}
