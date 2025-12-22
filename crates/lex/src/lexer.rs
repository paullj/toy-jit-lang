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
                text: self.inner.slice().to_string(),
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    fn kinds(input: &str) -> Vec<TokenKind> {
        Lexer::new(input)
            .filter_map(|r| r.ok())
            .map(|t| t.kind)
            .collect()
    }

    fn spans(input: &str) -> Vec<std::ops::Range<usize>> {
        Lexer::new(input)
            .filter_map(|r| r.ok())
            .map(|t| t.span)
            .collect()
    }

    #[rstest]
    #[case("x := 10", &[TokenKind::Identifier, TokenKind::Whitespace, TokenKind::Colon, TokenKind::Equals, TokenKind::Whitespace, TokenKind::Integer])]
    #[case("x: int = 10", &[TokenKind::Identifier, TokenKind::Colon, TokenKind::Whitespace, TokenKind::Identifier, TokenKind::Whitespace, TokenKind::Equals, TokenKind::Whitespace, TokenKind::Integer])]
    fn lex_kinds(#[case] input: &str, #[case] expected: &[TokenKind]) {
        assert_eq!(kinds(input), expected);
    }

    #[rstest]
    #[case("a b", &[0..1, 1..2, 2..3])]
    #[case(":=", &[0..1, 1..2])]
    fn lex_spans(#[case] input: &str, #[case] expected: &[std::ops::Range<usize>]) {
        assert_eq!(spans(input), expected);
    }

    #[test]
    fn invalid_token_error() {
        let results: Vec<_> = Lexer::new("a ? b").collect();
        let err = results.iter().find_map(|r| r.as_ref().err());
        assert_eq!(
            *err.unwrap(),
            Error::InvalidToken {
                at: (2..3).into(),
                text: "?".to_string()
            }
        );
    }

    #[test]
    fn recovers_after_error() {
        assert_eq!(
            kinds("a?b"),
            vec![TokenKind::Identifier, TokenKind::Identifier]
        );
    }
}
