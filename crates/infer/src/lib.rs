mod context;
mod diagnostic;
mod env;
mod ops;
mod scheme;
mod subst;
pub mod types;
mod unify;

pub use diagnostic::InferDiagnostic;
pub use types::Type;

use la_arena::ArenaMap;
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct InferenceResult {
    pub expression_types: ArenaMap<hir::ExprIdx, Type>,
    pub variable_types: HashMap<String, Type>,
    pub diagnostics: Vec<InferDiagnostic>,
}

impl InferenceResult {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn has_errors(&self) -> bool {
        !self.diagnostics.is_empty()
    }

    pub fn get_expression_type(&self, idx: hir::ExprIdx) -> Option<&Type> {
        self.expression_types.get(idx)
    }

    pub fn get_variable_type(&self, name: &str) -> Option<&Type> {
        self.variable_types.get(name)
    }
}

pub fn infer(lower_result: &hir::LowerResult) -> InferenceResult {
    let ctx = context::InferCtx::new(lower_result);
    ctx.infer_items()
}

#[cfg(test)]
mod tests {
    use super::*;
    use hir::{Definition, Expression, Item, Literal};

    fn make_lower_result(items: Vec<Item>) -> hir::LowerResult {
        let item_spans = vec![Default::default(); items.len()];
        hir::LowerResult {
            items,
            expressions: Default::default(),
            expr_spans: Default::default(),
            item_spans,
        }
    }

    #[test]
    fn test_variable_integer() {
        let result = make_lower_result(vec![Item::Definition(Definition::Variable {
            name: "x".to_string(),
            value: Expression::Literal(Literal::Integer(42)),
        })]);

        let inferred = infer(&result);
        assert_eq!(inferred.get_variable_type("x"), Some(&Type::Integer));
        assert!(!inferred.has_errors());
    }

    #[test]
    fn test_variable_float() {
        let result = make_lower_result(vec![Item::Definition(Definition::Variable {
            name: "y".to_string(),
            value: Expression::Literal(Literal::Float(3.5)),
        })]);

        let inferred = infer(&result);
        assert_eq!(inferred.get_variable_type("y"), Some(&Type::Float));
    }

    #[test]
    fn test_variable_boolean() {
        let result = make_lower_result(vec![Item::Definition(Definition::Variable {
            name: "flag".to_string(),
            value: Expression::Literal(Literal::Boolean(true)),
        })]);

        let inferred = infer(&result);
        assert_eq!(inferred.get_variable_type("flag"), Some(&Type::Boolean));
    }

    #[test]
    fn test_variable_string() {
        let result = make_lower_result(vec![Item::Definition(Definition::Variable {
            name: "s".to_string(),
            value: Expression::Literal(Literal::String("hello".to_string())),
        })]);

        let inferred = infer(&result);
        assert_eq!(inferred.get_variable_type("s"), Some(&Type::String));
    }

    #[test]
    fn test_undefined_variable() {
        let result = make_lower_result(vec![Item::Assignment {
            name: "unknown".to_string(),
            value: Expression::Literal(Literal::Integer(1)),
        }]);

        let inferred = infer(&result);
        assert!(inferred.has_errors());
    }

    #[test]
    fn test_missing_expr_returns_error() {
        let result = make_lower_result(vec![Item::Definition(Definition::Variable {
            name: "x".to_string(),
            value: Expression::Missing,
        })]);

        let inferred = infer(&result);
        assert_eq!(inferred.get_variable_type("x"), Some(&Type::Error));
    }
}
