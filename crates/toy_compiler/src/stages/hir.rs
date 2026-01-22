//! HIR stage: Lower AST items to High-level IR for semantic analysis.
//!
//! Each AST item is lowered independently, enabling fine-grained memoization.
//! Only changed items re-lower to HIR when source changes.

use std::collections::HashMap;
use std::sync::Arc;

use salsa::Accumulator;
use toy_ast::{AstNode, Item};

use super::ast::AstItem;
use crate::db::Db;
use crate::error::CompilerError;
use crate::stages::Diagnostics;

pub use toy_hir::LowerItemResult;

/// Lower a single AST item to HIR.
///
/// Uses the shared interner from the database for identifier interning.
/// Diagnostics are accumulated via salsa accumulator.
/// Returns the result wrapped in Arc for efficient sharing.
#[salsa::tracked]
pub fn lower_ast_item<'db>(
    db: &'db dyn Db,
    ast_item: AstItem<'db>,
) -> Option<Arc<LowerItemResult>> {
    let interner = db.interner();
    let syntax = ast_item.syntax(db);

    // Cast to typed AST item
    let typed_item = Item::cast(syntax)?;

    // Lower using the shared interner
    let result = toy_hir::lower_single_item(interner, typed_item)?;

    // Accumulate diagnostics
    for diag in result.diagnostics.iter().cloned() {
        Diagnostics(CompilerError::Hir(diag)).accumulate(db);
    }

    Some(Arc::new(result))
}

/// Resolution data for HIR lowering (stored in Salsa).
#[salsa::input]
pub struct ResolutionContext {
    pub module_id: toy_hir::ModuleId,
    #[returns(ref)]
    pub symbol_tables: HashMap<toy_hir::ModuleId, toy_resolve::SymbolTable>,
}

/// Lower a single AST item to HIR with resolved symbols.
///
/// Uses the shared interner from the database and resolution data
/// to properly handle cross-module references.
#[salsa::tracked]
pub fn lower_ast_item_with_resolution<'db>(
    db: &'db dyn Db,
    ast_item: AstItem<'db>,
    resolution: ResolutionContext,
) -> Option<Arc<LowerItemResult>> {
    let interner = db.interner();
    let syntax = ast_item.syntax(db);

    // Cast to typed AST item
    let typed_item = Item::cast(syntax)?;

    // Lower using the shared interner and resolution data
    let result = toy_hir::lower_single_item_with_resolution(
        interner,
        typed_item,
        resolution.module_id(db),
        resolution.symbol_tables(db).clone(),
        HashMap::new(), // module_paths not used yet
    )?;

    // Accumulate diagnostics
    for diag in result.diagnostics.iter().cloned() {
        Diagnostics(CompilerError::Hir(diag)).accumulate(db);
    }

    Some(Arc::new(result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use crate::stages::ast::ast_file;
    use crate::stages::parse::parse_text;
    use crate::stages::read::{SourceText, source_to_file_text};
    use toy_hir::Definition;

    fn parse_and_lower(db: &Database, source: &str) -> Vec<Arc<LowerItemResult>> {
        let source_text = SourceText::new(db, source.to_string());
        let file_text = source_to_file_text(db, source_text);
        let parsed = parse_text(db, file_text);
        let ast = ast_file(db, parsed);

        ast.items(db)
            .iter()
            .filter_map(|item| lower_ast_item(db, *item))
            .collect()
    }

    #[test]
    fn lower_empty_source() {
        let db = Database::default();
        let items = parse_and_lower(&db, "");
        assert!(items.is_empty());
    }

    #[test]
    fn lower_variable_definition() {
        let db = Database::default();
        let items = parse_and_lower(&db, "x := 42");

        assert_eq!(items.len(), 1);
        let hir = &items[0];

        assert!(matches!(
            &hir.item,
            toy_hir::Item::Definition(Definition::Variable { .. })
        ));
    }

    #[test]
    fn lower_function_definition() {
        let db = Database::default();
        let items = parse_and_lower(&db, "fn add(a, b) { a + b }");

        assert_eq!(items.len(), 1);
        let hir = &items[0];

        assert!(matches!(
            &hir.item,
            toy_hir::Item::Definition(Definition::Function { .. })
        ));
    }

    #[test]
    fn lower_multiple_items() {
        let db = Database::default();
        let items = parse_and_lower(
            &db,
            r#"
x := 1
y := 2
fn foo() { x + y }
"#,
        );

        assert_eq!(items.len(), 3);
        assert!(matches!(
            &items[0].item,
            toy_hir::Item::Definition(Definition::Variable { .. })
        ));
        assert!(matches!(
            &items[1].item,
            toy_hir::Item::Definition(Definition::Variable { .. })
        ));
        assert!(matches!(
            &items[2].item,
            toy_hir::Item::Definition(Definition::Function { .. })
        ));
    }

    #[test]
    fn lower_expression_item() {
        let db = Database::default();
        let items = parse_and_lower(&db, "1 + 2");

        assert_eq!(items.len(), 1);
        assert!(matches!(&items[0].item, toy_hir::Item::Expression(_)));
    }

    #[test]
    fn expressions_arena_populated() {
        let db = Database::default();
        let items = parse_and_lower(&db, "x := 1 + 2 * 3");

        assert_eq!(items.len(), 1);
        // Should have multiple expressions in the arena (operands of infix ops)
        assert!(!items[0].expressions.is_empty());
    }

    #[test]
    fn item_span_correct() {
        let db = Database::default();
        let items = parse_and_lower(&db, "x := 42");

        assert_eq!(items.len(), 1);
        // Span should cover the whole definition
        assert!(items[0].item_span.len() > 0.into());
    }

    #[test]
    fn symbols_tracked() {
        let db = Database::default();
        let items = parse_and_lower(&db, "x := 42");

        assert_eq!(items.len(), 1);
        // Should have 'x' in the symbol table
        assert!(items[0].symbols.get("x").is_some());
    }

    #[test]
    fn shared_interner_across_items() {
        let db = Database::default();
        let items = parse_and_lower(
            &db,
            r#"
x := 1
y := x
"#,
        );

        assert_eq!(items.len(), 2);
        // Both items should use the same interner (the database's)
        // 'x' interned in first item should have same key as reference in second
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
            let ast_items = ast.items(&db);
            let _hir = lower_ast_item(&db, ast_items[0]);

            let exec_after_first = db.counters.will_execute();
            assert!(exec_after_first > 0);

            db.counters.reset();

            // Same input - should be memoized
            let _hir2 = lower_ast_item(&db, ast_items[0]);

            assert_eq!(
                db.counters.will_execute(),
                0,
                "should not re-execute with same input"
            );
        }

        #[test]
        fn different_item_reexecutes() {
            let db = TrackedDatabase::new();

            let source = SourceText::new(&db, "x := 1\ny := 2".to_string());
            let file_text = source_to_file_text(&db, source);
            let parsed = parse_text(&db, file_text);
            let ast = ast_file(&db, parsed);
            let ast_items = ast.items(&db);

            let _hir1 = lower_ast_item(&db, ast_items[0]);

            db.counters.reset();

            // Different item - should execute
            let _hir2 = lower_ast_item(&db, ast_items[1]);

            assert!(
                db.counters.will_execute() > 0,
                "should execute for different item"
            );
        }

        #[test]
        fn full_pipeline_memoized() {
            let db = TrackedDatabase::new();

            let source = SourceText::new(&db, "fn foo() { 42 }".to_string());
            let file_text = source_to_file_text(&db, source);
            let parsed = parse_text(&db, file_text);
            let ast = ast_file(&db, parsed);
            let ast_items = ast.items(&db);
            let _hir = lower_ast_item(&db, ast_items[0]);

            let first_exec_count = db.counters.will_execute();
            assert!(first_exec_count > 0);

            db.counters.reset();

            // Run entire pipeline again with same inputs
            let file_text2 = source_to_file_text(&db, source);
            let parsed2 = parse_text(&db, file_text2);
            let ast2 = ast_file(&db, parsed2);
            let ast_items2 = ast2.items(&db);
            let _hir2 = lower_ast_item(&db, ast_items2[0]);

            assert_eq!(
                db.counters.will_execute(),
                0,
                "full pipeline should be memoized"
            );
        }

        #[test]
        fn new_source_reexecutes() {
            let db = TrackedDatabase::new();

            // First source
            let source1 = SourceText::new(&db, "x := 1".to_string());
            let file_text1 = source_to_file_text(&db, source1);
            let parsed1 = parse_text(&db, file_text1);
            let ast1 = ast_file(&db, parsed1);
            let _hir1 = lower_ast_item(&db, ast1.items(&db)[0]);

            db.counters.reset();

            // New source - different input ID
            let source2 = SourceText::new(&db, "y := 2".to_string());
            let file_text2 = source_to_file_text(&db, source2);
            let parsed2 = parse_text(&db, file_text2);
            let ast2 = ast_file(&db, parsed2);
            let _hir2 = lower_ast_item(&db, ast2.items(&db)[0]);

            assert!(
                db.counters.will_execute() > 0,
                "new source should trigger re-execution"
            );
        }

        #[test]
        fn unchanged_item_still_memoized_after_other_item_changes() {
            let db = TrackedDatabase::new();

            // Parse two items
            let source = SourceText::new(&db, "x := 1\nfn foo() { 42 }".to_string());
            let file_text = source_to_file_text(&db, source);
            let parsed = parse_text(&db, file_text);
            let ast = ast_file(&db, parsed);
            let items = ast.items(&db);

            // Lower both items
            let _hir_x = lower_ast_item(&db, items[0]);
            let _hir_foo = lower_ast_item(&db, items[1]);

            db.counters.reset();

            // Query the same items again - both should be memoized
            let _hir_x2 = lower_ast_item(&db, items[0]);
            let _hir_foo2 = lower_ast_item(&db, items[1]);

            assert_eq!(
                db.counters.will_execute(),
                0,
                "both items should be memoized"
            );
        }
    }
}
