use std::collections::{HashMap, HashSet};

use crate::scheme::Scheme;
use crate::types::TypeVar;

#[derive(Debug, Clone, Default)]
pub struct TypeEnv {
    bindings: HashMap<String, Scheme>,
}

impl TypeEnv {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, name: String, scheme: Scheme) {
        self.bindings.insert(name, scheme);
    }

    pub fn lookup(&self, name: &str) -> Option<&Scheme> {
        self.bindings.get(name)
    }

    pub fn free_vars(&self) -> HashSet<TypeVar> {
        let mut vars = HashSet::new();
        for scheme in self.bindings.values() {
            vars.extend(scheme.free_vars());
        }
        vars
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &Scheme)> {
        self.bindings.iter()
    }
}
