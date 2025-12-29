use std::{iter::Peekable, ops::Range};

use lex::{Lexer, Token};

/// Token source that separates trivia from meaningful tokens
pub struct Source<'a> {
    source_text: &'a str,
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
            source_text: source,
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

    /// Peeks at the Nth non-trivia token (0-indexed, so peek_nth(0) == peek())
    /// Uses speculative lexing to look ahead without consuming tokens.
    pub(crate) fn peek_nth(&mut self, n: usize) -> Option<lex::TokenKind> {
        self.consume_trivia();

        // Get current position
        let start_offset = match self.internal_peek() {
            Some(t) => t.span.start,
            None => return None,
        };

        // Create a new lexer from current position for speculative scanning
        let remaining = &self.source_text[start_offset..];
        let mut scan_lexer = Lexer::new(remaining);

        let mut count = 0;
        for result in scan_lexer.by_ref() {
            let Ok(token) = result else { continue };

            // Skip trivia
            if token.kind.is_trivia() {
                continue;
            }

            if count == n {
                return Some(token.kind);
            }
            count += 1;
        }

        None
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

    /// Check if there's a newline in pending trivia (before next non-trivia token)
    pub(crate) fn has_newline_before_next(&mut self) -> bool {
        self.consume_trivia();
        self.buffer
            .iter()
            .any(|t| t.kind == lex::TokenKind::NewLine)
    }

    /// Lookahead to check if bracket expression is index/slice (not list literal).
    /// Returns true if the `[...]` starting at current position contains no comma at depth 1.
    /// Used to determine if `x\n[0]` should continue as postfix index.
    /// This uses speculative re-lexing to avoid consuming tokens.
    pub(crate) fn is_bracket_index_not_list(&mut self) -> bool {
        use lex::TokenKind;

        self.consume_trivia();

        // Must be at `[`
        let start_offset = match self.internal_peek() {
            Some(t) if t.kind == TokenKind::LeftBracket => t.span.start,
            _ => return false,
        };

        // Create a new lexer from current position for speculative scanning
        let remaining = &self.source_text[start_offset..];
        let mut scan_lexer = Lexer::new(remaining);

        let mut depth: u32 = 0;

        for result in scan_lexer.by_ref() {
            let Ok(token) = result else { continue };

            // Skip trivia
            if token.kind.is_trivia() {
                continue;
            }

            match token.kind {
                TokenKind::LeftBracket | TokenKind::LeftParenthesis | TokenKind::LeftBrace => {
                    depth += 1;
                }
                TokenKind::RightBracket => {
                    if depth == 1 {
                        // Found matching `]` - is index expression
                        return true;
                    }
                    depth -= 1;
                }
                TokenKind::RightParenthesis | TokenKind::RightBrace => {
                    depth = depth.saturating_sub(1);
                }
                TokenKind::Comma if depth == 1 => {
                    // Found comma at depth 1 - is list expression
                    return false;
                }
                _ => {}
            }
        }

        // EOF before closing - assume not a valid index (shouldn't happen normally)
        false
    }
}
