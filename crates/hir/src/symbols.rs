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
}

impl Symbol {
    pub fn new(name: String, kind: SymbolKind, def_span: TextRange, name_span: TextRange) -> Self {
        Self {
            name,
            kind,
            def_span,
            name_span,
            references: Vec::new(),
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

#[derive(Debug, Clone, Default)]
pub struct SymbolTable {
    symbols: HashMap<String, Symbol>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn define(
        &mut self,
        name: String,
        kind: SymbolKind,
        def_span: TextRange,
        name_span: TextRange,
    ) {
        self.symbols
            .insert(name.clone(), Symbol::new(name, kind, def_span, name_span));
    }

    pub fn add_reference(&mut self, name: &str, span: TextRange) {
        if let Some(sym) = self.symbols.get_mut(name) {
            sym.add_reference(span);
        }
    }

    pub fn get(&self, name: &str) -> Option<&Symbol> {
        self.symbols.get(name)
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut Symbol> {
        self.symbols.get_mut(name)
    }

    /// Find symbol at a given byte offset
    pub fn symbol_at(&self, offset: u32) -> Option<&Symbol> {
        let offset = offset.into();
        for symbol in self.symbols.values() {
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
        self.symbols.iter()
    }
}
