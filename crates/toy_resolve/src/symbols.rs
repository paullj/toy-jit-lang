//! Symbol table and import/export types

use crate::ModuleId;
use crate::path::ModulePath;
use std::collections::HashMap;
use text_size::TextRange;

/// Output from collection phase for a single module
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolKind {
    Function,
    Variable,
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

impl SymbolTable {
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            exports: HashMap::new(),
            all_symbols: HashMap::new(),
        }
    }

    /// Add a locally defined symbol
    pub fn add_local_symbol(&mut self, export: &Export) {
        let symbol = ResolvedSymbol {
            name: export.name.clone(),
            kind: export.kind,
            is_pub: export.is_pub,
            span: export.span,
            source: SymbolSource::Local,
        };

        self.all_symbols.insert(export.name.clone(), symbol.clone());
        if export.is_pub {
            self.exports.insert(export.name.clone(), symbol);
        }
    }

    /// Add a re-exported symbol
    pub fn add_reexport(
        &mut self,
        local_name: String,
        symbol: &ResolvedSymbol,
        from_module: ModuleId,
    ) -> bool {
        if self.exports.contains_key(&local_name) {
            return false; // Already exists, no change
        }

        let reexport = ResolvedSymbol {
            name: local_name.clone(),
            kind: symbol.kind,
            is_pub: true,
            span: symbol.span,
            source: SymbolSource::ReExport {
                from_module,
                original_name: symbol.name.clone(),
            },
        };

        self.exports.insert(local_name.clone(), reexport.clone());
        self.all_symbols.insert(local_name, reexport);
        true // Changed
    }
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
    ReExport {
        from_module: ModuleId,
        original_name: String,
    },
}
