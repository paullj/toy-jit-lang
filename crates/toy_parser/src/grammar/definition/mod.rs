mod function;
mod variable;

pub(crate) use function::{
    function_definition_or_expression, function_definition_or_expression_pub,
};
pub(crate) use variable::{
    type_annotation, variable_definition_inferred, variable_definition_typed,
};
