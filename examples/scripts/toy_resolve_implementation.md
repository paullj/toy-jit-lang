# toy_resolve Implementation Plan

## Overview
`toy_resolve` is a module resolution crate that builds a dependency graph from multiple AST files, resolves imports, and creates a cross-module symbol index while gracefully handling cyclical dependencies.

## Core Requirements

### Functional Requirements
1. **Multi-file AST Processing**: Accept multiple AST roots through a provider function
2. **Path Resolution**: Resolve import paths (std, pkg, relative, absolute) to module identifiers
3. **Dependency Graph**: Build directed graph using petgraph library
4. **Cycle Detection**: Detect and handle circular dependencies gracefully
5. **Symbol Resolution**: Track exported symbols and their visibility across modules
6. **Re-export Handling**: Process `pub use` statements correctly
7. **Salsa Integration**: Support memoization and incremental computation
8. **Error Recovery**: Continue resolution even with missing/broken modules

### Non-Functional Requirements
1. **Performance**: Collection phase is parallelizable by caller, efficient graph algorithms
2. **Memory Efficiency**: Share string interning with AST/HIR
3. **Salsa Compatibility**: Immutable outputs, deterministic computation
4. **Testability**: Comprehensive unit and integration tests
5. **No Internal Parallelization**: Caller handles threadpools/parallelism

## Architecture

### High-Level Design
```
┌─────────────────────┐
│  AST Provider Fn    │ (Compiler provides)
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│ Phase 1: Collect    │ (Parallelizable via threadpool)
│ - Scan imports      │
│ - Scan exports      │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│ Phase 2: Build      │ (Sequential)
│ - Construct graph   │
│ - Detect cycles     │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│ Phase 3: Resolve    │ (Sequential/SCC)
│ - Link symbols      │
│ - Process reexports │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│  Resolution Output  │ (Salsa-tracked)
│  - Dep graph        │
│  - Symbol tables    │
│  - Diagnostics      │
└─────────────────────┘
           │
           ▼
    ┌──────────────┐
    │   Compiler   │
    │   (HIR/Type) │
    └──────────────┘
```

### Module Structure
```
toy_resolve/
├── Cargo.toml              # Dependencies: petgraph
├── src/
│   ├── lib.rs              # Public API (3 phases)
│   ├── collect.rs          # Phase 1: Import/export collection
│   ├── graph.rs            # Phase 2: Graph building with petgraph
│   ├── resolve.rs          # Phase 3: Symbol resolution
│   ├── path.rs             # Module path types
│   ├── symbols.rs          # Symbol table (Salsa-compatible)
│   ├── visitor.rs          # AST visitor helpers
│   ├── error.rs            # Error types
│   └── salsa_types.rs      # Salsa input/output wrappers
```

## Data Structures

### 1. AST Provider Interface
```rust
/// Compiler provides this function to get ASTs by module ID
/// This allows the compiler to control parsing and caching
pub type AstProvider = dyn Fn(ModuleId) -> Option<Arc<Root>>;

/// Module identifier (opaque type from compiler)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ModuleId(pub u32);

/// Input for resolution (Salsa-compatible)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolutionInput {
    /// All module IDs to resolve
    pub module_ids: Vec<ModuleId>,
    /// Module ID to path mapping (for diagnostics)
    pub module_paths: HashMap<ModuleId, ModulePath>,
}
```

### 2. Module Path Representation
```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ModulePath {
    /// Standard library: std.io
    Std(Vec<Ident>),
    /// Package: pkg.http
    Package(Vec<Ident>),
    /// Relative: .helpers, ..utils
    Relative {
        up: usize,           // Number of ".."
        segments: Vec<Ident>
    },
    /// Absolute: src.utils.helpers
    Absolute(Vec<Ident>),
}

impl ModulePath {
    /// Resolve relative to a base path
    pub fn resolve_relative_to(&self, base: &ModulePath) -> ModulePath;

    /// Convert to file system path
    pub fn to_fs_path(&self, project_root: &Path) -> PathBuf;
}
```

### 3. Dependency Graph (using petgraph)
```rust
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::algo::{kosaraju_scc, toposort};

/// Wrapper around petgraph for module dependencies
/// This is Salsa-compatible (immutable after construction)
#[derive(Debug, Clone, PartialEq, Eq)]
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
    pub fn new() -> Self;

    /// Add dependency: from imports to
    pub fn add_dependency(&mut self, from: ModuleId, to: ModuleId);

    /// Find strongly connected components (cycles)
    pub fn find_sccs(&self) -> Vec<Vec<ModuleId>>;

    /// Topological sort (may fail if cycles)
    pub fn toposort(&self) -> Result<Vec<ModuleId>, Vec<ModuleId>>;

    /// Get direct dependencies
    pub fn dependencies(&self, module: ModuleId) -> Vec<ModuleId>;

    /// Get reverse dependencies
    pub fn dependents(&self, module: ModuleId) -> Vec<ModuleId>;
}
```

### 4. Collection Phase Output (Phase 1)
```rust
/// Output from parallel collection phase
/// Salsa can track individual ModuleImportsExports
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleImportsExports {
    pub module_id: ModuleId,
    pub imports: Vec<Import>,
    pub exports: Vec<Export>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Import {
    /// Source location
    pub span: TextRange,
    /// Module path being imported
    pub path: ModulePath,
    /// What to import
    pub kind: ImportKind,
    /// Is public re-export?
    pub is_pub: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ImportKind {
    /// use std.io - import all public symbols
    All { alias: Option<String> },
    /// use std.math.{sin, cos} - selective import
    Selective { items: Vec<ImportItem> },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ImportItem {
    pub name: String,
    pub alias: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Export {
    /// Symbol name
    pub name: String,
    /// Symbol kind
    pub kind: SymbolKind,
    /// Is public?
    pub is_pub: bool,
    /// Definition location
    pub span: TextRange,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SymbolKind {
    Function,
    Variable,
}
```

### 5. Resolution Output (Phase 3)
```rust
/// Final resolution result - Salsa can track this
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolution {
    /// Dependency graph
    pub graph: DependencyGraph,
    /// Symbol tables per module
    pub symbol_tables: HashMap<ModuleId, SymbolTable>,
    /// Diagnostics
    pub diagnostics: Vec<Diagnostic>,
}

/// Symbol table for a resolved module
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolTable {
    /// Module ID
    pub module_id: ModuleId,
    /// Public exports
    pub exports: HashMap<String, ResolvedSymbol>,
    /// All symbols (including private)
    pub all_symbols: HashMap<String, ResolvedSymbol>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedSymbol {
    pub name: String,
    pub kind: SymbolKind,
    pub is_pub: bool,
    pub span: TextRange,
    /// Where this symbol came from (for re-exports)
    pub source: SymbolSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymbolSource {
    /// Defined locally
    Local,
    /// Re-exported from another module
    ReExport { from_module: ModuleId, original_name: String },
}
```

## Algorithms

### 1. Three-Phase Resolution Pipeline

#### Phase 1: Collection (collect.rs)
```rust
/// Collect imports and exports from a single module
/// This is a pure function - Salsa can memoize per module
/// Can be called in parallel from external threadpool
pub fn collect_module_info(
    module_id: ModuleId,
    ast_provider: &impl Fn(ModuleId) -> Option<Arc<Root>>,
) -> Result<ModuleImportsExports, CollectError> {
    let ast = ast_provider(module_id)
        .ok_or(CollectError::AstNotFound(module_id))?;

    let mut visitor = CollectVisitor::new(module_id);
    visitor.visit_root(&ast);

    Ok(ModuleImportsExports {
        module_id,
        imports: visitor.imports,
        exports: visitor.exports,
    })
}

// Note: Compiler handles parallelization with Salsa's built-in parallelism
// or external threadpool (rayon, tokio, etc.)
```

#### Phase 2: Sequential Graph Building (graph.rs)
```rust
/// Build dependency graph from collected imports
/// Must be sequential to ensure deterministic ordering
pub fn build_dependency_graph(
    collected: &[ModuleImportsExports],
    module_paths: &HashMap<ModuleId, ModulePath>,
) -> Result<DependencyGraph, GraphError> {
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
                    diagnostics.push(Diagnostic::error(e, import.span));
                }
            }
        }
    }

    Ok(graph)
}
```

#### Phase 3: Sequential Symbol Resolution (resolve.rs)
```rust
/// Resolve symbols using dependency order or SCCs
pub fn resolve_symbols(
    graph: &DependencyGraph,
    collected: &[ModuleImportsExports],
) -> Result<Resolution, ResolveError> {
    let mut symbol_tables = HashMap::new();
    let mut diagnostics = Vec::new();

    // Try topological sort first
    match graph.toposort() {
        Ok(order) => {
            // No cycles - resolve in dependency order
            for module_id in order {
                let table = resolve_module_sequential(module_id, collected, &symbol_tables)?;
                symbol_tables.insert(module_id, table);
            }
        }
        Err(_) => {
            // Has cycles - use SCCs
            let sccs = graph.find_sccs();
            for scc in sccs {
                let tables = resolve_scc_fixpoint(&scc, collected, &symbol_tables)?;
                symbol_tables.extend(tables);
            }
        }
    }

    Ok(Resolution {
        graph: graph.clone(),
        symbol_tables,
        diagnostics,
    })
}
```

### 2. Cycle Handling with Fixpoint Iteration
```rust
/// Resolve strongly connected component (cycle) using fixpoint iteration
fn resolve_scc_fixpoint(
    scc: &[ModuleId],
    collected: &[ModuleImportsExports],
    resolved: &HashMap<ModuleId, SymbolTable>,
) -> Result<HashMap<ModuleId, SymbolTable>, ResolveError> {
    let mut tables: HashMap<ModuleId, SymbolTable> = HashMap::new();

    // Initialize with local symbols only
    for &module_id in scc {
        let module_data = collected.iter()
            .find(|m| m.module_id == module_id)
            .ok_or(ResolveError::ModuleNotFound(module_id))?;

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
        let mut changed = false;

        for &module_id in scc {
            let module_data = collected.iter()
                .find(|m| m.module_id == module_id)
                .unwrap();

            // Try to resolve each import
            for import in &module_data.imports {
                if !import.is_pub {
                    continue; // Only care about re-exports
                }

                // Look up the imported module (could be in this SCC or already resolved)
                let source_table = tables.get(&import.path.module_id())
                    .or_else(|| resolved.get(&import.path.module_id()));

                if let Some(source) = source_table {
                    if add_reexport_symbols(&mut tables, module_id, import, source) {
                        changed = true;
                    }
                }
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
```

### 3. Path Resolution Helper
```rust
/// Resolve import path to target module ID
/// This needs the module_paths mapping from the compiler
fn resolve_import_path(
    import_path: &ModulePath,
    from_module: ModuleId,
    module_paths: &HashMap<ModuleId, ModulePath>,
) -> Result<ModuleId, PathError> {
    match import_path {
        ModulePath::Relative { up, segments } => {
            // Need to resolve relative to current module's path
            let base_path = module_paths.get(&from_module)
                .ok_or(PathError::BaseNotFound(from_module))?;

            let resolved = base_path.resolve_relative(*up, segments)?;

            // Find module ID with this path
            module_paths.iter()
                .find(|(_, path)| *path == &resolved)
                .map(|(id, _)| *id)
                .ok_or(PathError::ModuleNotFound(resolved))
        }
        _ => {
            // Absolute path - find matching module
            module_paths.iter()
                .find(|(_, path)| *path == import_path)
                .map(|(id, _)| *id)
                .ok_or(PathError::ModuleNotFound(import_path.clone()))
        }
    }
}
```

### 4. Sequential Module Resolution
```rust
/// Resolve a single module's symbols (dependencies already resolved)
fn resolve_module_sequential(
    module_id: ModuleId,
    collected: &[ModuleImportsExports],
    resolved: &HashMap<ModuleId, SymbolTable>,
) -> Result<SymbolTable, ResolveError> {
    let module_data = collected.iter()
        .find(|m| m.module_id == module_id)
        .ok_or(ResolveError::ModuleNotFound(module_id))?;

    let mut table = SymbolTable::new(module_id);

    // Add local exports
    for export in &module_data.exports {
        table.add_local_symbol(export);
    }

    // Process imports and re-exports
    for import in &module_data.imports {
        if let Some(source_table) = resolved.get(&import.path.module_id()) {
            match &import.kind {
                ImportKind::All { alias } => {
                    // Re-export all public symbols from source
                    for (name, symbol) in &source_table.exports {
                        let export_name = if let Some(a) = alias {
                            format!("{}.{}", a, name)
                        } else {
                            name.clone()
                        };

                        if import.is_pub {
                            table.add_reexport(export_name, symbol, import.path.module_id());
                        }
                    }
                }
                ImportKind::Selective { items } => {
                    // Re-export specific symbols
                    for item in items {
                        if let Some(symbol) = source_table.exports.get(&item.name) {
                            let local_name = item.alias.as_ref().unwrap_or(&item.name);

                            if import.is_pub {
                                table.add_reexport(local_name.clone(), symbol, import.path.module_id());
                            }
                        } else {
                            // Symbol not found - add diagnostic
                        }
                    }
                }
            }
        }
    }

    Ok(table)
}
```

## API Design

### Public Interface (Three Separate Functions)
```rust
// ============================================================================
// PHASE 1: Collection (Parallelizable by Caller)
// ============================================================================

/// Collect imports and exports for a single module
/// This can be memoized per module by Salsa
/// Call this function from your threadpool for parallelization
pub fn collect_module_imports_exports(
    module_id: ModuleId,
    get_ast: impl Fn(ModuleId) -> Option<Arc<Root>>,
) -> Result<ModuleImportsExports, CollectError> {
    // Implementation in collect.rs
}

// Example usage with Salsa parallelism:
// ```
// fn all_imports_exports(db: &dyn Resolve) -> Vec<ModuleImportsExports> {
//     db.all_module_ids()
//         .iter()
//         .map(|&id| db.module_imports_exports(id))
//         .collect()
// }
// ```
//
// Example usage with external threadpool (rayon):
// ```
// use rayon::prelude::*;
// let collected: Vec<_> = module_ids
//     .par_iter()
//     .filter_map(|&id| collect_module_imports_exports(id, &get_ast).ok())
//     .collect();
// ```

// ============================================================================
// PHASE 2: Sequential Graph Building
// ============================================================================

/// Build dependency graph from collected data
/// This must be deterministic and sequential
/// Salsa can memoize this based on the collected data
pub fn build_graph(
    collected: &[ModuleImportsExports],
    module_paths: &HashMap<ModuleId, ModulePath>,
) -> Result<GraphBuildResult, GraphError> {
    // Implementation in graph.rs
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphBuildResult {
    pub graph: DependencyGraph,
    pub diagnostics: Vec<Diagnostic>,
}

// ============================================================================
// PHASE 3: Sequential Symbol Resolution
// ============================================================================

/// Resolve all symbols using the graph
/// This must be deterministic and sequential
/// Salsa can memoize this based on graph + collected data
pub fn resolve_all_symbols(
    graph: &DependencyGraph,
    collected: &[ModuleImportsExports],
) -> Resolution {
    // Implementation in resolve.rs
}

// ============================================================================
// Convenience: All-in-One
// ============================================================================

/// Run all three phases in sequence
/// Useful for testing or simple use cases
/// Note: Collection phase runs sequentially here; caller can parallelize if needed
pub fn resolve_modules(
    module_ids: &[ModuleId],
    module_paths: HashMap<ModuleId, ModulePath>,
    get_ast: impl Fn(ModuleId) -> Option<Arc<Root>>,
) -> Result<Resolution, ResolveError> {
    // Phase 1: Collect sequentially (caller can parallelize by calling collect_module_imports_exports directly)
    let collected: Vec<_> = module_ids
        .iter()
        .filter_map(|&id| collect_module_imports_exports(id, &get_ast).ok())
        .collect();

    // Phase 2
    let graph_result = build_graph(&collected, &module_paths)?;

    // Phase 3
    let mut resolution = resolve_all_symbols(&graph_result.graph, &collected);
    resolution.diagnostics.extend(graph_result.diagnostics);

    Ok(resolution)
}
```

### Salsa Integration Example
```rust
// In the compiler's Salsa database:

#[salsa::query_group(ResolveDatabase)]
trait Resolve: salsa::Database {
    /// Salsa will memoize this per module
    #[salsa::invoke(toy_resolve::collect_module_imports_exports)]
    fn module_imports_exports(
        &self,
        module_id: ModuleId
    ) -> Result<ModuleImportsExports, CollectError>;

    /// Derived query: collect all modules
    fn all_imports_exports(&self) -> Vec<ModuleImportsExports>;

    /// Build graph from collected data
    #[salsa::invoke(toy_resolve::build_graph)]
    fn dependency_graph(&self) -> Result<GraphBuildResult, GraphError>;

    /// Final resolution
    #[salsa::invoke(toy_resolve::resolve_all_symbols)]
    fn resolution(&self) -> Resolution;
}

// Compiler implementation (Salsa handles parallelism automatically):
fn all_imports_exports(db: &dyn Resolve) -> Vec<ModuleImportsExports> {
    let module_ids = db.all_module_ids(); // Compiler provides this

    // Salsa parallelizes this automatically when queries are independent
    module_ids
        .iter()
        .filter_map(|&id| db.module_imports_exports(id).ok())
        .collect()
}

fn dependency_graph(db: &dyn Resolve) -> Result<GraphBuildResult, GraphError> {
    let collected = db.all_imports_exports();
    let paths = db.module_path_map();

    toy_resolve::build_graph(&collected, &paths)
}

fn resolution(db: &dyn Resolve) -> Resolution {
    let graph = db.dependency_graph().unwrap();
    let collected = db.all_imports_exports();

    toy_resolve::resolve_all_symbols(&graph.graph, &collected)
}
```

## Error Handling

### Error Types
```rust
#[derive(Debug)]
pub enum ResolveError {
    /// Module not found
    ModuleNotFound(ModulePath),

    /// Symbol not found in module
    SymbolNotFound {
        module: ModulePath,
        symbol: String
    },

    /// Private symbol accessed
    PrivateSymbol {
        module: ModulePath,
        symbol: String
    },

    /// Circular dependency detected
    CircularDependency(Vec<ModulePath>),

    /// Parse error in module
    ParseError {
        module: PathBuf,
        error: ParseError
    },

    /// IO error reading module
    IoError(std::io::Error),

    /// Conflicting exports
    ConflictingExports {
        name: String,
        modules: Vec<ModulePath>
    },
}

pub struct Diagnostic {
    pub level: DiagnosticLevel,
    pub message: String,
    pub span: Option<TextRange>,
    pub notes: Vec<String>,
}

pub enum DiagnosticLevel {
    Error,
    Warning,
    Info,
}
```

## Testing Strategy

### Unit Tests
1. **Path Resolution**
   - Test relative path resolution
   - Test absolute path resolution
   - Test std/pkg path resolution
   - Edge cases (empty paths, "..", etc.)

2. **Dependency Graph**
   - Test cycle detection
   - Test topological sort
   - Test SCC computation
   - Test path queries

3. **Symbol Resolution**
   - Test public/private visibility
   - Test re-exports
   - Test selective imports
   - Test import aliases

### Integration Tests
```rust
// tests/simple_modules.rs
#[test]
fn test_basic_module_resolution() {
    let files = vec![
        ("main.toy", "use .math\necho math.add(1, 2)"),
        ("math.toy", "pub fn add(a: int, b: int): int { a + b }"),
    ];

    let resolution = resolve_test_files(files);
    assert!(resolution.diagnostics.is_empty());
    assert_eq!(resolution.modules.len(), 2);
}

// tests/cycles.rs
#[test]
fn test_circular_dependencies() {
    let files = vec![
        ("a.toy", "use .b\npub fn foo() { b.bar() }"),
        ("b.toy", "use .a\npub fn bar() { a.foo() }"),
    ];

    let resolution = resolve_test_files(files);
    // Should handle cycle gracefully
    assert!(resolution.modules["a"].is_in_cycle);
    assert!(resolution.modules["b"].is_in_cycle);
}

// tests/reexports.rs
#[test]
fn test_reexports() {
    let files = vec![
        ("main.toy", "use .lib\nlib.helper()"),
        ("lib.toy", "pub use .internal.helper"),
        ("internal.toy", "pub fn helper() { }"),
    ];

    let resolution = resolve_test_files(files);
    // helper should be accessible through lib
    assert!(resolution.modules["lib"].exports.contains("helper"));
}
```

## Performance Considerations

### Optimization Strategies
1. **Lazy Parsing**: Parse modules only when needed
2. **Parallel Resolution**: Process independent modules concurrently
3. **Incremental Updates**: Re-resolve only affected modules
4. **Symbol Caching**: Cache resolved symbols for repeated lookups
5. **Graph Algorithms**: Use efficient algorithms (Tarjan's for SCC)

### Benchmarks
```rust
#[bench]
fn bench_large_project() {
    // 1000 modules with complex dependencies
    let modules = generate_large_project(1000);
    b.iter(|| resolve_modules(modules));
}

#[bench]
fn bench_deep_cycles() {
    // Deep circular dependency chains
    let modules = generate_cycle_chain(100);
    b.iter(|| resolve_modules(modules));
}
```

## Implementation Phases

### Phase 1: Setup & Data Structures (Week 1)
- [ ] Create toy_resolve crate with dependencies (petgraph)
- [ ] Implement ModulePath enum with path resolution
- [ ] Define all core data structures (ModuleImportsExports, Import, Export, etc.)
- [ ] Ensure all types are Clone + PartialEq + Eq for Salsa
- [ ] Write basic tests for path resolution

### Phase 2: Collection Phase (Week 2)
- [ ] Implement AST visitor for collecting imports
- [ ] Implement AST visitor for collecting exports
- [ ] Create collect_module_imports_exports function (pure, threadpool-friendly)
- [ ] Test collection with various module patterns
- [ ] Handle error recovery
- [ ] Document parallelization strategies for callers

### Phase 3: Graph Building (Week 3)
- [ ] Implement DependencyGraph wrapper around petgraph
- [ ] Add build_graph function
- [ ] Implement path resolution for imports
- [ ] Use petgraph's kosaraju_scc for cycle detection
- [ ] Use petgraph's toposort for ordering
- [ ] Test with cyclic and acyclic graphs

### Phase 4: Symbol Resolution (Week 4)
- [ ] Implement SymbolTable structure
- [ ] Create resolve_module_sequential function
- [ ] Implement resolve_scc_fixpoint for cycles
- [ ] Handle re-exports (pub use)
- [ ] Support selective imports and aliases
- [ ] Test resolution with various scenarios

### Phase 5: Integration & Polish (Week 5)
- [ ] Add comprehensive diagnostics
- [ ] Create all-in-one resolve_modules convenience function
- [ ] Write Salsa integration documentation
- [ ] Performance testing and optimization
- [ ] Error recovery improvements

### Phase 6: Testing & Documentation (Week 6)
- [ ] Comprehensive unit tests for each phase
- [ ] Integration tests with realistic module structures
- [ ] Benchmark parallel vs sequential collection
- [ ] Document Salsa integration patterns
- [ ] Write migration guide for compiler integration

## Integration Points

### With Compiler (Salsa-based)
- Compiler provides ModuleId and AST provider function
- Compiler provides module path mapping
- Compiler calls the three phases as separate Salsa queries
- Salsa automatically handles memoization and incremental updates
- When a module changes, only that module's collection is re-run
- Graph and resolution are recomputed from the new collected data

### With toy_ast
- Use AST types directly (UseStatement, FunctionDefinition, etc.)
- Extract import/export information from typed AST nodes
- Preserve TextRange for diagnostics

### With toy_hir
- Pass Resolution to HIR lowering
- HIR uses symbol tables to resolve references
- Include dependency graph for ordering lowering

### Salsa Benefits
- **Automatic Memoization**: Each phase result is cached
- **Incremental Updates**: Changing one module only re-resolves affected parts
- **Parallel Safety**: Salsa handles concurrent queries
- **Deterministic**: All functions are pure with immutable outputs

## Future Enhancements

1. **Module System Features**
   - Conditional imports (cfg)
   - Module visibility levels (pub(crate), etc.)
   - Glob imports (use std.*)
   - External package resolution

2. **Performance**
   - Persistent cache between runs
   - Lazy symbol resolution
   - Parallel module discovery
   - Memory mapping for large projects

3. **Developer Experience**
   - Better error messages with suggestions
   - Import organization/sorting
   - Unused import detection
   - Circular dependency visualization

## Success Criteria

1. **Correctness**
   - All valid imports resolve correctly
   - Cycles are detected and handled gracefully
   - Visibility rules are enforced (pub vs private)
   - Re-exports work correctly

2. **Performance**
   - Collection phase is pure and parallelizable by caller
   - < 100ms for 1000 module project (full resolution with parallelization)
   - Salsa provides automatic incremental updates and parallelism
   - Graph algorithms use efficient petgraph implementations

3. **Salsa Compatibility**
   - All outputs are immutable and cloneable
   - All functions are deterministic
   - Types implement PartialEq + Eq + Clone
   - No internal mutation during queries

4. **API Design**
   - Three clear phases (collect, graph, resolve)
   - Each phase can be called independently
   - Compiler has full control over module discovery and parallelization
   - Easy integration with Salsa query groups
   - No internal parallelization dependencies

## Key Design Decisions

1. **Three-Phase Architecture**: Allows compiler to control when each phase runs and enables Salsa to memoize each phase independently.

2. **AST Provider Function**: Compiler provides ASTs on-demand rather than toy_resolve owning them, giving compiler full control over parsing and caching.

3. **petgraph Integration**: Use battle-tested graph library instead of reimplementing algorithms. Abstract if needed but start with direct usage.

4. **External Parallelization**: Collection phase is a pure function per module, allowing caller to parallelize with their chosen strategy (Salsa's automatic parallelism, rayon, tokio, etc.). No internal parallelization dependencies keeps the crate simple and flexible.

5. **Sequential Graph/Resolution**: These phases have dependencies and need deterministic ordering, so they run sequentially but are fast due to good algorithms.

6. **Immutable Outputs**: All outputs are immutable and cloneable to work well with Salsa's memoization system.

7. **Cycle Handling**: Use fixpoint iteration within SCCs to handle circular dependencies gracefully.
