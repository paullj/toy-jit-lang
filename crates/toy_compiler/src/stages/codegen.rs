//! Codegen stage: Lower MIR to native code via Cranelift.
//!
//! This stage takes MIR and produces a native object file.
//! Results are memoized at the file level.

use std::sync::Arc;

use super::ast::AstFile;
use super::mir::lower_to_mir;
use crate::db::Db;

/// Result of codegen for a file.
#[derive(Debug, Clone, PartialEq)]
pub struct CodegenResult {
    /// The compiled native object bytes (or None if compilation failed/empty).
    pub object: Option<toy_codegen::CompiledObject>,
    /// Any codegen errors encountered.
    pub error: Option<String>,
    /// Module ID this result belongs to
    pub module_id: toy_hir::ModuleId,
}

/// Compile an entire file to native code.
///
/// Takes an AstFile, runs MIR lowering, then generates native code via Cranelift.
/// Results are memoized at the file level.
#[salsa::tracked]
pub fn codegen<'db>(db: &'db dyn Db, ast_file: AstFile<'db>) -> Arc<CodegenResult> {
    let mir_result = lower_to_mir(db, ast_file);
    let module_id = mir_result.module_id;

    if mir_result.module.functions.is_empty() {
        return Arc::new(CodegenResult {
            object: None,
            error: None,
            module_id,
        });
    }

    match toy_codegen::compile_native(&mir_result.module) {
        Ok(object) => Arc::new(CodegenResult {
            object: Some(object),
            error: None,
            module_id,
        }),
        Err(e) => Arc::new(CodegenResult {
            object: None,
            error: Some(e.to_string()),
            module_id,
        }),
    }
}

/// Compile a single module to native code with external module info.
///
/// This function is designed for parallel compilation where modules
/// need to reference functions from other modules.
pub fn codegen_module(
    module_id: toy_hir::ModuleId,
    mir_module: &toy_mir::Module,
    external_modules: &[(toy_hir::ModuleId, &toy_mir::Module)],
) -> CodegenResult {
    if mir_module.functions.is_empty() {
        return CodegenResult {
            object: None,
            error: None,
            module_id,
        };
    }

    match toy_codegen::compile_module(mir_module, module_id, external_modules) {
        Ok(object) => CodegenResult {
            object: Some(object),
            error: None,
            module_id,
        },
        Err(e) => CodegenResult {
            object: None,
            error: Some(e.to_string()),
            module_id,
        },
    }
}

/// Compile multiple modules in parallel to native code.
///
/// Takes MIR modules and compiles them with cross-module references.
/// Returns a vector of CodegenResults, one per module.
pub fn codegen_modules(
    mir_modules: Vec<(toy_hir::ModuleId, Arc<super::mir::MirResult>)>,
) -> Result<Vec<CodegenResult>, String> {
    use std::sync::mpsc::channel;
    use std::time::Instant;
    use threadpool::ThreadPool;

    let workers = match std::thread::available_parallelism() {
        Ok(n) => n.get(),
        Err(_) => 1,
    };
    let pool = ThreadPool::with_name("toy-codegen-worker".to_string(), workers);
    let (tx, rx) = channel();
    let n_jobs = mir_modules.len();

    // Clone all MIR modules for parallel processing
    let all_modules: Vec<(toy_hir::ModuleId, toy_mir::Module)> = mir_modules
        .iter()
        .map(|(id, mir_result)| (*id, mir_result.module.clone()))
        .collect();

    // Compile each module in parallel
    for (module_id, mir_result) in mir_modules {
        let tx = tx.clone();
        let mir_module = mir_result.module.clone();
        let external_modules: Vec<(toy_hir::ModuleId, toy_mir::Module)> = all_modules
            .iter()
            .filter(|(id, _)| *id != module_id)
            .map(|(id, m)| (*id, m.clone()))
            .collect();

        pool.execute(move || {
            let thread_id = std::thread::current().id();
            let start = Instant::now();

            let external_refs: Vec<(toy_hir::ModuleId, &toy_mir::Module)> =
                external_modules.iter().map(|(id, m)| (*id, m)).collect();

            let result = codegen_module(module_id, &mir_module, &external_refs);

            let elapsed = start.elapsed().as_secs_f64() * 1000.0;
            log::debug!(
                "  Completed codegen for module {} ({:?}) in {:.2}ms",
                module_id.0,
                thread_id,
                elapsed
            );

            // Fail fast on codegen errors
            if result.error.is_some() {
                tx.send(Err(format!(
                    "Codegen failed for module {:?}: {}",
                    module_id,
                    result.error.as_ref().unwrap()
                )))
                .unwrap();
            } else {
                tx.send(Ok(result)).unwrap();
            }
        });
    }

    // Collect results, failing fast on any error
    let mut results = Vec::new();
    for _ in 0..n_jobs {
        match rx.recv().unwrap() {
            Ok(result) => results.push(result),
            Err(e) => return Err(e),
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use crate::stages::ast::ast_file;
    use crate::stages::parse::parse_text;
    use crate::stages::read::{SourceText, source_to_file_text};

    fn compile_source(db: &Database, source: &str) -> Arc<CodegenResult> {
        let source_text = SourceText::new(db, source.to_string());
        let file_text = source_to_file_text(db, source_text);
        let parsed = parse_text(db, file_text);
        let ast = ast_file(db, parsed);
        codegen(db, ast)
    }

    #[test]
    fn codegen_empty_source() {
        let db = Database::default();
        let result = compile_source(&db, "");
        assert!(result.object.is_none());
        assert!(result.error.is_none());
        assert_eq!(result.module_id, toy_hir::ModuleId(0));
    }

    #[test]
    fn codegen_variable_definition() {
        let db = Database::default();
        let result = compile_source(&db, "x := 42");

        assert!(result.object.is_some());
        assert!(!result.object.as_ref().unwrap().bytes.is_empty());
    }

    #[test]
    fn codegen_function_definition() {
        let db = Database::default();
        let result = compile_source(&db, "fn add(a, b) { a + b }");

        assert!(result.error.is_none(), "codegen error: {:?}", result.error);
        assert!(result.object.is_some());
    }

    #[test]
    fn codegen_function_call() {
        let db = Database::default();
        let result = compile_source(
            &db,
            "fn double(x) { x + x }
             double(5)",
        );

        assert!(result.error.is_none(), "codegen error: {:?}", result.error);
        assert!(result.object.is_some());
    }

    #[test]
    fn codegen_multiple_functions() {
        let db = Database::default();
        let result = compile_source(
            &db,
            "fn foo() { 1 }
             fn bar() { foo() + 1 }
             bar()",
        );

        assert!(result.object.is_some());
    }

    #[test]
    fn codegen_control_flow() {
        let db = Database::default();
        let result = compile_source(&db, "x := if true { 1 } else { 2 }");

        assert!(result.object.is_some());
    }

    #[test]
    fn codegen_while_loop() {
        let db = Database::default();
        let result = compile_source(
            &db,
            "x := 0
             while x < 10 {
                 x = x + 1
             }",
        );

        assert!(result.object.is_some());
    }

    #[test]
    fn codegen_echo() {
        let db = Database::default();
        let result = compile_source(&db, "echo 42");

        assert!(result.object.is_some());
    }

    #[test]
    fn codegen_for_loop() {
        let db = Database::default();
        let result = compile_source(
            &db,
            "sum := 0
             for i in 0..10 {
                 sum = sum + i
             }",
        );

        assert!(result.object.is_some());
    }

    #[test]
    fn codegen_arithmetic() {
        let db = Database::default();
        let result = compile_source(&db, "x := 1 + 2 * 3 - 4 / 2");

        assert!(result.object.is_some());
    }

    // Tests for v1-unsupported features (should error)
    #[test]
    fn codegen_list_unsupported() {
        let db = Database::default();
        let result = compile_source(&db, "xs := [1, 2, 3]");

        // Lists not supported in v1 - should have error
        assert!(result.error.is_some());
    }

    #[test]
    fn codegen_closure_unsupported() {
        let db = Database::default();
        let result = compile_source(
            &db,
            "x := 10
             f := fn(y) { x + y }
             f(5)",
        );

        // Closures not supported in v1 - should have error
        assert!(result.error.is_some());
    }

    #[test]
    fn codegen_tuple_unsupported() {
        let db = Database::default();
        let result = compile_source(&db, "t := (1, 2, 3)");

        // Tuples not supported in v1 - should have error
        assert!(result.error.is_some());
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
            let _result = codegen(&db, ast);

            let exec_after_first = db.counters.will_execute();
            assert!(exec_after_first > 0);

            db.counters.reset();

            // Same input - should be memoized
            let _result2 = codegen(&db, ast);

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
            let _result1 = codegen(&db, ast1);

            db.counters.reset();

            // Different source - should re-execute
            let source2 = SourceText::new(&db, "y := 2".to_string());
            let file_text2 = source_to_file_text(&db, source2);
            let parsed2 = parse_text(&db, file_text2);
            let ast2 = ast_file(&db, parsed2);
            let _result2 = codegen(&db, ast2);

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
            let _result = codegen(&db, ast);

            let first_exec_count = db.counters.will_execute();
            assert!(first_exec_count > 0);

            db.counters.reset();

            // Run entire pipeline again with same source input
            let file_text2 = source_to_file_text(&db, source);
            let parsed2 = parse_text(&db, file_text2);
            let ast2 = ast_file(&db, parsed2);
            let _result2 = codegen(&db, ast2);

            assert_eq!(
                db.counters.will_execute(),
                0,
                "full pipeline should be memoized"
            );
        }
    }
}
