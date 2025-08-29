use lex::Token;
use rowan::{GreenNode, GreenNodeBuilder};

use crate::{ParseError, error::Issue, event::Event};
use syntax::SyntaxKind;

/// Builds a concrete syntax tree from parsing events
pub struct Sink {
    builder: GreenNodeBuilder<'static>,
    errors: Vec<Issue>,
}

// TODO: Change this api to be a bit nicer
impl Sink {
    /// Creates a new sink for building a syntax tree
    pub fn new() -> Self {
        Self {
            builder: GreenNodeBuilder::new(),
            errors: Vec::new(),
        }
    }

    /// Processes events to build the final syntax tree
    pub fn build(mut self, events: Vec<Event>) -> (GreenNode, Vec<Issue>) {
        for event in events {
            match event {
                Event::StartNode { kind, at: _ } => {
                    self.builder.start_node(kind.into());
                }
                Event::AddToken { token } => {
                    self.add_token(token);
                }
                Event::FinishNode => {
                    self.builder.finish_node();
                }
                Event::Placeholder => todo!("placeholder needs dealing with"),
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
