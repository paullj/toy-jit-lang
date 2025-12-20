mod context;
mod diagnostic;
mod env;
mod ops;
mod scheme;
mod subst;
pub mod types;
mod unify;

pub use diagnostic::InferDiagnostic;
pub use env::TypeEnv;
pub use scheme::Scheme;
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

/// Persistent state for incremental type inference in REPL.
#[derive(Debug, Clone, Default)]
pub struct InferState {
    pub env: TypeEnv,
    pub next_var: u32,
}

impl InferState {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Result of inference with updated state for REPL.
pub struct InferWithState {
    pub result: InferenceResult,
    pub state: InferState,
}

pub fn infer(lower_result: &hir::LowerResult) -> InferenceResult {
    let ctx = context::InferCtx::new(lower_result);
    ctx.infer_items()
}

/// Infer with existing state, returning updated state for subsequent calls.
pub fn infer_with_state(lower_result: &hir::LowerResult, state: InferState) -> InferWithState {
    let ctx = context::InferCtx::with_env(lower_result, state.env, state.next_var);
    ctx.infer_items_with_state()
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

    #[test]
    fn test_incremental_inference() {
        // First command: x := 1
        let result1 = make_lower_result(vec![Item::Definition(Definition::Variable {
            name: "x".to_string(),
            value: Expression::Literal(Literal::Integer(42)),
        })]);

        let infer1 = infer_with_state(&result1, InferState::new());
        assert_eq!(infer1.result.get_variable_type("x"), Some(&Type::Integer));

        // Second command: x = 10 (should work because x is in state)
        let result2 = make_lower_result(vec![Item::Assignment {
            name: "x".to_string(),
            value: Expression::Literal(Literal::Integer(10)),
        }]);

        let infer2 = infer_with_state(&result2, infer1.state);
        assert!(
            !infer2.result.has_errors(),
            "assignment to existing var should work"
        );
    }

    #[test]
    fn test_incremental_inference_undefined() {
        // First command: assign to undefined y (should error)
        let result1 = make_lower_result(vec![Item::Assignment {
            name: "y".to_string(),
            value: Expression::Literal(Literal::Integer(1)),
        }]);

        let infer1 = infer_with_state(&result1, InferState::new());
        assert!(
            infer1.result.has_errors(),
            "assignment to undefined var should error"
        );
    }
}
