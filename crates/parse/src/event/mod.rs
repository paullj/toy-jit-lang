use lex::Token;
use syntax::SyntaxKind;

mod sink;
mod source;

pub(crate) use sink::Sink;
pub(crate) use source::Source;

#[derive(Debug, PartialEq)]
pub enum Event<'a> {
    StartNode { kind: SyntaxKind, at: Option<usize> },
    AddToken { token: Token<'a> },
    FinishNode,
    Placeholder,
}
