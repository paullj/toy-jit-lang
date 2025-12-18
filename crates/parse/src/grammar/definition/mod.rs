mod variable;

use lex::TokenKind;
pub(crate) use variable::{variable_definition_inferred, variable_definition_typed};

pub(crate) const DEFINITION_KINDS: &[TokenKind] = &[TokenKind::Identifier];
