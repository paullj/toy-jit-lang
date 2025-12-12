mod event;
mod grammar;
mod marker;

#[macro_use]
mod utils;

pub mod error;
pub mod parser;

use event::{Sink, Source};
use syntax::SyntaxNode;

pub use error::ParseError;
pub use parser::Parser;

pub fn parse(input: &str) -> (SyntaxNode, Vec<ParseError>) {
    let source = Source::new(input);
    let parser = Parser::new(source);

    let (events, lexer_errors) = parser.parse();
    
    let sink = Sink::new();
    let (green_node, mut parse_errors) = sink.build(events);
    
    // Convert lexer errors to parse errors and include them
    for lexer_error in lexer_errors {
        let parse_error = ParseError::from(lexer_error);
        parse_errors.push(parse_error);
    }

    (SyntaxNode::new_root(green_node), parse_errors)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_function() {
        let (tree, errors) = parse("fn test(x: int) -> int { x + 1 }");
        assert!(errors.is_empty(), "Valid function should parse without errors");
        assert!(tree.kind() == syntax::SyntaxKind::Root);
    }

    #[test]
    fn parse_missing_parameter_type() {
        let (tree, errors) = parse("fn test(x) -> int { x + 1 }");
        assert!(!errors.is_empty(), "Missing parameter type should produce errors");
        assert!(tree.kind() == syntax::SyntaxKind::Root);
        
        // Check that we get the right kind of errors
        let has_parameter_error = errors.iter().any(|e| matches!(e, ParseError::MissingParameterType { .. }));
        assert!(has_parameter_error, "Should have MissingParameterType error");
    }

    #[test]
    fn parse_invalid_character() {
        let (tree, errors) = parse("fn test() { @ }");
        assert!(!errors.is_empty(), "Invalid character should produce errors");
        assert!(tree.kind() == syntax::SyntaxKind::Root);
        
        // Check that lexer errors are converted to parse errors
        let has_unexpected_token = errors.iter().any(|e| matches!(e, ParseError::UnexpectedToken { .. }));
        assert!(has_unexpected_token, "Should have UnexpectedToken error from lexer");
    }

    #[test]
    fn parse_infinite_loop_prevention() {
        let (tree, errors) = parse("fn test(x: int { x + 1 }");
        assert!(!errors.is_empty(), "Malformed syntax should produce errors");
        assert!(tree.kind() == syntax::SyntaxKind::Root);
        
        // Should not hang and should produce limited number of errors
        assert!(errors.len() < 10, "Should not produce excessive errors due to infinite loop");
    }

    #[test]
    fn parse_error_recovery() {
        let (tree, errors) = parse("fn test() { invalid_statement; let x = 1; }");
        assert!(!errors.is_empty(), "Invalid statement should produce errors");
        assert!(tree.kind() == syntax::SyntaxKind::Root);
        
        // Parser should recover and continue parsing after error
        // This is indicated by the tree still being created successfully
    }
}
