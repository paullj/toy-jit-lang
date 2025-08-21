use lex::{Token, TokenKind};

use crate::error::ParseError;
use crate::event::{Event, Source};

use crate::grammar;
use crate::marker::Marker;

/// Event-driven parser that converts tokens into parsing events
pub struct Parser<'a> {
    source: Source<'a>,
    pub(crate) events: Vec<Event<'a>>,
    current_token: Option<Token<'a>>,
    errors: Vec<ParseError>,
}

impl<'a> Parser<'a> {
    /// Creates a new parser from a token source
    pub fn new(source: Source<'a>) -> Self {
        Self {
            source,
            events: Vec::new(),
            current_token: None,
            errors: Vec::new(),
        }
    }

    /// Parses the input and returns events and errors
    pub fn parse(mut self) -> (Vec<Event<'a>>, Vec<ParseError>) {
        grammar::root(&mut self);
        (self.events, self.errors)
    }

    /// Creates a marker for the start of a syntax node
    pub(crate) fn start(&mut self) -> Marker {
        let pos = self.events.len();
        self.events.push(Event::Placeholder);

        Marker::new(pos)
    }

    /// Expects a specific token kind, consuming it or erroring
    pub(crate) fn expect(&mut self, kind: TokenKind) {
        if self.is_at(kind) {
            self.consume();
        } else {
            todo!("Handle errors")
        }
    }

    /// Consumes the next token and its trivia
    pub(crate) fn consume(&mut self) {
        let next = self.source.next();
        for trivia in next.trivia {
            self.events.push(Event::AddToken { token: trivia });
        }

        if let Some(token) = next.token {
            self.events.push(Event::AddToken { token });
        }
    }

    /// Checks if the current token matches the given kind
    pub(crate) fn is_at(&mut self, kind: TokenKind) -> bool {
        if let Some(token) = self.source.peek() {
            token.kind == kind
        } else {
            false
        }
    }

    /// Checks if at end of input
    pub(crate) fn is_at_end(&mut self) -> bool {
        self.source.peek().is_none()
    }
}
