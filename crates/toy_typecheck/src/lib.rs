mod context;
mod diagnostic;
mod env;
mod ops;
mod scheme;
mod subst;
mod suggest;
pub mod typed_hir;
pub mod types;
mod unify;

pub use diagnostic::InferDiagnostic;
pub use env::TypeEnv;
pub use scheme::Scheme;
pub use typed_hir::{TypedExpression, TypedItem, TypedModule, TypedModules};
pub use types::Type;

use la_arena::ArenaMap;
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct InferenceResult {
    pub expression_types: ArenaMap<toy_hir::ExprIdx, Type>,
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

    pub fn get_expression_type(&self, idx: toy_hir::ExprIdx) -> Option<&Type> {
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

pub fn infer(lower_result: &toy_hir::LowerResult) -> InferenceResult {
    let ctx = context::InferCtx::new(lower_result);
    ctx.infer_items()
}

/// Infer types using an external interner (for incremental compilation).
///
/// Use this when the LowerResult was constructed with merged items from
/// different sources that used a shared interner.
pub fn infer_with_interner(
    lower_result: &toy_hir::LowerResult,
    interner: &toy_hir::Interner,
) -> InferenceResult {
    let ctx = context::InferCtx::with_interner(lower_result, interner);
    ctx.infer_items()
}

/// Infer with existing state, returning updated state for subsequent calls.
pub fn infer_with_state(lower_result: &toy_hir::LowerResult, state: InferState) -> InferWithState {
    let ctx = context::InferCtx::with_env(lower_result, state.env, state.next_var);
    ctx.infer_items_with_state()
}

/// Infer types for a module and produce TypedModule.
///
/// This is the main entry point for module-level type inference in the compiler pipeline.
/// It takes HIR items for a single module and produces a TypedModule with all type information.
pub fn infer_module(
    module_id: toy_hir::ModuleId,
    hir_items: &[std::sync::Arc<toy_hir::LowerItemResult>],
    interner: &toy_hir::Interner,
) -> TypedModule {
    // Build combined HIR for the module
    let combined = build_module_hir(hir_items);

    // Run type inference with the shared interner
    let inference = infer_with_interner(&combined, interner);

    // Split expression types back to per-item
    let expr_types_per_item = split_module_expression_types(hir_items, &inference);

    // Convert to TypedModule - pass the expressions arena
    typed_hir::from_inference_result(
        module_id,
        hir_items,
        &inference,
        &expr_types_per_item,
        combined.expressions,
    )
}

/// Build a combined LowerResult for a single module's items.
fn build_module_hir(items: &[std::sync::Arc<toy_hir::LowerItemResult>]) -> toy_hir::LowerResult {
    use la_arena::Arena;
    use toy_hir::{Expression, Interner, LowerResult, SymbolTable};

    let mut combined_items = Vec::new();
    let mut combined_expressions: Arena<Expression> = Arena::new();
    let mut combined_expr_spans = ArenaMap::new();
    let mut combined_item_spans = Vec::new();
    let mut combined_symbols = SymbolTable::default();

    for item_result in items {
        let offset = combined_expressions.len();

        // Remap the item to use combined arena indices
        let remapped_item = remap_hir_item(&item_result.item, offset);
        combined_items.push(remapped_item);
        combined_item_spans.push(item_result.item_span);

        // Copy expressions with remapped indices
        for (old_idx, expr) in item_result.expressions.iter() {
            let remapped_expr = remap_hir_expression(expr, offset);
            let new_idx = combined_expressions.alloc(remapped_expr);

            if let Some(span) = item_result.expr_spans.get(old_idx) {
                combined_expr_spans.insert(new_idx, *span);
            }
        }

        // Merge symbols
        for (name, symbol) in item_result.symbols.iter() {
            combined_symbols.define(
                name.clone(),
                symbol.kind,
                symbol.def_span,
                symbol.name_span,
                symbol.doc_comment.clone(),
            );
            for ref_span in &symbol.references {
                combined_symbols.add_reference(name, *ref_span);
            }
        }
    }

    LowerResult {
        items: combined_items,
        expressions: combined_expressions,
        expr_spans: combined_expr_spans,
        item_spans: combined_item_spans,
        symbols: combined_symbols,
        diagnostics: vec![],
        interner: Interner::default(), // Dummy; use infer_with_interner
    }
}

/// Split combined expression types back to per-item maps.
fn split_module_expression_types(
    items: &[std::sync::Arc<toy_hir::LowerItemResult>],
    inference: &InferenceResult,
) -> Vec<ArenaMap<toy_hir::ExprIdx, Type>> {
    let mut result = Vec::with_capacity(items.len());
    let mut offset = 0usize;

    for item in items {
        let item_expr_count = item.expressions.len();
        let mut item_types = ArenaMap::new();

        for (old_idx, _) in item.expressions.iter() {
            let combined_idx = remap_expr_idx(old_idx, offset);
            if let Some(ty) = inference.expression_types.get(combined_idx) {
                item_types.insert(old_idx, ty.clone());
            }
        }

        result.push(item_types);
        offset += item_expr_count;
    }

    result
}

/// Helper function to remap HIR items with expression index offsets.
fn remap_hir_item(item: &toy_hir::Item, offset: usize) -> toy_hir::Item {
    use toy_hir::{Definition, Item};

    match item {
        Item::Definition(Definition::Variable { name, value }) => {
            Item::Definition(Definition::Variable {
                name: *name,
                value: remap_hir_expression(value, offset),
            })
        }
        Item::Definition(Definition::Function {
            name,
            params,
            return_type,
            body,
        }) => {
            let remapped_params: Vec<_> = params
                .iter()
                .map(|p| toy_hir::FunctionParam {
                    name: p.name,
                    ty: p.ty,
                    default: p.default.map(|idx| remap_expr_idx(idx, offset)),
                })
                .collect();
            Item::Definition(Definition::Function {
                name: *name,
                params: remapped_params,
                return_type: *return_type,
                body: remap_expr_idx(*body, offset),
            })
        }
        Item::Assignment { name, value } => Item::Assignment {
            name: *name,
            value: remap_hir_expression(value, offset),
        },
        Item::IndexAssignment {
            collection,
            index,
            value,
        } => Item::IndexAssignment {
            collection: remap_expr_idx(*collection, offset),
            index: remap_expr_idx(*index, offset),
            value: remap_hir_expression(value, offset),
        },
        Item::Expression(expr) => Item::Expression(remap_hir_expression(expr, offset)),
    }
}

/// Helper function to remap expression indices by adding an offset.
fn remap_hir_expression(expr: &toy_hir::Expression, offset: usize) -> toy_hir::Expression {
    use toy_hir::Expression;

    match expr {
        Expression::Missing => Expression::Missing,
        Expression::Literal(lit) => Expression::Literal(lit.clone()),
        Expression::Infix { op, lhs, rhs } => Expression::Infix {
            op: *op,
            lhs: remap_expr_idx(*lhs, offset),
            rhs: remap_expr_idx(*rhs, offset),
        },
        Expression::Prefix { op, expr } => Expression::Prefix {
            op: *op,
            expr: remap_expr_idx(*expr, offset),
        },
        Expression::VariableRef { name } => Expression::VariableRef { name: *name },
        Expression::Block { items, tail } => Expression::Block {
            items: items.iter().map(|i| remap_block_item(i, offset)).collect(),
            tail: tail.map(|idx| remap_expr_idx(idx, offset)),
        },
        Expression::If {
            condition,
            then_branch,
            else_branch,
        } => Expression::If {
            condition: remap_expr_idx(*condition, offset),
            then_branch: remap_expr_idx(*then_branch, offset),
            else_branch: else_branch.map(|idx| remap_expr_idx(idx, offset)),
        },
        Expression::Function {
            params,
            return_type,
            body,
            captures,
        } => {
            let remapped_params: Vec<_> = params
                .iter()
                .map(|p| toy_hir::FunctionParam {
                    name: p.name,
                    ty: p.ty,
                    default: p.default.map(|idx| remap_expr_idx(idx, offset)),
                })
                .collect();
            Expression::Function {
                params: remapped_params,
                return_type: *return_type,
                body: remap_expr_idx(*body, offset),
                captures: captures.clone(),
            }
        }
        Expression::Call { callee, args } => Expression::Call {
            callee: remap_expr_idx(*callee, offset),
            args: args
                .iter()
                .map(|idx| remap_expr_idx(*idx, offset))
                .collect(),
        },
        Expression::Return { value } => Expression::Return {
            value: value.map(|idx| remap_expr_idx(idx, offset)),
        },
        Expression::Echo { value } => Expression::Echo {
            value: remap_expr_idx(*value, offset),
        },
        Expression::Loop { label, body } => Expression::Loop {
            label: *label,
            body: remap_expr_idx(*body, offset),
        },
        Expression::While {
            condition,
            label,
            body,
        } => Expression::While {
            condition: remap_expr_idx(*condition, offset),
            label: *label,
            body: remap_expr_idx(*body, offset),
        },
        Expression::For {
            binding,
            iterable,
            label,
            body,
        } => Expression::For {
            binding: *binding,
            iterable: remap_expr_idx(*iterable, offset),
            label: *label,
            body: remap_expr_idx(*body, offset),
        },
        Expression::Range { start, end } => Expression::Range {
            start: remap_expr_idx(*start, offset),
            end: remap_expr_idx(*end, offset),
        },
        Expression::Break { label } => Expression::Break { label: *label },
        Expression::Continue { label } => Expression::Continue { label: *label },
        Expression::List { elements } => Expression::List {
            elements: elements
                .iter()
                .map(|idx| remap_expr_idx(*idx, offset))
                .collect(),
        },
        Expression::Index { collection, index } => Expression::Index {
            collection: remap_expr_idx(*collection, offset),
            index: remap_expr_idx(*index, offset),
        },
        Expression::Slice {
            collection,
            start,
            end,
        } => Expression::Slice {
            collection: remap_expr_idx(*collection, offset),
            start: start.map(|idx| remap_expr_idx(idx, offset)),
            end: end.map(|idx| remap_expr_idx(idx, offset)),
        },
        Expression::Tuple { elements } => Expression::Tuple {
            elements: elements
                .iter()
                .map(|idx| remap_expr_idx(*idx, offset))
                .collect(),
        },
        Expression::TupleAccess { tuple, index } => Expression::TupleAccess {
            tuple: remap_expr_idx(*tuple, offset),
            index: *index,
        },
    }
}

/// Helper to remap block items.
fn remap_block_item(item: &toy_hir::BlockItem, offset: usize) -> toy_hir::BlockItem {
    use toy_hir::BlockItem;

    match item {
        BlockItem::Definition { name, value } => BlockItem::Definition {
            name: *name,
            value: remap_expr_idx(*value, offset),
        },
        BlockItem::Assignment { name, value } => BlockItem::Assignment {
            name: *name,
            value: remap_expr_idx(*value, offset),
        },
        BlockItem::IndexAssignment {
            collection,
            index,
            value,
        } => BlockItem::IndexAssignment {
            collection: remap_expr_idx(*collection, offset),
            index: remap_expr_idx(*index, offset),
            value: remap_expr_idx(*value, offset),
        },
        BlockItem::Expression(idx) => BlockItem::Expression(remap_expr_idx(*idx, offset)),
        BlockItem::Return { value } => BlockItem::Return {
            value: value.map(|idx| remap_expr_idx(idx, offset)),
        },
        BlockItem::Echo { value } => BlockItem::Echo {
            value: remap_expr_idx(*value, offset),
        },
        BlockItem::Break { label } => BlockItem::Break { label: *label },
        BlockItem::Continue { label } => BlockItem::Continue { label: *label },
    }
}

/// Helper to remap an expression index by adding an offset.
fn remap_expr_idx(idx: toy_hir::ExprIdx, offset: usize) -> toy_hir::ExprIdx {
    la_arena::Idx::from_raw(la_arena::RawIdx::from_u32(
        (idx.into_raw().into_u32() as usize + offset) as u32,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use toy_ast::AstNode;

    /// Helper to infer types from source code using the full pipeline
    fn infer_from_source(source: &str) -> InferenceResult {
        let (syntax, _errors) = toy_parser::parse(source);
        let root = toy_ast::Root::cast(&syntax).expect("failed to cast to Root");
        let hir = toy_hir::lower(root);
        infer(&hir)
    }

    /// Helper to get HIR from source for incremental tests
    fn hir_from_source(source: &str) -> toy_hir::LowerResult {
        let (syntax, _errors) = toy_parser::parse(source);
        let root = toy_ast::Root::cast(&syntax).expect("failed to cast to Root");
        toy_hir::lower(root)
    }

    #[test]
    fn test_variable_integer() {
        let result = infer_from_source("x := 42");
        assert_eq!(result.get_variable_type("x"), Some(&Type::Integer));
        assert!(!result.has_errors());
    }

    #[test]
    fn test_variable_float() {
        let result = infer_from_source("y := 3.5");
        assert_eq!(result.get_variable_type("y"), Some(&Type::Float));
    }

    #[test]
    fn test_variable_boolean() {
        let result = infer_from_source("flag := true");
        assert_eq!(result.get_variable_type("flag"), Some(&Type::Boolean));
    }

    #[test]
    fn test_variable_string() {
        let result = infer_from_source("s := \"hello\"");
        assert_eq!(result.get_variable_type("s"), Some(&Type::String));
    }

    #[test]
    fn test_undefined_variable() {
        let result = infer_from_source("unknown = 1");
        assert!(result.has_errors());
    }

    #[test]
    fn test_incremental_inference() {
        // First command: x := 1
        let hir1 = hir_from_source("x := 42");
        let infer1 = infer_with_state(&hir1, InferState::new());
        assert_eq!(infer1.result.get_variable_type("x"), Some(&Type::Integer));

        // Second command: x = 10 (should work because x is in state)
        let hir2 = hir_from_source("x = 10");
        let infer2 = infer_with_state(&hir2, infer1.state);
        assert!(
            !infer2.result.has_errors(),
            "assignment to existing var should work"
        );
    }

    #[test]
    fn test_incremental_inference_undefined() {
        // First command: assign to undefined y (should error)
        let hir1 = hir_from_source("y = 1");
        let infer1 = infer_with_state(&hir1, InferState::new());
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
