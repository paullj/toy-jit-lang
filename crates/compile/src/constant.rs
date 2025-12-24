use std::collections::HashMap;
use std::fmt;

use lasso::{Rodeo, Spur};

use crate::bytecode::ConstIdx;

/// A constant value in the constant pool
#[derive(Debug, Clone, PartialEq)]
pub enum Constant {
    Float(f64),
    StringRef(Spur),
}

impl Constant {
    /// Display constant, requires interner for string lookup
    pub fn display(&self, interner: &Rodeo) -> String {
        match self {
            Constant::Float(n) => format!("{}", n),
            Constant::StringRef(spur) => {
                format!("\"{}\"", interner.resolve(spur).escape_default())
            }
        }
    }
}

impl fmt::Display for Constant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Constant::Float(n) => write!(f, "{}", n),
            Constant::StringRef(spur) => write!(f, "StringRef({:?})", spur),
        }
    }
}

/// Pool of constants with deduplication
#[derive(Debug, Clone, Default)]
pub struct ConstantPool {
    constants: Vec<Constant>,
    float_map: HashMap<u64, ConstIdx>,
}

impl ConstantPool {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_float(&mut self, f: f64) -> ConstIdx {
        let bits = f.to_bits();
        if let Some(&idx) = self.float_map.get(&bits) {
            return idx;
        }
        let idx = ConstIdx(self.constants.len() as u32);
        self.constants.push(Constant::Float(f));
        self.float_map.insert(bits, idx);
        idx
    }

    pub fn add_string(&mut self, s: &str, interner: &mut Rodeo) -> ConstIdx {
        let spur = interner.get_or_intern(s);
        // Note: We still create a new ConstIdx even for duplicate strings
        // because different LoadConst instructions may reference the same string.
        // The actual string storage is deduplicated in the Rodeo.
        let idx = ConstIdx(self.constants.len() as u32);
        self.constants.push(Constant::StringRef(spur));
        idx
    }

    pub fn get(&self, idx: ConstIdx) -> Option<&Constant> {
        self.constants.get(idx.0 as usize)
    }

    pub fn len(&self) -> usize {
        self.constants.len()
    }

    pub fn is_empty(&self) -> bool {
        self.constants.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (ConstIdx, &Constant)> {
        self.constants
            .iter()
            .enumerate()
            .map(|(i, c)| (ConstIdx(i as u32), c))
    }
}
