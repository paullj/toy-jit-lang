use std::mem;

use lex::Token;
use rowan::{GreenNode, GreenNodeBuilder};

use crate::{ParseError, event::Event};
use syntax::SyntaxKind;

/// Builds a concrete syntax tree from parsing events
pub struct Sink {
    builder: GreenNodeBuilder<'static>,
    errors: Vec<ParseError>,
}

impl Sink {
    /// Creates a new sink for building a syntax tree
    pub fn new() -> Self {
        Self {
            builder: GreenNodeBuilder::new(),
            errors: Vec::new(),
        }
    }

    /// Processes events to build the final syntax tree
    pub fn build(mut self, mut events: Vec<Event>) -> (GreenNode, Vec<ParseError>) {
        for i in 0..events.len() {
            match mem::replace(&mut events[i], Event::Placeholder) {
                Event::StartNode { kind, at } => {
                    let mut kinds = vec![kind];

                    let mut j = i;
                    let mut at = at;

                    // Walk through the forward parent of the forward parent and the forward parent
                    // of that, and of that, etc. until we reach a StartNode event without a forward
                    // parent.
                    while let Some(fp) = at {
                        j += fp;

                        at = if let Event::StartNode { kind, at } =
                            mem::replace(&mut events[j], Event::Placeholder)
                        {
                            kinds.push(kind);
                            at
                        } else {
                            unreachable!("Unexpected event type")
                        };
                    }

                    for kind in kinds.into_iter().rev() {
                        self.builder.start_node(kind.into());
                    }
                }
                Event::AddToken { token } => self.add_token(token),
                Event::FinishNode => self.builder.finish_node(),
                Event::Placeholder => {}
                Event::Error(parse_error) => self.errors.push(parse_error),
            }
        }
        (self.builder.finish(), self.errors)
    }

    fn add_token(&mut self, token: Token) {
        let kind: SyntaxKind = token.kind.into();
        // TODO: Figure out whether to pass string in each token or to slice here
        self.builder.token(kind.into(), token.text);
    }
}
