use std::ops::Range;

use lex::TokenKind;
use syntax::SyntaxKind;

use crate::error::ParseError;
use crate::event::{Event, Source};
use crate::token_set::TokenSet;

use crate::grammar;
use crate::marker::{CompletedMarker, Marker};

/// Event-driven parser that converts tokens into parsing events
pub struct Parser<'a> {
    source: Source<'a>,
    pub(crate) events: Vec<Event<'a>>,
}

impl<'a> Parser<'a> {
    /// Creates a new parser from a token source
    pub fn new(source: Source<'a>) -> Self {
        Self {
            source,
            events: Vec::new(),
        }
    }

    /// Parses the input and returns events and errors
    pub fn parse(mut self) -> (Vec<Event<'a>>, Vec<lex::Error>) {
        grammar::root(&mut self);
        let lexer_errors = self.source.take_pending_errors();
        (self.events, lexer_errors)
    }

    /// Creates a marker for the start of a syntax node
    pub(crate) fn start(&mut self) -> Marker {
        let pos = self.events.len();
        self.events.push(Event::Placeholder);
        Marker::new(pos)
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

    /// Checks if at end of input
    pub(crate) fn is_at_end(&mut self) -> bool {
        self.source.peek().is_none()
    }

    /// Returns the current parser position for progress tracking
    pub(crate) fn position(&mut self) -> usize {
        self.source
            .peek()
            .map(|t| t.span.start)
            .unwrap_or_else(|| self.source.last_span().start)
    }

    /// Check if current token matches kind
    pub(crate) fn at(&mut self, kind: TokenKind) -> bool {
        self.source.peek().is_some_and(|t| t.kind == kind)
    }

    /// Check if current token is in the set
    pub(crate) fn at_set(&mut self, set: TokenSet) -> bool {
        self.source.peek().is_some_and(|t| set.contains(t.kind))
    }

    /// Returns current token kind without consuming
    pub(crate) fn current(&mut self) -> Option<TokenKind> {
        self.source.peek().map(|t| t.kind)
    }

    /// Returns current span
    pub(crate) fn current_span(&mut self) -> Range<usize> {
        self.source
            .peek()
            .map(|t| t.span.clone())
            .unwrap_or_else(|| self.source.last_span())
    }

    /// Consume token if it matches, returns whether consumed
    pub(crate) fn eat(&mut self, kind: TokenKind) -> bool {
        if self.at(kind) {
            self.consume();
            true
        } else {
            false
        }
    }

    /// Emit a parse error directly
    pub(crate) fn error(&mut self, error: ParseError) {
        self.events.push(Event::Error(error));
    }

    /// Consume if matches, else emit error with message
    pub(crate) fn expect(&mut self, kind: TokenKind, msg: &str) -> bool {
        if self.eat(kind) {
            true
        } else {
            let span = self.current_span();
            let found = self.current().map(|k| k.to_string());
            self.error(ParseError::UnexpectedToken {
                at: span.into(),
                expected: msg.to_string(),
                found,
            });
            false
        }
    }

    /// Recover by skipping tokens until recovery set, wrapping skipped in Error node
    pub(crate) fn recover(&mut self, msg: &str, recovery: TokenSet) -> Option<CompletedMarker> {
        // Don't skip if already at recovery point or end
        if self.at_set(recovery) || self.is_at_end() {
            let span = self.current_span();
            let found = self.current().map(|k| k.to_string());
            self.error(ParseError::UnexpectedToken {
                at: span.into(),
                expected: msg.to_string(),
                found,
            });
            return None;
        }

        // Wrap skipped tokens in single Error node
        let span = self.current_span();
        let found = self.current().map(|k| k.to_string());
        let marker = self.start();
        self.error(ParseError::UnexpectedToken {
            at: span.into(),
            expected: msg.to_string(),
            found,
        });

        while !self.at_set(recovery) && !self.is_at_end() {
            self.consume();
        }

        Some(marker.complete(self, SyntaxKind::Error))
    }
}
