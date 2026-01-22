//! Module resolution for the Toy language
//!
//! This crate provides three-phase module resolution:
//! 1. Collection: Extract imports and exports from ASTs (parallelizable)
//! 2. Graph Building: Construct dependency graph with cycle detection
//! 3. Symbol Resolution: Resolve symbols across modules with re-exports

pub mod collect;
pub mod error;
pub mod graph;
pub mod path;
pub mod resolve;
pub mod symbols;

pub use collect::collect_module_imports_exports;
pub use error::{CollectError, Diagnostic, DiagnosticLevel, GraphError, ResolveError};
pub use graph::{DependencyGraph, GraphBuildResult, build_graph};
pub use path::ModulePath;
pub use resolve::{Resolution, resolve_all_symbols};
pub use symbols::{
    Export, Import, ImportItem, ImportKind, ModuleImportsExports, ResolvedSymbol, SymbolKind,
    SymbolSource, SymbolTable,
};

use std::collections::HashMap;
use std::sync::Arc;

/// Module identifier - opaque type from compiler
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ModuleId(pub u32);

/// Convenience function: Run all three phases in sequence with AST provider
///
/// Note: Collection phase runs sequentially here. For parallelization,
/// call `collect_module_imports_exports` directly with your threadpool.
pub fn resolve_modules<F>(
    module_ids: &[ModuleId],
    module_paths: HashMap<ModuleId, ModulePath>,
    get_ast: F,
) -> Result<Resolution, ResolveError>
where
    F: Fn(ModuleId) -> Option<Arc<toy_ast::Root>>,
{
    // Phase 1: Collect (sequential - caller can parallelize externally)
    let collected: Result<Vec<_>, _> = module_ids
        .iter()
        .map(|&id| collect_module_imports_exports(id, &get_ast))
        .collect();

    let collected = collected?;

    // Phase 2: Build graph
    let graph_result = build_graph(&collected, &module_paths)?;

    // Phase 3: Resolve symbols
    let mut resolution = resolve_all_symbols(&graph_result.graph, &collected);
    resolution.diagnostics.extend(graph_result.diagnostics);

    Ok(resolution)
}

/// Simple convenience function: Run all three phases with ASTs provided directly
///
/// This is a simpler API when you already have all ASTs in memory.
/// For Salsa integration or lazy evaluation, use `resolve_modules` instead.
pub fn resolve_modules_with_asts(
    module_asts: HashMap<ModuleId, Arc<toy_ast::Root>>,
    module_paths: HashMap<ModuleId, ModulePath>,
) -> Result<Resolution, ResolveError> {
    let module_ids: Vec<_> = module_asts.keys().copied().collect();
    let get_ast = move |id: ModuleId| module_asts.get(&id).cloned();

    resolve_modules(&module_ids, module_paths, get_ast)
}
