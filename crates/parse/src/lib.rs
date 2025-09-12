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

pub fn parse(input: &str) -> Result<SyntaxNode, Vec<ParseError>> {
    let source = Source::new(input);
    let parser = Parser::new(source);

    let events = parser.parse();
    let sink = Sink::new();
    let (green_node, errors) = sink.build(events);

    if errors.is_empty() {
        Ok(SyntaxNode::new_root(green_node))
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_test() {}
}
