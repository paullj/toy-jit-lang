//! Dependency graph construction and algorithms

use crate::ModuleId;
use crate::error::{Diagnostic, GraphError, PathError};
use crate::path::ModulePath;
use crate::symbols::ModuleImportsExports;
use petgraph::Direction;
use petgraph::algo::{kosaraju_scc, toposort};
use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::HashMap;

/// Result of graph building phase
#[derive(Debug, Clone)]
pub struct GraphBuildResult {
    pub graph: DependencyGraph,
    pub diagnostics: Vec<Diagnostic>,
}

/// Wrapper around petgraph for module dependencies
#[derive(Debug, Clone)]
pub struct DependencyGraph {
    /// Petgraph directed graph
    graph: DiGraph<ModuleId, ()>,
    /// Map from ModuleId to NodeIndex
    node_map: HashMap<ModuleId, NodeIndex>,
    /// Reverse map
    index_map: HashMap<NodeIndex, ModuleId>,
}

impl DependencyGraph {
    /// Create new empty graph
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            node_map: HashMap::new(),
            index_map: HashMap::new(),
        }
    }

    /// Add a node for a module
    pub fn add_node(&mut self, module: ModuleId) {
        if !self.node_map.contains_key(&module) {
            let idx = self.graph.add_node(module);
            self.node_map.insert(module, idx);
            self.index_map.insert(idx, module);
        }
    }

    /// Add dependency: from imports to
    pub fn add_dependency(&mut self, from: ModuleId, to: ModuleId) {
        self.add_node(from);
        self.add_node(to);

        let from_idx = self.node_map[&from];
        let to_idx = self.node_map[&to];

        // Avoid duplicate edges
        if !self.graph.contains_edge(from_idx, to_idx) {
            self.graph.add_edge(from_idx, to_idx, ());
        }
    }

    /// Find strongly connected components (cycles)
    pub fn find_sccs(&self) -> Vec<Vec<ModuleId>> {
        let sccs = kosaraju_scc(&self.graph);
        sccs.into_iter()
            .map(|scc| scc.into_iter().map(|idx| self.index_map[&idx]).collect())
            .collect()
    }

    /// Topological sort (may fail if cycles exist)
    pub fn toposort(&self) -> Result<Vec<ModuleId>, Vec<ModuleId>> {
        match toposort(&self.graph, None) {
            Ok(order) => Ok(order.into_iter().map(|idx| self.index_map[&idx]).collect()),
            Err(_) => {
                // Return the cycles
                let sccs = self.find_sccs();
                let cycles: Vec<_> = sccs
                    .into_iter()
                    .filter(|scc| scc.len() > 1)
                    .flatten()
                    .collect();
                Err(cycles)
            }
        }
    }

    /// Get direct dependencies (modules that this module imports)
    pub fn dependencies(&self, module: ModuleId) -> Vec<ModuleId> {
        if let Some(&idx) = self.node_map.get(&module) {
            self.graph
                .neighbors_directed(idx, Direction::Outgoing)
                .map(|idx| self.index_map[&idx])
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get reverse dependencies (modules that import this module)
    pub fn dependents(&self, module: ModuleId) -> Vec<ModuleId> {
        if let Some(&idx) = self.node_map.get(&module) {
            self.graph
                .neighbors_directed(idx, Direction::Incoming)
                .map(|idx| self.index_map[&idx])
                .collect()
        } else {
            Vec::new()
        }
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Build dependency graph from collected imports/exports
pub fn build_graph(
    collected: &[ModuleImportsExports],
    module_paths: &HashMap<ModuleId, ModulePath>,
) -> Result<GraphBuildResult, GraphError> {
    let mut graph = DependencyGraph::new();
    let mut diagnostics = Vec::new();

    // Add all nodes first
    for module in collected {
        graph.add_node(module.module_id);
    }

    // Add edges based on imports
    for module in collected {
        for import in &module.imports {
            match resolve_import_path(&import.path, module.module_id, module_paths) {
                Ok(target_id) => {
                    graph.add_dependency(module.module_id, target_id);
                }
                Err(e) => {
                    let message = match e {
                        PathError::ModuleNotFound(ref path) => {
                            format!("Cannot resolve import: module not found: {:?}", path)
                        }
                        PathError::BaseNotFound(id) => {
                            format!(
                                "Cannot resolve relative import: base module {:?} not found",
                                id
                            )
                        }
                        PathError::TooManyParents => {
                            "Too many '..' in relative import path".to_string()
                        }
                        PathError::InvalidPath => "Invalid import path".to_string(),
                    };
                    diagnostics.push(Diagnostic::error(message, import.span));
                }
            }
        }
    }

    Ok(GraphBuildResult { graph, diagnostics })
}

/// Resolve import path to target module ID
fn resolve_import_path(
    import_path: &ModulePath,
    from_module: ModuleId,
    module_paths: &HashMap<ModuleId, ModulePath>,
) -> Result<ModuleId, PathError> {
    match import_path {
        ModulePath::Relative { .. } => {
            // Need to resolve relative to current module's path
            let base_path = module_paths
                .get(&from_module)
                .ok_or(PathError::BaseNotFound(from_module))?;

            let resolved = import_path.resolve_relative_to(base_path)?;

            // Find module ID with this path
            module_paths
                .iter()
                .find(|(_, path)| **path == resolved)
                .map(|(id, _)| *id)
                .ok_or(PathError::ModuleNotFound(resolved))
        }
        _ => {
            // Absolute path - find matching module
            module_paths
                .iter()
                .find(|(_, path)| **path == *import_path)
                .map(|(id, _)| *id)
                .ok_or(PathError::ModuleNotFound(import_path.clone()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_creation() {
        let mut graph = DependencyGraph::new();
        let m1 = ModuleId(1);
        let m2 = ModuleId(2);

        graph.add_dependency(m1, m2);

        assert_eq!(graph.dependencies(m1), vec![m2]);
        assert_eq!(graph.dependents(m2), vec![m1]);
    }

    #[test]
    fn test_cycle_detection() {
        let mut graph = DependencyGraph::new();
        let m1 = ModuleId(1);
        let m2 = ModuleId(2);

        graph.add_dependency(m1, m2);
        graph.add_dependency(m2, m1);

        let sccs = graph.find_sccs();
        let has_cycle = sccs.iter().any(|scc| scc.len() > 1);
        assert!(has_cycle);
    }
}
