use std::{iter::Peekable, ops::Range};

use lex::{Lexer, Token};

/// Token source that separates trivia from meaningful tokens
pub struct Source<'a> {
    lexer: Peekable<Lexer<'a>>,
    buffer: Vec<Token<'a>>,
    last_span: Range<usize>,
    pending_errors: Vec<lex::Error>,
}

/// Wrapper that includes trivia tokens alongside the main token
pub(crate) struct WithTrivia<'a, T> {
    pub trivia: Vec<Token<'a>>,
    pub token: Option<T>,
}

impl<'a> Source<'a> {
    /// Creates a new token source from source code
    pub(crate) fn new(source: &'a str) -> Self {
        Self {
            lexer: Lexer::new(source).peekable(),
            buffer: Vec::new(),
            last_span: source.len()..source.len(),
            pending_errors: Vec::new(),
        }
    }

    fn internal_next(&mut self) -> Option<Token<'a>> {
        match self.lexer.next() {
            Some(Ok(token)) => {
                self.last_span = token.span.clone();
                Some(token)
            }
            Some(Err(err)) => {
                // Store the error for later reporting
                self.pending_errors.push(err);
                // Skip this invalid token and try the next one
                self.internal_next()
            }
            None => None,
        }
    }

    fn internal_peek(&mut self) -> Option<&Token<'a>> {
        // Keep consuming error tokens until we find a valid token or reach end
        while let Some(Err(_)) = self.lexer.peek() {
            if let Some(Err(err)) = self.lexer.next() {
                self.pending_errors.push(err);
            }
        }

        // Now peek should either be Some(Ok(token)) or None
        match self.lexer.peek() {
            Some(Ok(token)) => Some(token),
            _ => None,
        }
    }

    /// Returns next token with any preceding trivia
    pub(crate) fn next(&mut self) -> WithTrivia<'a, Token<'a>> {
        self.consume_trivia();

        // NOTE: take() moves trivia out of buffer, leaving empty vec
        let trivia = std::mem::take(&mut self.buffer);
        let token = self.internal_next();

        WithTrivia { trivia, token }
    }

    /// Peeks at next non-trivia token
    pub(crate) fn peek(&mut self) -> Option<&Token<'a>> {
        self.consume_trivia();
        self.internal_peek()
    }

    /// Checks if current token is trivia without consuming it
    pub(crate) fn at_trivia(&mut self) -> bool {
        if let Some(token) = self.internal_peek() {
            token.kind.is_trivia()
        } else {
            false
        }
    }

    /// Consumes all consecutive trivia tokens into buffer
    pub(crate) fn consume_trivia(&mut self) {
        while self.at_trivia() {
            if let Some(token) = self.internal_next() {
                self.buffer.push(token);
            }
        }
    }

    #[allow(dead_code)] // Reserved for future use
    pub(crate) fn next_trivia(&mut self) -> WithTrivia<'a, Token<'a>> {
        self.consume_trivia();

        // NOTE: take() moves trivia out of buffer, leaving empty vec
        let trivia = std::mem::take(&mut self.buffer);

        WithTrivia {
            trivia,
            token: None,
        }
    }

    // pub(crate) fn at_new_line(&mut self) -> bool {
    //     if let Some(token) = self.internal_peek() {
    //         token.kind == TokenKind::NewLine
    //     } else {
    //         false
    //     }
    // }

    // pub(crate) fn consume_till_new_line(&mut self) {
    //     while !self.at_new_line() {
    //         if let Some(token) = self.internal_next() {
    //             self.buffer.push(token);
    //         }
    //     }
    // }

    /// Returns the last span of the source code
    pub(crate) fn last_span(&self) -> Range<usize> {
        self.last_span.clone()
    }

    /// Returns and clears any pending lexer errors
    pub(crate) fn take_pending_errors(&mut self) -> Vec<lex::Error> {
        std::mem::take(&mut self.pending_errors)
    }
}
