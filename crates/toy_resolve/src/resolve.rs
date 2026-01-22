//! Phase 3: Symbol resolution across modules

use crate::ModuleId;
use crate::error::{Diagnostic, ResolveError};
use crate::graph::DependencyGraph;
use crate::symbols::{ImportKind, ModuleImportsExports, SymbolTable};
use std::collections::HashMap;

/// Final resolution result
#[derive(Debug, Clone)]
pub struct Resolution {
    /// Dependency graph
    pub graph: DependencyGraph,
    /// Symbol tables per module
    pub symbol_tables: HashMap<ModuleId, SymbolTable>,
    /// Diagnostics
    pub diagnostics: Vec<Diagnostic>,
}

/// Resolve all symbols using dependency graph
pub fn resolve_all_symbols(
    graph: &DependencyGraph,
    collected: &[ModuleImportsExports],
) -> Resolution {
    let mut symbol_tables = HashMap::new();
    let mut diagnostics = Vec::new();

    // Try topological sort first
    match graph.toposort() {
        Ok(order) => {
            // No cycles - resolve in dependency order
            for module_id in order {
                match resolve_module_sequential(module_id, collected, &symbol_tables) {
                    Ok(table) => {
                        symbol_tables.insert(module_id, table);
                    }
                    Err(e) => {
                        diagnostics.push(Diagnostic::error(
                            format!("Failed to resolve module {:?}: {:?}", module_id, e),
                            None,
                        ));
                    }
                }
            }
        }
        Err(_) => {
            // Has cycles - use SCCs
            let sccs = graph.find_sccs();
            for scc in sccs {
                if scc.len() == 1 {
                    // Single module - no cycle
                    match resolve_module_sequential(scc[0], collected, &symbol_tables) {
                        Ok(table) => {
                            symbol_tables.insert(scc[0], table);
                        }
                        Err(e) => {
                            diagnostics.push(Diagnostic::error(
                                format!("Failed to resolve module {:?}: {:?}", scc[0], e),
                                None,
                            ));
                        }
                    }
                } else {
                    // Multiple modules in cycle
                    match resolve_scc_fixpoint(&scc, collected, &symbol_tables) {
                        Ok(tables) => {
                            symbol_tables.extend(tables);
                        }
                        Err(e) => {
                            diagnostics.push(Diagnostic::error(
                                format!("Failed to resolve cyclic modules {:?}: {:?}", scc, e),
                                None,
                            ));
                        }
                    }
                }
            }
        }
    }

    Resolution {
        graph: graph.clone(),
        symbol_tables,
        diagnostics,
    }
}

/// Resolve a single module's symbols (dependencies already resolved)
fn resolve_module_sequential(
    module_id: ModuleId,
    collected: &[ModuleImportsExports],
    _resolved: &HashMap<ModuleId, SymbolTable>,
) -> Result<SymbolTable, ResolveError> {
    let module_data = collected
        .iter()
        .find(|m| m.module_id == module_id)
        .ok_or(ResolveError::ModuleDataNotFound(module_id))?;

    let mut table = SymbolTable::new(module_id);

    // Add local exports
    for export in &module_data.exports {
        table.add_local_symbol(export);
    }

    // Process imports and re-exports
    for _import in &module_data.imports {
        // Try to find the imported module in resolved tables
        // For now, skip if not found (proper error handling would add diagnostics)

        // TODO: Need to map ModulePath to ModuleId
        // This requires additional context from the compiler
        // For now, this is a placeholder
    }

    Ok(table)
}

/// Resolve strongly connected component (cycle) using fixpoint iteration
fn resolve_scc_fixpoint(
    scc: &[ModuleId],
    collected: &[ModuleImportsExports],
    _resolved: &HashMap<ModuleId, SymbolTable>,
) -> Result<HashMap<ModuleId, SymbolTable>, ResolveError> {
    let mut tables: HashMap<ModuleId, SymbolTable> = HashMap::new();

    // Initialize with local symbols only
    for &module_id in scc {
        let module_data = collected
            .iter()
            .find(|m| m.module_id == module_id)
            .ok_or(ResolveError::ModuleDataNotFound(module_id))?;

        let mut table = SymbolTable::new(module_id);

        // Add local exports
        for export in &module_data.exports {
            table.add_local_symbol(export);
        }

        tables.insert(module_id, table);
    }

    // Fixpoint iteration: resolve re-exports until stable
    let max_iterations = 100; // Prevent infinite loops
    for iteration in 0..max_iterations {
        let changed = false;

        for &module_id in scc {
            let module_data = collected.iter().find(|m| m.module_id == module_id).unwrap();

            // Try to resolve each public import (re-export)
            for import in &module_data.imports {
                if !import.is_pub {
                    continue; // Only care about re-exports
                }

                // TODO: Need to map ModulePath to ModuleId
                // This requires additional context from the compiler
                // For now, this is a placeholder
            }
        }

        if !changed {
            break; // Reached fixpoint
        }

        if iteration == max_iterations - 1 {
            return Err(ResolveError::FixpointNotReached(scc.to_vec()));
        }
    }

    Ok(tables)
}

/// Helper to add re-exported symbols from source table
#[allow(dead_code)]
fn add_reexport_symbols(
    tables: &mut HashMap<ModuleId, SymbolTable>,
    module_id: ModuleId,
    import: &crate::symbols::Import,
    source_table: &SymbolTable,
) -> bool {
    let mut changed = false;

    let table = tables.get_mut(&module_id).unwrap();

    match &import.kind {
        ImportKind::All { alias } => {
            // Re-export all public symbols from source
            for (name, symbol) in &source_table.exports {
                let export_name = if let Some(a) = alias {
                    format!("{}.{}", a, name)
                } else {
                    name.clone()
                };

                if table.add_reexport(export_name, symbol, source_table.module_id) {
                    changed = true;
                }
            }
        }
        ImportKind::Selective { items } => {
            // Re-export specific symbols
            for item in items {
                if let Some(symbol) = source_table.exports.get(&item.name) {
                    let local_name = item.alias.as_ref().unwrap_or(&item.name).clone();

                    if table.add_reexport(local_name, symbol, source_table.module_id) {
                        changed = true;
                    }
                }
            }
        }
    }

    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::symbols::Export;
    use crate::symbols::SymbolKind;
    use text_size::TextRange;

    #[test]
    fn test_resolve_empty_module() {
        let graph = DependencyGraph::new();
        let collected = vec![];

        let resolution = resolve_all_symbols(&graph, &collected);
        assert!(resolution.symbol_tables.is_empty());
    }

    #[test]
    fn test_resolve_single_module() {
        let mut graph = DependencyGraph::new();
        let module_id = ModuleId(1);
        graph.add_node(module_id);

        let collected = vec![ModuleImportsExports {
            module_id,
            imports: vec![],
            exports: vec![Export {
                name: "test".to_string(),
                kind: SymbolKind::Function,
                is_pub: true,
                span: TextRange::default(),
            }],
        }];

        let resolution = resolve_all_symbols(&graph, &collected);
        assert_eq!(resolution.symbol_tables.len(), 1);
        assert!(
            resolution.symbol_tables[&module_id]
                .exports
                .contains_key("test")
        );
    }
}
