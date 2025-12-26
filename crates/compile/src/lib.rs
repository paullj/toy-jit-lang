mod bytecode;
mod bytecode_writer;
mod chunk;
mod compiler;
mod constant;
mod display;
mod opcode;

pub use bytecode::{ConstIdx, FuncIdx, Label, Slot};
pub use chunk::{Chunk, CompiledModule};
pub use compiler::compile;
pub use constant::{Constant, ConstantPool};
pub use opcode::{NO_SLOT, Opcode};

// Re-export lasso types for VM
pub use lasso::{Rodeo, Spur};
