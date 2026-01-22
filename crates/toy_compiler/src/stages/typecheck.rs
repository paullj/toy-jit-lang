//! Typecheck stage: Run type inference on HIR items.
//!
//! This stage takes lowered HIR items and performs Hindley-Milner type inference.
//! Results include expression types, variable types, and type diagnostics.

use std::sync::Arc;

use la_arena::ArenaMap;
use salsa::Accumulator;

use super::ast::AstFile;
use super::hir::{LowerItemResult, lower_ast_item};
use crate::db::Db;
use crate::error::CompilerError;
use crate::stages::Diagnostics;

pub use toy_typecheck::{InferenceResult, Type, TypedModule};

/// Result of typechecking a file.
///
/// Contains type information for all expressions and variables in the file.
#[derive(Debug, Clone, PartialEq)]
pub struct TypecheckResult {
    /// Types for all expressions, keyed by expression index within each item.
    /// Maps (item_index, expr_idx) to Type.
    pub expression_types: Vec<ArenaMap<toy_hir::ExprIdx, Type>>,
    /// Types for all named variables/functions.
    pub variable_types: std::collections::HashMap<String, Type>,
    /// Whether there were type errors.
    pub has_errors: bool,
}

/// Typecheck all HIR items in a file.
///
/// Takes an AstFile and runs type inference on all items together,
/// which is necessary for cross-item references (e.g., function calls).
/// Diagnostics are accumulated via salsa accumulator.
#[salsa::tracked]
pub fn typecheck_file<'db>(db: &'db dyn Db, ast_file: AstFile<'db>) -> Arc<TypecheckResult> {
    let interner = db.interner();
    let ast_items = ast_file.items(db);

    // Lower all AST items to HIR
    let hir_items: Vec<Arc<LowerItemResult>> = ast_items
        .iter()
        .filter_map(|item| lower_ast_item(db, *item))
        .collect();

    if hir_items.is_empty() {
        return Arc::new(TypecheckResult {
            expression_types: vec![],
            variable_types: std::collections::HashMap::new(),
            has_errors: false,
        });
    }

    // Build a combined view for type inference
    // We need to merge the items into a LowerResult-like structure
    let combined = build_combined_hir(&hir_items);

    // Run type inference with the shared interner
    let infer_result = toy_typecheck::infer_with_interner(&combined, interner);

    // Accumulate diagnostics
    for diag in &infer_result.diagnostics {
        Diagnostics(CompilerError::Typecheck(diag.clone())).accumulate(db);
    }

    // Split expression types back per-item
    let expression_types = split_expression_types(&hir_items, &infer_result);
    let has_errors = infer_result.has_errors();

    Arc::new(TypecheckResult {
        expression_types,
        variable_types: infer_result.variable_types,
        has_errors,
    })
}

/// Build a combined LowerResult from multiple LowerItemResults.
///
/// This merges items into a single structure suitable for type inference.
/// Expression indices are remapped to work in a shared arena.
/// Note: The resulting LowerResult has a dummy interner; use `infer_with_interner`
/// to provide the actual interner.
fn build_combined_hir(items: &[Arc<LowerItemResult>]) -> toy_hir::LowerResult {
    use la_arena::Arena;
    use toy_hir::{Expression, LowerResult, SymbolTable};

    let mut combined_items = Vec::new();
    let mut combined_expressions: Arena<Expression> = Arena::new();
    let mut combined_expr_spans = ArenaMap::new();
    let mut combined_item_spans = Vec::new();
    let mut combined_symbols = SymbolTable::default();

    // Track the offset for each item's expressions in the combined arena
    let mut _expr_offsets = Vec::new();

    for item_result in items {
        let offset = combined_expressions.len();
        _expr_offsets.push(offset);

        // Remap the item to use combined arena indices
        let remapped_item = remap_item(&item_result.item, offset);
        combined_items.push(remapped_item);
        combined_item_spans.push(item_result.item_span);

        // Copy expressions with remapped indices
        for (old_idx, expr) in item_result.expressions.iter() {
            let remapped_expr = remap_expression(expr, offset);
            let new_idx = combined_expressions.alloc(remapped_expr);

            // Copy span
            if let Some(span) = item_result.expr_spans.get(old_idx) {
                combined_expr_spans.insert(new_idx, *span);
            }

            // Verify index mapping is correct (indices should be sequential)
            debug_assert_eq!(
                new_idx.into_raw().into_u32() as usize,
                old_idx.into_raw().into_u32() as usize + offset
            );
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
        diagnostics: vec![], // HIR diagnostics already accumulated
        interner: lasso::ThreadedRodeo::default(), // Dummy; use infer_with_interner
    }
}

/// Remap expression indices in an item by adding an offset.
fn remap_item(item: &toy_hir::Item, offset: usize) -> toy_hir::Item {
    use toy_hir::{Definition, Item};

    match item {
        Item::Definition(Definition::Variable { name, value }) => {
            Item::Definition(Definition::Variable {
                name: *name,
                value: remap_expression(value, offset),
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
                    default: p.default.map(|idx| remap_idx(idx, offset)),
                })
                .collect();
            Item::Definition(Definition::Function {
                name: *name,
                params: remapped_params,
                return_type: *return_type,
                body: remap_idx(*body, offset),
            })
        }
        Item::Assignment { name, value } => Item::Assignment {
            name: *name,
            value: remap_expression(value, offset),
        },
        Item::IndexAssignment {
            collection,
            index,
            value,
        } => Item::IndexAssignment {
            collection: remap_idx(*collection, offset),
            index: remap_idx(*index, offset),
            value: remap_expression(value, offset),
        },
        Item::Expression(expr) => Item::Expression(remap_expression(expr, offset)),
    }
}

/// Remap expression indices in an expression by adding an offset.
fn remap_expression(expr: &toy_hir::Expression, offset: usize) -> toy_hir::Expression {
    use toy_hir::Expression;

    match expr {
        Expression::Missing => Expression::Missing,
        Expression::Literal(lit) => Expression::Literal(lit.clone()),
        Expression::Infix { op, lhs, rhs } => Expression::Infix {
            op: *op,
            lhs: remap_idx(*lhs, offset),
            rhs: remap_idx(*rhs, offset),
        },
        Expression::Prefix { op, expr } => Expression::Prefix {
            op: *op,
            expr: remap_idx(*expr, offset),
        },
        Expression::VariableRef { name } => Expression::VariableRef { name: *name },
        Expression::Block { items, tail } => Expression::Block {
            items: items.iter().map(|i| remap_block_item(i, offset)).collect(),
            tail: tail.map(|idx| remap_idx(idx, offset)),
        },
        Expression::If {
            condition,
            then_branch,
            else_branch,
        } => Expression::If {
            condition: remap_idx(*condition, offset),
            then_branch: remap_idx(*then_branch, offset),
            else_branch: else_branch.map(|idx| remap_idx(idx, offset)),
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
                    default: p.default.map(|idx| remap_idx(idx, offset)),
                })
                .collect();
            Expression::Function {
                params: remapped_params,
                return_type: *return_type,
                body: remap_idx(*body, offset),
                captures: captures.clone(),
            }
        }
        Expression::Call { callee, args } => Expression::Call {
            callee: remap_idx(*callee, offset),
            args: args.iter().map(|idx| remap_idx(*idx, offset)).collect(),
        },
        Expression::Return { value } => Expression::Return {
            value: value.map(|idx| remap_idx(idx, offset)),
        },
        Expression::Echo { value } => Expression::Echo {
            value: remap_idx(*value, offset),
        },
        Expression::Loop { label, body } => Expression::Loop {
            label: *label,
            body: remap_idx(*body, offset),
        },
        Expression::While {
            condition,
            label,
            body,
        } => Expression::While {
            condition: remap_idx(*condition, offset),
            label: *label,
            body: remap_idx(*body, offset),
        },
        Expression::For {
            binding,
            iterable,
            label,
            body,
        } => Expression::For {
            binding: *binding,
            iterable: remap_idx(*iterable, offset),
            label: *label,
            body: remap_idx(*body, offset),
        },
        Expression::Range { start, end } => Expression::Range {
            start: remap_idx(*start, offset),
            end: remap_idx(*end, offset),
        },
        Expression::Break { label } => Expression::Break { label: *label },
        Expression::Continue { label } => Expression::Continue { label: *label },
        Expression::List { elements } => Expression::List {
            elements: elements.iter().map(|idx| remap_idx(*idx, offset)).collect(),
        },
        Expression::Index { collection, index } => Expression::Index {
            collection: remap_idx(*collection, offset),
            index: remap_idx(*index, offset),
        },
        Expression::Slice {
            collection,
            start,
            end,
        } => Expression::Slice {
            collection: remap_idx(*collection, offset),
            start: start.map(|idx| remap_idx(idx, offset)),
            end: end.map(|idx| remap_idx(idx, offset)),
        },
        Expression::Tuple { elements } => Expression::Tuple {
            elements: elements.iter().map(|idx| remap_idx(*idx, offset)).collect(),
        },
        Expression::TupleAccess { tuple, index } => Expression::TupleAccess {
            tuple: remap_idx(*tuple, offset),
            index: *index,
        },
    }
}

/// Remap block item indices.
fn remap_block_item(item: &toy_hir::BlockItem, offset: usize) -> toy_hir::BlockItem {
    use toy_hir::BlockItem;

    match item {
        BlockItem::Definition { name, value } => BlockItem::Definition {
            name: *name,
            value: remap_idx(*value, offset),
        },
        BlockItem::Assignment { name, value } => BlockItem::Assignment {
            name: *name,
            value: remap_idx(*value, offset),
        },
        BlockItem::IndexAssignment {
            collection,
            index,
            value,
        } => BlockItem::IndexAssignment {
            collection: remap_idx(*collection, offset),
            index: remap_idx(*index, offset),
            value: remap_idx(*value, offset),
        },
        BlockItem::Expression(idx) => BlockItem::Expression(remap_idx(*idx, offset)),
        BlockItem::Return { value } => BlockItem::Return {
            value: value.map(|idx| remap_idx(idx, offset)),
        },
        BlockItem::Echo { value } => BlockItem::Echo {
            value: remap_idx(*value, offset),
        },
        BlockItem::Break { label } => BlockItem::Break { label: *label },
        BlockItem::Continue { label } => BlockItem::Continue { label: *label },
    }
}

/// Remap an expression index by adding an offset.
fn remap_idx(idx: toy_hir::ExprIdx, offset: usize) -> toy_hir::ExprIdx {
    la_arena::Idx::from_raw(la_arena::RawIdx::from_u32(
        (idx.into_raw().into_u32() as usize + offset) as u32,
    ))
}

/// Split combined expression types back to per-item maps.
fn split_expression_types(
    items: &[Arc<LowerItemResult>],
    infer_result: &InferenceResult,
) -> Vec<ArenaMap<toy_hir::ExprIdx, Type>> {
    let mut result = Vec::with_capacity(items.len());
    let mut offset = 0usize;

    for item in items {
        let item_expr_count = item.expressions.len();
        let mut item_types = ArenaMap::new();

        for (old_idx, _) in item.expressions.iter() {
            let combined_idx = remap_idx(old_idx, offset);
            if let Some(ty) = infer_result.expression_types.get(combined_idx) {
                item_types.insert(old_idx, ty.clone());
            }
        }

        result.push(item_types);
        offset += item_expr_count;
    }

    result
}

/// Typecheck a single module and return a TypedModule.
///
/// This function is designed to be called in parallel for independent modules.
/// It takes HIR items for a single module and produces complete type information.
#[salsa::tracked]
pub fn typecheck_module<'db>(
    db: &'db dyn Db,
    module_id: toy_hir::ModuleId,
    ast_file: AstFile<'db>,
) -> Arc<TypedModule> {
    let interner = db.interner();
    let ast_items = ast_file.items(db);

    // Lower all AST items to HIR
    let hir_items: Vec<Arc<LowerItemResult>> = ast_items
        .iter()
        .filter_map(|item| lower_ast_item(db, *item))
        .collect();

    if hir_items.is_empty() {
        return Arc::new(TypedModule::new(module_id));
    }

    // Use the new module-level inference
    let typed_module = toy_typecheck::infer_module(module_id, &hir_items, interner);

    // Accumulate diagnostics
    if typed_module.has_errors {
        // TODO: Accumulate specific diagnostics when available
        Diagnostics(CompilerError::Typecheck(
            toy_typecheck::InferDiagnostic::GenericError {
                message: "Type errors detected in module".to_string(),
                span: None,
            },
        ))
        .accumulate(db);
    }

    Arc::new(typed_module)
}

/// Typecheck a strongly connected component of modules.
///
/// When modules have circular dependencies (mutually recursive), they must be
/// type-checked together to properly handle cross-module references.
pub fn typecheck_scc(
    db: &dyn Db,
    module_ids: &[toy_hir::ModuleId],
    ast_files: &[AstFile],
) -> Vec<Arc<TypedModule>> {
    let _interner = db.interner();
    let mut all_typed_modules = Vec::with_capacity(module_ids.len());

    // For SCCs, we need to type check all modules together
    // This is more complex and would require combining HIR from multiple modules
    // For now, we'll type check them individually but in sequence
    for (module_id, ast_file) in module_ids.iter().zip(ast_files.iter()) {
        let typed_module = typecheck_module(db, *module_id, *ast_file);
        all_typed_modules.push(typed_module);
    }

    // TODO: Implement proper SCC type checking that handles mutual recursion
    // This would involve:
    // 1. Combining HIR from all modules in the SCC
    // 2. Running type inference on the combined HIR
    // 3. Splitting results back to individual modules

    all_typed_modules
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use crate::stages::ast::ast_file;
    use crate::stages::parse::parse_text;
    use crate::stages::read::{SourceText, source_to_file_text};

    fn typecheck_source(db: &Database, source: &str) -> Arc<TypecheckResult> {
        let source_text = SourceText::new(db, source.to_string());
        let file_text = source_to_file_text(db, source_text);
        let parsed = parse_text(db, file_text);
        let ast = ast_file(db, parsed);
        typecheck_file(db, ast)
    }

    #[test]
    fn typecheck_empty_source() {
        let db = Database::default();
        let result = typecheck_source(&db, "");
        assert!(!result.has_errors);
        assert!(result.variable_types.is_empty());
    }

    #[test]
    fn typecheck_variable_integer() {
        let db = Database::default();
        let result = typecheck_source(&db, "x := 42");
        assert!(!result.has_errors);
        assert_eq!(result.variable_types.get("x"), Some(&Type::Integer));
    }

    #[test]
    fn typecheck_variable_float() {
        let db = Database::default();
        let result = typecheck_source(&db, "y := 3.5");
        assert!(!result.has_errors);
        assert_eq!(result.variable_types.get("y"), Some(&Type::Float));
    }

    #[test]
    fn typecheck_variable_boolean() {
        let db = Database::default();
        let result = typecheck_source(&db, "flag := true");
        assert!(!result.has_errors);
        assert_eq!(result.variable_types.get("flag"), Some(&Type::Boolean));
    }

    #[test]
    fn typecheck_variable_string() {
        let db = Database::default();
        let result = typecheck_source(&db, "s := \"hello\"");
        assert!(!result.has_errors);
        assert_eq!(result.variable_types.get("s"), Some(&Type::String));
    }

    #[test]
    fn typecheck_undefined_variable() {
        let db = Database::default();
        let result = typecheck_source(&db, "unknown = 1");
        assert!(result.has_errors);
    }

    #[test]
    fn typecheck_function_simple() {
        let db = Database::default();
        let result = typecheck_source(&db, "fn add(a, b) { a + b }");
        assert!(!result.has_errors);

        let fn_ty = result.variable_types.get("add").expect("add should exist");
        assert!(matches!(fn_ty, Type::Function { params, ret }
                if params.len() == 2 && **ret == Type::Integer));
    }

    #[test]
    fn typecheck_function_call() {
        let db = Database::default();
        let result = typecheck_source(
            &db,
            "fn double(x) { x + x }
             double(5)",
        );
        assert!(!result.has_errors);
    }

    #[test]
    fn typecheck_cross_item_reference() {
        let db = Database::default();
        let result = typecheck_source(
            &db,
            "x := 42
             y := x + 1",
        );
        assert!(!result.has_errors);
        assert_eq!(result.variable_types.get("x"), Some(&Type::Integer));
        assert_eq!(result.variable_types.get("y"), Some(&Type::Integer));
    }

    #[test]
    fn typecheck_recursive_function() {
        let db = Database::default();
        let result = typecheck_source(
            &db,
            "fn fac(n) {
                if n <= 1 { 1 }
                else { n * fac(n - 1) }
             }",
        );
        assert!(!result.has_errors);

        let fn_ty = result.variable_types.get("fac").expect("fac should exist");
        assert!(matches!(fn_ty, Type::Function { params, ret }
                if params == &[Type::Integer] && **ret == Type::Integer));
    }

    #[test]
    fn typecheck_list() {
        let db = Database::default();
        let result = typecheck_source(&db, "xs := [1, 2, 3]");
        assert!(!result.has_errors);

        let list_ty = result.variable_types.get("xs").expect("xs should exist");
        assert!(matches!(list_ty, Type::List(elem) if **elem == Type::Integer));
    }

    #[test]
    fn typecheck_tuple() {
        let db = Database::default();
        let result = typecheck_source(&db, "t := (1, true, \"hi\")");
        assert!(!result.has_errors);

        let tuple_ty = result.variable_types.get("t").expect("t should exist");
        assert!(matches!(tuple_ty, Type::Tuple(elems)
                if elems == &[Type::Integer, Type::Boolean, Type::String]));
    }

    #[test]
    fn typecheck_arity_mismatch() {
        let db = Database::default();
        let result = typecheck_source(
            &db,
            "fn add(a, b) { a + b }
             add(1)",
        );
        assert!(result.has_errors);
    }

    #[test]
    fn typecheck_return_type_mismatch() {
        let db = Database::default();
        let result = typecheck_source(&db, "fn foo(): int { true }");
        assert!(result.has_errors);
    }

    // ========================================
    // Memoization tests using TrackedDatabase
    // ========================================

    mod memoization {
        use super::*;
        use crate::db::test_utils::TrackedDatabase;

        #[test]
        fn same_input_no_reexecution() {
            let db = TrackedDatabase::new();

            let source = SourceText::new(&db, "x := 42".to_string());
            let file_text = source_to_file_text(&db, source);
            let parsed = parse_text(&db, file_text);
            let ast = ast_file(&db, parsed);
            let _result = typecheck_file(&db, ast);

            let exec_after_first = db.counters.will_execute();
            assert!(exec_after_first > 0);

            db.counters.reset();

            // Same input - should be memoized
            let _result2 = typecheck_file(&db, ast);

            assert_eq!(
                db.counters.will_execute(),
                0,
                "should not re-execute with same input"
            );
        }

        #[test]
        fn different_source_reexecutes() {
            let db = TrackedDatabase::new();

            // First source
            let source1 = SourceText::new(&db, "x := 1".to_string());
            let file_text1 = source_to_file_text(&db, source1);
            let parsed1 = parse_text(&db, file_text1);
            let ast1 = ast_file(&db, parsed1);
            let _result1 = typecheck_file(&db, ast1);

            db.counters.reset();

            // Different source - should re-execute
            let source2 = SourceText::new(&db, "y := 2".to_string());
            let file_text2 = source_to_file_text(&db, source2);
            let parsed2 = parse_text(&db, file_text2);
            let ast2 = ast_file(&db, parsed2);
            let _result2 = typecheck_file(&db, ast2);

            assert!(
                db.counters.will_execute() > 0,
                "should re-execute for different source"
            );
        }

        #[test]
        fn full_pipeline_memoized() {
            let db = TrackedDatabase::new();

            let source = SourceText::new(&db, "fn foo() { 42 }".to_string());
            let file_text = source_to_file_text(&db, source);
            let parsed = parse_text(&db, file_text);
            let ast = ast_file(&db, parsed);
            let _result = typecheck_file(&db, ast);

            let first_exec_count = db.counters.will_execute();
            assert!(first_exec_count > 0);

            db.counters.reset();

            // Run entire pipeline again with same source input
            let file_text2 = source_to_file_text(&db, source);
            let parsed2 = parse_text(&db, file_text2);
            let ast2 = ast_file(&db, parsed2);
            let _result2 = typecheck_file(&db, ast2);

            assert_eq!(
                db.counters.will_execute(),
                0,
                "full pipeline should be memoized"
            );
        }

        #[test]
        fn new_input_with_same_content_reexecutes() {
            let db = TrackedDatabase::new();

            // First input
            let source1 = SourceText::new(&db, "x := 42".to_string());
            let file_text1 = source_to_file_text(&db, source1);
            let parsed1 = parse_text(&db, file_text1);
            let ast1 = ast_file(&db, parsed1);
            let _result1 = typecheck_file(&db, ast1);

            db.counters.reset();

            // New input with SAME content - different ID, should re-execute
            let source2 = SourceText::new(&db, "x := 42".to_string());
            let file_text2 = source_to_file_text(&db, source2);
            let parsed2 = parse_text(&db, file_text2);
            let ast2 = ast_file(&db, parsed2);
            let _result2 = typecheck_file(&db, ast2);

            assert!(
                db.counters.will_execute() > 0,
                "new input with same content should re-execute (identity-based)"
            );
        }

        #[test]
        fn typecheck_with_errors_also_memoized() {
            let db = TrackedDatabase::new();

            // Source with type error
            let source = SourceText::new(&db, "fn foo(): int { true }".to_string());
            let file_text = source_to_file_text(&db, source);
            let parsed = parse_text(&db, file_text);
            let ast = ast_file(&db, parsed);
            let result1 = typecheck_file(&db, ast);

            assert!(result1.has_errors);

            db.counters.reset();

            // Same input - should be memoized even with errors
            let result2 = typecheck_file(&db, ast);

            assert_eq!(
                db.counters.will_execute(),
                0,
                "typecheck with errors should also be memoized"
            );
            assert!(result2.has_errors);
        }
    }
}
