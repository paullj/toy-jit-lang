use std::collections::HashMap;
use syntax::TextRange;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Variable,
}

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub def_span: TextRange,
    pub name_span: TextRange, // just the identifier token
    pub references: Vec<TextRange>,
    pub doc_comment: Option<String>,
}

impl Symbol {
    pub fn new(
        name: String,
        kind: SymbolKind,
        def_span: TextRange,
        name_span: TextRange,
        doc_comment: Option<String>,
    ) -> Self {
        Self {
            name,
            kind,
            def_span,
            name_span,
            references: Vec::new(),
            doc_comment,
        }
    }

    pub fn add_reference(&mut self, span: TextRange) {
        self.references.push(span);
    }

    /// All locations where this symbol appears (definition + references)
    pub fn all_occurrences(&self) -> impl Iterator<Item = TextRange> + '_ {
        std::iter::once(self.name_span).chain(self.references.iter().copied())
    }
}

#[derive(Debug, Clone)]
pub struct SymbolTable {
    /// Stack of scopes, innermost scope is last
    scopes: Vec<HashMap<String, Symbol>>,
    /// Archived symbols from popped scopes (for LSP queries)
    archived: Vec<Symbol>,
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self {
            scopes: vec![HashMap::new()], // Start with global scope
            archived: Vec::new(),
        }
    }
}

impl SymbolTable {
    pub fn new() -> Self {
        Self::default()
    }

    /// Push a new scope for block expressions
    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    /// Pop the current scope
    pub fn pop_scope(&mut self) {
        // Keep at least the global scope
        if self.scopes.len() > 1
            && let Some(scope) = self.scopes.pop()
        {
            self.archived.extend(scope.into_values());
        }
    }

    pub fn define(
        &mut self,
        name: String,
        kind: SymbolKind,
        def_span: TextRange,
        name_span: TextRange,
        docs: Option<String>,
    ) {
        // Define in the current (innermost) scope
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(
                name.clone(),
                Symbol::new(name, kind, def_span, name_span, docs),
            );
        }
    }

    pub fn add_reference(&mut self, name: &str, span: TextRange) {
        // Search from innermost to outermost scope
        for scope in self.scopes.iter_mut().rev() {
            if let Some(sym) = scope.get_mut(name) {
                sym.add_reference(span);
                return;
            }
        }
    }

    pub fn get(&self, name: &str) -> Option<&Symbol> {
        // Search from innermost to outermost scope
        for scope in self.scopes.iter().rev() {
            if let Some(sym) = scope.get(name) {
                return Some(sym);
            }
        }
        None
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut Symbol> {
        // Search from innermost to outermost scope
        for scope in self.scopes.iter_mut().rev() {
            if let Some(sym) = scope.get_mut(name) {
                return Some(sym);
            }
        }
        None
    }

    /// Find symbol at a given byte offset
    pub fn symbol_at(&self, offset: u32) -> Option<&Symbol> {
        let offset = offset.into();
        // Search active scopes first
        for scope in self.scopes.iter().rev() {
            for symbol in scope.values() {
                if symbol.name_span.contains(offset) {
                    return Some(symbol);
                }
                for &ref_span in &symbol.references {
                    if ref_span.contains(offset) {
                        return Some(symbol);
                    }
                }
            }
        }
        // Then search archived symbols
        for symbol in &self.archived {
            if symbol.name_span.contains(offset) {
                return Some(symbol);
            }
            for &ref_span in &symbol.references {
                if ref_span.contains(offset) {
                    return Some(symbol);
                }
            }
        }
        None
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &Symbol)> {
        self.scopes
            .iter()
            .flat_map(|scope| scope.iter())
            .chain(self.archived.iter().map(|sym| (&sym.name, sym)))
    }
}
