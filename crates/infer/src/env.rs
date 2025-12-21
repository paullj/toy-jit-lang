use std::collections::{HashMap, HashSet};

use crate::scheme::Scheme;
use crate::types::TypeVar;

#[derive(Debug, Clone)]
pub struct TypeEnv {
    /// Stack of scopes, innermost scope is last
    scopes: Vec<HashMap<String, Scheme>>,
}

impl Default for TypeEnv {
    fn default() -> Self {
        Self {
            scopes: vec![HashMap::new()], // Start with global scope
        }
    }
}

impl TypeEnv {
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
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    pub fn insert(&mut self, name: String, scheme: Scheme) {
        // Insert into the current (innermost) scope
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, scheme);
        }
    }

    pub fn lookup(&self, name: &str) -> Option<&Scheme> {
        // Search from innermost to outermost scope
        for scope in self.scopes.iter().rev() {
            if let Some(scheme) = scope.get(name) {
                return Some(scheme);
            }
        }
        None
    }

    pub fn free_vars(&self) -> HashSet<TypeVar> {
        let mut vars = HashSet::new();
        for scope in &self.scopes {
            for scheme in scope.values() {
                vars.extend(scheme.free_vars());
            }
        }
        vars
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &Scheme)> {
        self.scopes.iter().flat_map(|scope| scope.iter())
    }
}
