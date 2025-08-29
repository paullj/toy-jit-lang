use std::mem;

use lex::{Token, TokenKind};
use syntax::SyntaxKind;

use crate::error::{Issue, ParseError};
use crate::event::{Event, Source};

use crate::grammar;
use crate::marker::Marker;

const RECOVERABLE_KINDS: [TokenKind; 1] = [TokenKind::ColonEquals];

/// Event-driven parser that converts tokens into parsing events
pub struct Parser<'a> {
    source: Source<'a>,
    pub(crate) events: Vec<Event<'a>>,
    current_token: Option<Token<'a>>,
    expected_kinds: Vec<TokenKind>,
}

impl<'a> Parser<'a> {
    /// Creates a new parser from a token source
    pub fn new(source: Source<'a>) -> Self {
        Self {
            source,
            current_token: None,
            events: Vec::new(),
            expected_kinds: Vec::new(),
        }
    }

    /// Parses the input and returns events and errors
    pub fn parse(mut self) -> Vec<Event<'a>> {
        grammar::root(&mut self);
        self.events
    }

    /// Creates a marker for the start of a syntax node
    pub(crate) fn start(&mut self) -> Marker {
        let pos = self.events.len();
        self.events.push(Event::Placeholder);

        Marker::new(pos)
    }

    /// Expects a specific token kind, consuming it or erroring
    pub(crate) fn expect(&mut self, kind: TokenKind, or: ParseError) {
        if self.is_at(kind) {
            self.consume();
        } else {
            self.error(or);
        }
    }

    /// Consumes the next token and its trivia
    pub(crate) fn consume(&mut self) {
        self.expected_kinds.clear();
        let next = self.source.next();
        for trivia in next.trivia {
            self.events.push(Event::AddToken { token: trivia });
        }

        if let Some(token) = next.token {
            self.events.push(Event::AddToken { token });
        }
    }

    pub(crate) fn error(&mut self, error: ParseError) {
        let found = match self.source.peek() {
            Some(token) => token,
            None => todo!("Handle end of input"),
        };
        let expected = mem::take(&mut self.expected_kinds);

        self.events.push(Event::Error(Issue {
            expected,
            found: Some(found.kind),
            span: found.span.clone(),
            kind: error,
        }));
        if !self.is_at_one_of(&RECOVERABLE_KINDS) && !self.is_at_end() {
            let marker = self.start();
            self.consume();
            marker.complete(self, SyntaxKind::Error);
        }
    }

    /// Checks if the current token matches the given kind
    pub(crate) fn is_at(&mut self, kind: TokenKind) -> bool {
        self.expected_kinds.push(kind);
        if let Some(token) = self.source.peek() {
            token.kind == kind
        } else {
            false
        }
    }

    /// Checks if the current token matches one of the given kinds
    pub(crate) fn is_at_one_of(&mut self, set: &[TokenKind]) -> bool {
        self.source.peek().map_or(false, |t| set.contains(&t.kind))
    }

    /// Checks if at end of input
    pub(crate) fn is_at_end(&mut self) -> bool {
        self.source.peek().is_none()
    }
}
