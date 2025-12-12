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
            Err(_) => Some(Err(Error::InvalidToken {
                at: self.inner.span().into(),
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case("x := 10", vec![TokenKind::Identifier, TokenKind::Whitespace, TokenKind::ColonEquals, TokenKind::Whitespace, TokenKind::Integer])]
    #[case("a + b * c", vec![TokenKind::Identifier, TokenKind::Whitespace, TokenKind::Plus, TokenKind::Whitespace, TokenKind::Identifier, TokenKind::Whitespace, TokenKind::Asterisk, TokenKind::Whitespace, TokenKind::Identifier])]
    fn test_expression(#[case] input: &str, #[case] expected_kinds: Vec<TokenKind>) {
        let lexer = Lexer::new(input);
        let kinds: Vec<TokenKind> = lexer
            .map(|t| t.expect("unexpected error").kind)
            .into_iter()
            .collect();

        assert!(!kinds.is_empty(), "Expected at least one token result");
        assert_eq!(kinds, expected_kinds);
    }

    #[rstest]
    #[case("?")]
    fn test_unexpected_token(#[case] input: &str) {
        let lexer = Lexer::new(input);
        let results: Vec<Result<Token, Error>> = lexer.collect();

        // Check that we have at least one result and that it contains an error
        assert!(!results.is_empty(), "Expected at least one token result");
        assert!(
            results.iter().any(|result| result.is_err()),
            "Expected at least one error in the results"
        );
    }
}
