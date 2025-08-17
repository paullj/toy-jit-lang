use crate::{Error, Token, TokenKind};
use logos::Logos;

#[derive(Debug)]
pub struct Lexer<'a> {
    inner: logos::Lexer<'a, TokenKind>,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            inner: TokenKind::lexer(source),
        }
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Result<Token<'a>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        let result = self.inner.next()?;

        match result {
            Ok(token_kind) => Some(Ok(Token {
                kind: token_kind,
                text: self.inner.slice(),
                span: self.inner.span(),
            })),
            Err(error) => Some(Err(error)),
        }
    }
}
