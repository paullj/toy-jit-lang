//! MIR stage: Lower typed HIR to Mid-level IR.
//!
//! This stage takes typechecked HIR and produces MIR (Module with Functions).
//! MIR is file-level because cross-function references need consistent FuncIds.

use std::sync::Arc;

use super::ast::AstFile;
use super::typecheck::{TypedModule, typecheck_module};
use crate::db::Db;

/// Result of MIR lowering for a file.
#[derive(Debug, Clone, PartialEq)]
pub struct MirResult {
    /// The lowered MIR module containing all functions.
    pub module: toy_mir::Module,
    /// The module ID this MIR belongs to.
    pub module_id: toy_hir::ModuleId,
}

/// Lower an entire file to MIR.
///
/// Takes an AstFile, runs typecheck (which builds combined HIR),
/// then lowers to MIR. Results are memoized at the file level.
#[salsa::tracked]
pub fn lower_to_mir<'db>(db: &'db dyn Db, ast_file: AstFile<'db>) -> Arc<MirResult> {
    // Get typed module from type checking
    let module_id = toy_hir::ModuleId(0); // Single file for now
    let typed_module = typecheck_module(db, module_id, ast_file);

    if typed_module.items.is_empty() {
        return Arc::new(MirResult {
            module: toy_mir::Module {
                functions: vec![],
                main_id: toy_mir::FuncId(0),
            },
            module_id,
        });
    }

    // Lower to MIR using the typed module directly
    let interner = db.interner();
    let module = toy_mir::lower_typed_module(&typed_module, interner);

    Arc::new(MirResult { module, module_id })
}

/// Lower a typed module directly to MIR.
///
/// This function is designed for use in the parallel compilation pipeline
/// where modules have already been type-checked.
#[allow(dead_code)]
pub fn lower_typed_module_to_mir(
    db: &dyn Db,
    module_id: toy_hir::ModuleId,
    typed_module: Arc<TypedModule>,
) -> Arc<MirResult> {
    // Lower to MIR using the typed module
    let interner = db.interner();
    let module = toy_mir::lower_typed_module(&typed_module, interner);

    Arc::new(MirResult { module, module_id })
}

/// Lower a module to MIR in the parallel compilation pipeline.
///
/// This is a non-Salsa function for use with multiple modules in parallel.
/// Takes a typed module and produces MIR.
pub fn lower_module(
    db: &dyn Db,
    module_id: toy_hir::ModuleId,
    typed_module: Arc<TypedModule>,
) -> Arc<MirResult> {
    log::trace!("Lowering module {:?} to MIR", module_id);

    // If the typed module is empty, return empty MIR
    // Note: We still try to lower even with type errors for debugging
    if typed_module.items.is_empty() {
        log::debug!("Module {:?} has no items, returning empty MIR", module_id);
        return Arc::new(MirResult {
            module: toy_mir::Module {
                functions: vec![],
                main_id: toy_mir::FuncId(0),
            },
            module_id,
        });
    }

    if typed_module.has_errors {
        log::debug!(
            "Module {:?} has type errors but continuing with MIR lowering",
            module_id
        );
    }

    // Lower to MIR using the typed module
    let interner = db.interner();
    let module = toy_mir::lower_typed_module(&typed_module, interner);

    Arc::new(MirResult { module, module_id })
}

/// Lower a strongly connected component (SCC) of modules to MIR.
///
/// This handles mutually recursive modules that need to be lowered together.
/// For now, this sequences individual module lowering, but could be enhanced
/// to handle cross-module references more efficiently.
pub fn lower_scc(
    db: &dyn Db,
    module_ids: &[toy_hir::ModuleId],
    typed_modules: &[Arc<TypedModule>],
) -> Vec<Arc<MirResult>> {
    log::trace!("Lowering SCC with {} modules to MIR", module_ids.len());

    // For now, lower each module individually in the SCC
    // TODO: Implement proper cross-module reference handling for mutually recursive modules
    module_ids
        .iter()
        .zip(typed_modules.iter())
        .map(|(module_id, typed_module)| lower_module(db, *module_id, Arc::clone(typed_module)))
        .collect()
}

// Note: The helper functions for building combined HIR and inference results
// have been removed as they are no longer needed with the TypedModule approach.
// The TypedModule already contains all necessary type information in a clean format.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use crate::stages::ast::ast_file;
    use crate::stages::parse::parse_text;
    use crate::stages::read::{SourceText, source_to_file_text};

    fn lower_source_to_mir(db: &Database, source: &str) -> Arc<MirResult> {
        let source_text = SourceText::new(db, source.to_string());
        let file_text = source_to_file_text(db, source_text);
        let parsed = parse_text(db, file_text);
        let ast = ast_file(db, parsed);
        lower_to_mir(db, ast)
    }

    #[test]
    fn mir_empty_source() {
        let db = Database::default();
        let result = lower_source_to_mir(&db, "");
        assert!(result.module.functions.is_empty());
    }

    #[test]
    fn mir_variable_definition() {
        let db = Database::default();
        let result = lower_source_to_mir(&db, "x := 42");

        // Should have main function
        assert_eq!(result.module.functions.len(), 1);
        let main = result.module.main();
        assert_eq!(main.name.as_deref(), Some("main"));
        assert_eq!(main.local_count, 1); // x
    }

    #[test]
    fn mir_function_definition() {
        let db = Database::default();
        let result = lower_source_to_mir(&db, "fn add(a, b) { a + b }");

        // Should have add + main
        assert_eq!(result.module.functions.len(), 2);

        let add = result
            .module
            .functions
            .iter()
            .find(|f| f.name.as_deref() == Some("add"))
            .expect("add function should exist");
        assert_eq!(add.param_count, 2);
    }

    #[test]
    fn mir_function_call() {
        let db = Database::default();
        let result = lower_source_to_mir(
            &db,
            "fn double(x) { x + x }
             double(5)",
        );

        // double + main
        assert_eq!(result.module.functions.len(), 2);

        // Main should have a direct call to double
        let main = result.module.main();
        let has_call = main
            .blocks
            .iter()
            .flat_map(|b| &b.insts)
            .any(|inst| matches!(inst, toy_mir::Inst::Call { func, .. } if func.0 == 0));
        assert!(has_call, "main should call double");
    }

    #[test]
    fn mir_multiple_functions() {
        let db = Database::default();
        let result = lower_source_to_mir(
            &db,
            "fn foo() { 1 }
             fn bar() { foo() + 1 }
             bar()",
        );

        // foo, bar, main
        assert_eq!(result.module.functions.len(), 3);
    }

    #[test]
    fn mir_control_flow() {
        let db = Database::default();
        let result = lower_source_to_mir(&db, "x := if true { 1 } else { 2 }");

        let main = result.module.main();
        // Should have multiple blocks for if/else
        assert!(main.blocks.len() > 1);

        // Should have branch instruction
        let has_branch = main
            .blocks
            .iter()
            .flat_map(|b| &b.insts)
            .any(|inst| matches!(inst, toy_mir::Inst::Branch { .. }));
        assert!(has_branch);
    }

    #[test]
    fn mir_while_loop() {
        let db = Database::default();
        let result = lower_source_to_mir(
            &db,
            "x := 0
             while x < 10 {
                 x = x + 1
             }",
        );

        let main = result.module.main();
        // Should have multiple blocks for condition/body/exit
        assert!(main.blocks.len() > 1);
    }

    #[test]
    fn mir_list_operations() {
        let db = Database::default();
        let result = lower_source_to_mir(&db, "xs := [1, 2, 3]");

        let main = result.module.main();
        // Should have ListNew instruction
        let has_list_new = main
            .blocks
            .iter()
            .flat_map(|b| &b.insts)
            .any(|inst| matches!(inst, toy_mir::Inst::ListNew { .. }));
        assert!(has_list_new);
    }

    #[test]
    fn mir_closure() {
        let db = Database::default();
        let result = lower_source_to_mir(
            &db,
            "x := 10
             f := fn(y) { x + y }
             f(5)",
        );

        // Should have closure function + main
        assert!(
            result.module.functions.len() >= 2,
            "expected at least 2 functions, got {}",
            result.module.functions.len()
        );

        // Should have MakeClosure instruction
        let main = result.module.main();
        let has_make_closure = main
            .blocks
            .iter()
            .flat_map(|b| &b.insts)
            .any(|inst| matches!(inst, toy_mir::Inst::MakeClosure { .. }));
        assert!(has_make_closure);
    }

    #[test]
    fn mir_tuple() {
        let db = Database::default();
        let result = lower_source_to_mir(&db, "t := (1, true, \"hi\")");

        let main = result.module.main();
        let has_tuple_new = main
            .blocks
            .iter()
            .flat_map(|b| &b.insts)
            .any(|inst| matches!(inst, toy_mir::Inst::TupleNew { .. }));
        assert!(has_tuple_new);
    }

    #[test]
    fn mir_echo() {
        let db = Database::default();
        let result = lower_source_to_mir(&db, "echo 42");

        let main = result.module.main();
        let has_echo = main
            .blocks
            .iter()
            .flat_map(|b| &b.insts)
            .any(|inst| matches!(inst, toy_mir::Inst::Echo { .. }));
        assert!(has_echo);
    }

    #[test]
    fn mir_for_loop() {
        let db = Database::default();
        let result = lower_source_to_mir(
            &db,
            "sum := 0
             for i in 0..10 {
                 sum = sum + i
             }",
        );

        let main = result.module.main();
        // For loop desugars to while, should have multiple blocks
        assert!(main.blocks.len() > 1);
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
            let _result = lower_to_mir(&db, ast);

            let exec_after_first = db.counters.will_execute();
            assert!(exec_after_first > 0);

            db.counters.reset();

            // Same input - should be memoized
            let _result2 = lower_to_mir(&db, ast);

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
            let _result1 = lower_to_mir(&db, ast1);

            db.counters.reset();

            // Different source - should re-execute
            let source2 = SourceText::new(&db, "y := 2".to_string());
            let file_text2 = source_to_file_text(&db, source2);
            let parsed2 = parse_text(&db, file_text2);
            let ast2 = ast_file(&db, parsed2);
            let _result2 = lower_to_mir(&db, ast2);

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
            let _result = lower_to_mir(&db, ast);

            let first_exec_count = db.counters.will_execute();
            assert!(first_exec_count > 0);

            db.counters.reset();

            // Run entire pipeline again with same source input
            let file_text2 = source_to_file_text(&db, source);
            let parsed2 = parse_text(&db, file_text2);
            let ast2 = ast_file(&db, parsed2);
            let _result2 = lower_to_mir(&db, ast2);

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
            let _result1 = lower_to_mir(&db, ast1);

            db.counters.reset();

            // New input with SAME content - different ID, should re-execute
            let source2 = SourceText::new(&db, "x := 42".to_string());
            let file_text2 = source_to_file_text(&db, source2);
            let parsed2 = parse_text(&db, file_text2);
            let ast2 = ast_file(&db, parsed2);
            let _result2 = lower_to_mir(&db, ast2);

            assert!(
                db.counters.will_execute() > 0,
                "new input with same content should re-execute (identity-based)"
            );
        }

        #[test]
        fn mir_with_type_errors_returns_empty() {
            let db = TrackedDatabase::new();

            // Source with undefined variable (type error)
            let source = SourceText::new(&db, "unknown = 1".to_string());
            let file_text = source_to_file_text(&db, source);
            let parsed = parse_text(&db, file_text);
            let ast = ast_file(&db, parsed);
            let result = lower_to_mir(&db, ast);

            // Should still produce some MIR (main function at minimum)
            // The type error is handled by typecheck stage
            assert!(!result.module.functions.is_empty());
        }
    }
}
