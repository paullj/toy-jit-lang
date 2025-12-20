use std::collections::HashMap;
use std::fmt;

use crate::bytecode::ConstIdx;

/// A constant value in the constant pool
#[derive(Debug, Clone, PartialEq)]
pub enum Constant {
    Float(f64),
    String(String),
}

impl fmt::Display for Constant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Constant::Float(n) => write!(f, "{}", n),
            Constant::String(s) => write!(f, "\"{}\"", s.escape_default()),
        }
    }
}

/// Pool of constants with deduplication
#[derive(Debug, Clone, Default)]
pub struct ConstantPool {
    constants: Vec<Constant>,
    float_map: HashMap<u64, ConstIdx>,
    string_map: HashMap<String, ConstIdx>,
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

    pub fn add_string(&mut self, s: String) -> ConstIdx {
        if let Some(&idx) = self.string_map.get(&s) {
            return idx;
        }
        let idx = ConstIdx(self.constants.len() as u32);
        self.string_map.insert(s.clone(), idx);
        self.constants.push(Constant::String(s));
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
