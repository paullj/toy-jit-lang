use std::mem;
use std::ops::Range;

use lex::TokenKind;
use syntax::SyntaxKind;

use crate::error::ParseError;
use crate::event::{Event, Source};

use crate::grammar;
use crate::marker::{CompletedMarker, Marker};

const RECOVERABLE_KINDS: [TokenKind; 3] = [
    TokenKind::Colon,      // Variable declaration
    TokenKind::NewLine,    // Statement boundary
    TokenKind::Identifier, // Potential variable
];

/// Event-driven parser that converts tokens into parsing events
pub struct Parser<'a> {
    source: Source<'a>,
    pub(crate) events: Vec<Event<'a>>,
    expected_kinds: Vec<TokenKind>,
}

pub struct ErrorContext {
    pub(crate) found: Option<TokenKind>,
    pub(crate) at: Range<usize>,
    pub(crate) expected: Vec<TokenKind>,
}

impl ErrorContext {
    pub(crate) fn one_of(&self) -> String {
        let tokens: Vec<String> = self.expected.iter().map(|f| f.to_string()).collect();
        let (token_last, tokens) = match tokens.split_last() {
            Some((token_last, &[])) => return token_last.to_string(),
            Some((token_last, tokens)) => (token_last, tokens),
            None => return "nothing".to_string(),
        };

        let mut output = String::new();
        for token in tokens {
            output.push_str(token);
            output.push_str(", ");
        }
        output.push_str("or ");
        output.push_str(token_last);
        output
    }

    pub(crate) fn found_string(&self) -> Option<String> {
        self.found.map(|kind| kind.to_string())
    }
}

impl<'a> Parser<'a> {
    /// Creates a new parser from a token source
    pub fn new(source: Source<'a>) -> Self {
        Self {
            source,
            events: Vec::new(),
            expected_kinds: Vec::new(),
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

    /// Expects a specific token kind, consuming it or erroring
    pub(crate) fn expect<F>(&mut self, kind: TokenKind, callback: F)
    where
        F: Fn(ErrorContext) -> ParseError,
    {
        if self.is_at(kind) {
            self.consume();
        } else {
            self.error_with_callback(callback);
        }
    }

    #[allow(dead_code)] // Reserved for future use
    pub(crate) fn consume_trivia(&mut self) {
        let next = self.source.next_trivia();
        for trivia in next.trivia {
            self.events.push(Event::AddToken { token: trivia });
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

    pub(crate) fn error_with_callback<F>(&mut self, callback: F) -> Option<CompletedMarker>
    where
        F: Fn(ErrorContext) -> ParseError,
    {
        let (found, at) = match self.source.peek() {
            Some(token) => (Some(token.kind), token.span.clone()),
            None => (None, self.source.last_span()),
        };
        let expected = mem::take(&mut self.expected_kinds);

        let error = callback(ErrorContext {
            found,
            at,
            expected,
        });

        self.events.push(Event::Error(error));

        match self.is_at_one_of(&RECOVERABLE_KINDS) {
            None if !self.is_at_end() => {
                let marker = self.start();
                self.consume();
                Some(marker.complete(self, SyntaxKind::Error))
            }
            _ => None,
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
    pub(crate) fn is_at_one_of(&mut self, set: &[TokenKind]) -> Option<TokenKind> {
        self.source
            .peek()
            .filter(|t| set.contains(&t.kind))
            .map(|t| t.kind)
    }

    /// Checks if at end of input
    pub(crate) fn is_at_end(&mut self) -> bool {
        self.source.peek().is_none()
    }

    /// Creates a standard unexpected token error
    pub(crate) fn unexpected_token_error(&mut self) -> Option<CompletedMarker> {
        self.error_with_callback(|ctx| ParseError::UnexpectedToken {
            at: ctx.at.clone().into(),
            expected: ctx.one_of(),
            found: ctx.found_string(),
        })
    }

    /// Returns the current parser position for progress tracking
    pub(crate) fn position(&mut self) -> usize {
        if let Some(token) = self.source.peek() {
            token.span.start
        } else {
            self.source.last_span().start
        }
    }
}
