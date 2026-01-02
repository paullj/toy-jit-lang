mod function;
mod struct_def;
mod variable;

pub(crate) use function::function_definition_or_expression;
pub(crate) use struct_def::struct_definition;
pub(crate) use variable::{
    type_annotation, variable_definition_inferred, variable_definition_typed,
};
