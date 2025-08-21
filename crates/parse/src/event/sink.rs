use lex::Token;
use rowan::{GreenNode, GreenNodeBuilder};

use crate::event::Event;
use syntax::SyntaxKind;

pub struct Sink {
    builder: GreenNodeBuilder<'static>,
}

// TODO: Change this api to be a bit nicer
impl Sink {
    pub fn new() -> Self {
        Self {
            builder: GreenNodeBuilder::new(),
        }
    }

    pub fn build(mut self, events: Vec<Event>) -> GreenNode {
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
                Event::Placeholder => todo!(),
            }
        }
        self.builder.finish()
    }

    fn add_token(&mut self, token: Token) {
        let kind: SyntaxKind = token.kind.into();
        // TODO: Figure out whether to pass string in each token or to slice here
        self.builder.token(kind.into(), token.text);
    }
}
