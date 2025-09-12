mod function;
mod variable;

pub(crate) use function::function_definition;
use lex::TokenKind;
pub(crate) use variable::variable_definition;

pub(crate) const DEFINITION_KINDS: &[TokenKind] = &[TokenKind::Function, TokenKind::Identifier];
