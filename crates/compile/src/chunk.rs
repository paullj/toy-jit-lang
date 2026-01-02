use lasso::Rodeo;

use crate::constant::ConstantPool;

/// A compiled function/chunk with compact bytecode
#[derive(Debug, Clone)]
pub struct Chunk {
    /// Compact bytecode (Vec<u8> instead of Vec<Instruction>)
    pub code: Vec<u8>,
    pub constants: ConstantPool,
    pub param_count: u8,
    pub local_count: u8,
    pub register_count: u8,
}

impl Chunk {
    pub fn new() -> Self {
        Self {
            code: Vec::new(),
            constants: ConstantPool::new(),
            param_count: 0,
            local_count: 0,
            register_count: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.code.len()
    }

    pub fn is_empty(&self) -> bool {
        self.code.is_empty()
    }
}

impl Default for Chunk {
    fn default() -> Self {
        Self::new()
    }
}

/// A compiled module containing all functions
#[derive(Debug)]
pub struct CompiledModule {
    pub chunks: Vec<Chunk>,
    pub main_idx: usize,
    /// Interned string table for all string constants
    pub strings: Rodeo,
    /// Struct type metadata for display
    pub struct_metadata: Vec<mir::StructMeta>,
}

impl CompiledModule {
    pub fn main(&self) -> &Chunk {
        &self.chunks[self.main_idx]
    }

    pub fn get_chunk(&self, idx: usize) -> &Chunk {
        &self.chunks[idx]
    }
}
