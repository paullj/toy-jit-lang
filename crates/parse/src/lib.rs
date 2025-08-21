pub mod error;
mod event;
mod grammar;

mod marker;
pub mod parser;
pub use error::ParseError;
use event::{Sink, Source};
pub use parser::Parser;
use syntax::SyntaxNode;

pub fn parse(input: &str) -> (SyntaxNode, Vec<ParseError>) {
    let source = Source::new(input);
    let parser = Parser::new(source);

    let (events, errors) = parser.parse();
    let sink = Sink::new();
    let green_node = sink.build(events);
    (SyntaxNode::new_root(green_node), errors)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_test() {
        let (tree, _errors) = parse("my_var := 100");
        println!("{:#?}", tree);
    }
}
