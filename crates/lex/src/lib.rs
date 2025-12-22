#![allow(unused_assignments)] // false positive from thiserror derive on Error::InvalidToken.text

mod error;
mod lexer;
mod token;

pub use error::Error;
pub use lexer::Lexer;
pub use token::Token;
pub use tokens::TokenKind;

pub fn lex(input: &str) -> Vec<Result<Token<'_>, Error>> {
    let lexer = Lexer::new(input);
    lexer.collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lex_variable_definition() {
        let tokens = lex("x := 1");
        assert_eq!(tokens.len(), 6); // x, ws, :, =, ws, 1
        assert!(tokens.iter().all(|r| r.is_ok()));
    }

    #[test]
    fn lex_doc_comment_vs_comment() {
        let tokens = lex("## doc comment");
        let first = tokens[0].as_ref().unwrap();
        assert_eq!(
            first.kind,
            TokenKind::DocComment,
            "## should be DocComment, got {:?}",
            first.kind
        );

        let tokens = lex("# regular comment");
        let first = tokens[0].as_ref().unwrap();
        assert_eq!(
            first.kind,
            TokenKind::Comment,
            "# should be Comment, got {:?}",
            first.kind
        );
    }
}
