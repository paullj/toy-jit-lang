use crate::bytecode::Instruction;
use crate::constant::ConstantPool;

/// A compiled function/chunk
#[derive(Debug, Clone)]
pub struct Chunk {
    pub instructions: Vec<Instruction>,
    pub constants: ConstantPool,
    pub local_count: u32,
    pub register_count: u32,
}

impl Chunk {
    pub fn new() -> Self {
        Self {
            instructions: Vec::new(),
            constants: ConstantPool::new(),
            local_count: 0,
            register_count: 0,
        }
    }

    pub fn emit(&mut self, inst: Instruction) {
        self.instructions.push(inst);
    }

    pub fn len(&self) -> usize {
        self.instructions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.instructions.is_empty()
    }
}

impl Default for Chunk {
    fn default() -> Self {
        Self::new()
    }
}

/// A compiled module
#[derive(Debug, Clone)]
pub struct CompiledModule {
    pub main: Chunk,
}
