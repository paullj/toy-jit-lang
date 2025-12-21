// Generated token definitions from tokens.toml
include!(concat!(env!("OUT_DIR"), "/token_kind.rs"));
include!(concat!(env!("OUT_DIR"), "/metadata.rs"));

#[cfg(test)]
mod tests {
    use super::*;
    use logos::Logos;

    #[test]
    fn test_integer_token() {
        let mut lexer = TokenKind::lexer("123");
        assert_eq!(lexer.next(), Some(Ok(TokenKind::Integer)));
    }

    #[test]
    fn test_operator_token() {
        let mut lexer = TokenKind::lexer("+");
        assert_eq!(lexer.next(), Some(Ok(TokenKind::Plus)));
    }

    #[test]
    fn test_category() {
        assert_eq!(TokenKind::Integer.category(), "number");
        assert_eq!(TokenKind::Plus.category(), "operator");
        assert_eq!(TokenKind::True.category(), "keyword");
    }

    #[test]
    fn test_semantic_token_type() {
        assert_eq!(TokenKind::Integer.semantic_token_type(), Some("number"));
        assert_eq!(TokenKind::Plus.semantic_token_type(), Some("operator"));
    }
}
