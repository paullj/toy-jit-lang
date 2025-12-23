mod context;
mod diagnostic;
mod env;
mod ops;
mod scheme;
mod subst;
mod suggest;
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
    use ast::AstNode;
    use hir::{Definition, Expression, Item, Literal};

    fn make_lower_result(items: Vec<Item>) -> hir::LowerResult {
        let item_spans = vec![Default::default(); items.len()];
        hir::LowerResult {
            items,
            expressions: Default::default(),
            expr_spans: Default::default(),
            item_spans,
            symbols: Default::default(),
            diagnostics: Default::default(),
        }
    }

    /// Helper to infer types from source code using the full pipeline
    fn infer_from_source(source: &str) -> InferenceResult {
        let (syntax, _errors) = parse::parse(source);
        let root = ast::Root::cast(syntax).expect("failed to cast to Root");
        let hir = hir::lower(root);
        infer(&hir)
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

    // ===================== Function Type Inference Tests =====================

    #[test]
    fn test_function_simple() {
        let result = infer_from_source("fn add(a, b) { a + b }");
        assert!(!result.has_errors(), "no errors expected");

        let fn_ty = result
            .get_variable_type("add")
            .expect("add should be defined");
        // Should be (int, int) -> int from the + operator
        assert!(
            matches!(fn_ty, Type::Function { params, ret } if params.len() == 2 && **ret == Type::Integer),
            "expected (int, int) -> int, got {fn_ty}"
        );
    }

    #[test]
    fn test_function_with_type_annotation() {
        let result = infer_from_source("fn greet(name: string): string { name }");
        assert!(!result.has_errors(), "no errors expected");

        let fn_ty = result
            .get_variable_type("greet")
            .expect("greet should be defined");
        assert!(
            matches!(fn_ty, Type::Function { params, ret }
                if params == &[Type::String] && **ret == Type::String),
            "expected string -> string, got {fn_ty}"
        );
    }

    #[test]
    fn test_function_call() {
        let result = infer_from_source(
            "fn double(x) { x + x }
             double(5)",
        );
        assert!(!result.has_errors(), "no errors expected");
    }

    #[test]
    fn test_lambda_expression() {
        let result = infer_from_source("inc := fn(x) { x + 1 }");
        assert!(!result.has_errors(), "no errors expected");

        let fn_ty = result
            .get_variable_type("inc")
            .expect("inc should be defined");
        assert!(
            matches!(fn_ty, Type::Function { params, ret }
                if params.len() == 1 && **ret == Type::Integer),
            "expected int -> int, got {fn_ty}"
        );
    }

    #[test]
    fn test_recursive_function() {
        let result = infer_from_source(
            "fn fac(n) {
                if n <= 1 { 1 }
                else { n * fac(n - 1) }
             }",
        );
        assert!(!result.has_errors(), "no errors expected");

        let fn_ty = result
            .get_variable_type("fac")
            .expect("fac should be defined");
        assert!(
            matches!(fn_ty, Type::Function { params, ret }
                if params == &[Type::Integer] && **ret == Type::Integer),
            "expected int -> int, got {fn_ty}"
        );
    }

    #[test]
    fn test_higher_order_function() {
        let result = infer_from_source(
            "fn apply(f, x) { f(x) }
             fn double(n) { n + n }
             apply(double, 5)",
        );
        assert!(!result.has_errors(), "no errors expected");
    }

    #[test]
    fn test_function_call_arity_mismatch() {
        let result = infer_from_source(
            "fn add(a, b) { a + b }
             add(1)",
        );
        assert!(result.has_errors(), "should error on arity mismatch");
    }

    #[test]
    fn test_call_non_function() {
        let result = infer_from_source(
            "x := 42
             x(1)",
        );
        assert!(
            result.has_errors(),
            "should error when calling non-function"
        );
    }

    #[test]
    fn test_return_type_mismatch() {
        let result = infer_from_source("fn foo(): int { true }");
        assert!(result.has_errors(), "should error on return type mismatch");
    }

    #[test]
    fn test_function_no_params() {
        let result = infer_from_source("fn answer() { 42 }");
        assert!(!result.has_errors(), "no errors expected");

        let fn_ty = result
            .get_variable_type("answer")
            .expect("answer should be defined");
        assert!(
            matches!(fn_ty, Type::Function { params, ret }
                if params.is_empty() && **ret == Type::Integer),
            "expected () -> int, got {fn_ty}"
        );
    }

    #[test]
    fn test_polymorphic_identity() {
        // Identity function should work with different types
        let result = infer_from_source(
            "fn id(x) { x }
             id(42)
             id(true)",
        );
        assert!(!result.has_errors(), "polymorphic identity should work");
    }
}
