use toy_cst::SyntaxKind;
use toy_lexer::Token;

mod sink;
mod source;

pub(crate) use sink::Sink;
pub(crate) use source::Source;

use crate::ParseError;

#[derive(Debug, PartialEq)]
pub enum Event<'a> {
    StartNode { kind: SyntaxKind, at: Option<usize> },
    AddToken { token: Token<'a> },
    FinishNode,
    Placeholder,
    Error(ParseError),
}
