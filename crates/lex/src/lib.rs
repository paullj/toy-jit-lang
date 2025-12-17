mod error;
mod lexer;
mod token;
mod token_kind;

pub use error::Error;
pub use lexer::Lexer;
pub use token::Token;
pub use token_kind::TokenKind;

pub fn lex(input: &str) -> Vec<Result<Token<'_>, Error>> {
    let lexer = Lexer::new(input);
    lexer.collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lex_function() {
        let tokens = lex("x := 1");
        assert_eq!(tokens.len(), 6); // x, ws, :, =, ws, 1
        assert!(tokens.iter().all(|r| r.is_ok()));
    }
}
