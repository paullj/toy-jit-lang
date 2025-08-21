use std::iter::Peekable;

use lex::{Lexer, Token};

pub(crate) struct Source<'a> {
    lexer: Peekable<Lexer<'a>>,
    trivia_buffer: Vec<Token<'a>>,
}

pub(crate) struct WithTrivia<'a, T> {
    pub trivia: Vec<Token<'a>>,
    pub token: Option<T>,
}

impl<'a> Source<'a> {
    pub(crate) fn new(source: &'a str) -> Self {
        Self {
            lexer: Lexer::new(source).peekable(),
            trivia_buffer: Vec::new(),
        }
    }

    fn internal_next(&mut self) -> Option<Token<'a>> {
        match self.lexer.next() {
            Some(Ok(token)) => Some(token),
            Some(Err(err)) => todo!("Handle unexpected tokens: {:?}", err),
            None => None,
        }
    }

    fn internal_peek(&mut self) -> Option<&Token<'a>> {
        match self.lexer.peek() {
            Some(Ok(token)) => Some(token),
            Some(Err(err)) => todo!("Handle unexpected tokens: {:?}", err),
            None => None,
        }
    }

    pub(crate) fn next(&mut self) -> WithTrivia<'a, Token<'a>> {
        self.consume_trivia();

        let trivia = std::mem::take(&mut self.trivia_buffer);
        let token = self.internal_next();

        WithTrivia { trivia, token }
    }

    pub(crate) fn peek(&mut self) -> Option<&Token<'a>> {
        self.consume_trivia();
        self.internal_peek()
    }

    pub(crate) fn at_trivia(&mut self) -> bool {
        if let Some(token) = self.internal_peek() {
            token.kind.is_trivia()
        } else {
            false
        }
    }

    pub(crate) fn consume_trivia(&mut self) {
        while self.at_trivia() {
            if let Some(token) = self.internal_next() {
                self.trivia_buffer.push(token);
            }
        }
    }
}
